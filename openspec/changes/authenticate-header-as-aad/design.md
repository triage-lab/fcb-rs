## Context

FCB container 目前的信任邊界：payload 由 AEAD（XChaCha20-Poly1305）保護機密性與完整性，但 plaintext header 是純明文且**未被認證**。seal/open（crates/fcb/src/crypto.rs）用空 AAD，bundle.rs 只把 payload 餵進 cipher；header 也不在 bundle_hash 內（bundle_hash 只蓋 canonical payload）。因此 header 任一欄位（task prompt、stream manifest、case_id、bundle_hash）可被無感竄改，submission↔case 的 binding 也不是密碼學保證。

container 線上佈局：magic(4) | KIND(u8) | container_version(u16) | hdr_len(u32) | header(CBOR, hdr_len bytes) | payload(AEAD(zstd(canonical bytes)))。header 在 open 前即可讀（解 KDF/nonce 需要），這個「無密碼讀 header」能力必須保留。

## Goals / Non-Goals

**Goals:**

- 讓整個 plaintext header（及其前綴 magic/KIND/version/hdr_len）受 AEAD 認證：任何 header 竄改在 open 時失敗為 Corrupt。
- 讓 .case 的 bundle_hash 成為可驗證的內容定址，而非僅是宣告值。
- 對不支援 AAD 的舊 reader graceful refuse，而非誤讀。

**Non-Goals:**

- 不支援回讀舊（pre-AAD）bundle——v0.1 採乾淨斷裂。
- 不改變線上 byte 佈局與 header schema 欄位（僅新增 AEAD 的 AAD 語意）。
- 不處理 .casework 的 bundle_hash 重算（該欄位是 binding 參照＝案件的 hash，並非提交 payload 的 hash）。
- 不觸碰 fcb-wasm 對 JS 大整數的處理（屬另一個 change）。

## Decisions

### 決策一：AAD 綁定整個 container prefix（magic + KIND + container_version + hdr_len + header CBOR）

AAD 取「ciphertext 之前的全部 bytes」，即整個 prefix。如此一次認證 magic、KIND、container_version、hdr_len 與完整 header（含 case_id / bundle_hash / kdf / nonce / key_check / meta）。綁定 literal 線上 bytes（而非重新序列化的 header）以避免 canonicalization 不一致。

替代方案：只綁 header CBOR（不含 KIND/version）。否決原因：KIND（case↔work）與 version 仍可被翻動而不被密碼學偵測；綁整段 prefix 成本相同卻覆蓋更廣。

可行性：pack 時 header 不依賴 ciphertext，故可先組出 prefix → 以 prefix 為 AAD seal payload → 串接 prefix + ciphertext 輸出。open 時 read_container 已解析出 payload 起始位移，AAD 即為 bytes[0..payload_start]，與 pack 寫入的 prefix 必然逐 byte 相同。

### 決策二：以 min_reader 1→2 做版本閘，container_version 不動，不回讀舊 bundle

byte 佈局與 header schema 欄位都沒變，改變的只有 AEAD 的 AAD 語意，這正是 min_reader 該守的（reader 是否具備 AAD 能力）。故 container_version 與 header_schema_ver 維持 1，僅把寫入的 min_reader 由 1 提升為 2，同時把 reader 端「支援版本」常數提升為 2，讓新 reader 接受自己產出的 min_reader=2 bundle。

效果：舊 reader（支援 1）開新 bundle → min_reader(2) > 支援(1) → UnsupportedVersion，graceful refuse。新 reader 開舊（pre-AAD）bundle → AAD 驗證失敗 → Corrupt（可接受：v0.1 無流通中的真實舊 bundle，golden vector 為測試夾具，一律重生）。

替代方案：同時 bump container_version 並保留雙路徑（AAD / 非 AAD）。否決原因：v0.1 不需向後相容，雙路徑徒增攻擊面與維護成本。

### 決策三：open_case 另外重算 bundle_hash 比對（僅 .case），mismatch → Corrupt

AAD 只保證 header bytes 未被竄改（含 producer 寫入的 bundle_hash），不保證該 bundle_hash 真的等於 payload 的 hash。為讓 .case 真正成為內容定址、binding 端到端可信，open_case 在解出 canonical payload 後重算 compute_bundle_hash(payload) 並與 header.bundle_hash 比對，不符即 Corrupt。

僅限 .case：.casework 的 header.bundle_hash 是 binding 參照（所綁案件的 hash），並非提交 payload 的 hash，故 open_submission 不做此重算。

替代方案：只靠 AAD、不重算。否決原因：留下「producer 寫入與 payload 不符的 bundle_hash」缺口，正是討論中點名「bundle_hash 是廣告值而非已驗證」的核心。重算成本僅一次 SHA-256，廉價。

### 決策四：重生 golden vectors 並新增 header 竄改與 round-trip 測試

AEAD tag 因 AAD 改變，vectors/frozen_case.hex 與 vectors/frozen_work.hex 必須以新 codec 重生，crates/fcb/tests/vectors.rs 的 frozen 常數同步更新。新增測試：翻動 header 任一 byte（manifest / task / case_id）→ open 回 Corrupt；翻動 KIND 或 container_version → open 回 Corrupt（或對應 KIND/version 既有錯誤）；正常 pack→open round-trip 仍通過；篡改 bundle_hash 值（決策三）→ Corrupt。

## Implementation Contract

**Behavior（可觀察行為）：**
- pack 出的新 bundle，其 AEAD 認證範圍涵蓋整個 prefix（magic/KIND/version/hdr_len/header）。
- open 任何 header／prefix 被竄改一個 byte 的 bundle → 回 Corrupt，不吐出任何已解碼資料。
- open_case 對 header.bundle_hash 與實際 payload hash 不符的 .case → 回 Corrupt。
- 舊 reader 對新 bundle → UnsupportedVersion；正常 round-trip 不受影響。

**Interface / data shape：**
- crates/fcb/src/crypto.rs：seal 與 open（以及 open_payload）新增 `aad: &[u8]` 參數，透過 chacha20poly1305 的 `Payload { msg, aad }` 傳入。
- crates/fcb/src/bundle.rs：pack_bytes 先組 prefix bytes 當 AAD 再 seal；open_bytes 由解析後的 container 取得 payload 起始位移，以 prefix bytes 當 AAD 傳入 open。min_reader 寫入值改為 2。
- crates/fcb/src/container.rs：提供一個回傳「prefix（含 hdr_len 與 header CBOR）bytes」的序列化 helper，供 pack 端產生並當 AAD；read_container 暴露 payload 起始位移或 prefix slice 供 open 端重建 AAD；reader 支援版本常數提升為 2。
- .case 的 open 路徑（fcb-wasm open_case，或 fcb 提供之 case 開啟 helper）：解出 payload 後 compute_bundle_hash 比對 header.bundle_hash。

**Failure modes：**
- header／prefix 竄改、bundle_hash 不符、payload 竄改 → 一律 Corrupt（非靜默）。
- 錯誤密碼 → 仍為 WrongPassphrase（KCV 先判，與 AAD 無關）。
- 舊 reader 開新 bundle → UnsupportedVersion。

**Acceptance criteria：**
- cargo test --workspace 全綠（含重生後的 vectors.rs byte-stable 測試與新竄改測試）。
- cargo clippy --all-targets --all-features -- -D warnings 無警。
- cargo build -p fcb --target wasm32-unknown-unknown 過。
- 新增測試明確涵蓋：header byte 翻動→Corrupt、bundle_hash 不符→Corrupt、round-trip 成功、舊 reader graceful refuse。

**Scope boundaries：**
- In scope：crypto/bundle/container 的 AAD 綁定、min_reader 提升、open_case 的 bundle_hash 重算、golden vector 重生、相關測試。
- Out of scope：JS 大整數 / safe-int 契約（另一 change）、使用者文件更新（另一 change）、.casework bundle_hash 重算、manifest.records 驗證。

## Risks / Trade-offs

- [新 reader 開舊 bundle 得到 Corrupt 而非更明確的「legacy 不支援」] → v0.1 無流通舊 bundle，且行為仍是 fail-closed；可接受，必要時未來再加 min_reader 過低的專屬訊息。
- [prefix 當 AAD 需在 seal 前先定出 header 與 hdr_len] → header 不依賴 ciphertext，重排 pack 順序即可，無循環相依。
- [golden vector 重生若手誤會鎖錯 byte] → 以「pack 後立即 open round-trip + 既有 decode 結構測試」雙重把關，且重生流程寫入 tasks。
- [open_case 重算 bundle_hash 增加一次 SHA-256] → payload 已在記憶體，成本可忽略；換得 binding 端到端可信。
