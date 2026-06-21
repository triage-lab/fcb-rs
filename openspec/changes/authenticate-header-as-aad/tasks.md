## 1. AEAD AAD 綁定（決策一：AAD 綁定整個 container prefix（magic + KIND + container_version + hdr_len + header CBOR））

- [x] 1.1 先寫 failing 單元測試：seal/open 帶 AAD 後正常 round-trip 通過、AAD 不一致時 open 失敗（crates/fcb/src/crypto.rs）。驗證：cargo test -p fcb crypto 由紅轉綠。
- [x] 1.2 讓 seal、open、open_payload 接受 `aad: &[u8]` 並透過 chacha20poly1305 的 `Payload { msg, aad }` 傳入，落地 `Authenticated plaintext header` 與 `Passphrase-based cryptography` 的 AEAD 綁定。驗證：1.1 測試轉綠且既有 crypto 測試不退步。
- [x] 1.3 在 crates/fcb/src/container.rs 提供回傳 prefix bytes（magic + KIND + container_version + hdr_len + header CBOR）的序列化 helper，並讓 read_container 暴露 payload 起始位移，作為 AAD 綁定整個 container prefix 的來源。驗證：新增 helper round-trip 單元測試通過、cargo test -p fcb container 通過。
- [x] 1.4 在 crates/fcb/src/bundle.rs 的 pack_bytes 先組 prefix 當 AAD 再 seal payload、open_bytes 以讀入的 prefix bytes 當 AAD 傳入 open，使任何 header byte 竄改→ Corrupt。驗證：新增「翻動 header 任一 byte → open 回 Corrupt」測試通過（對應 `Passphrase-based cryptography` 的 Tampered header scenario）。

## 2. 版本閘（決策二：以 min_reader 1→2 做版本閘，container_version 不動，不回讀舊 bundle）

- [x] 2.1 把寫入的 min_reader 由 1 改為 2、reader 端支援版本常數提升為 2，達成「舊 reader 對新 bundle graceful refuse、新 reader 接受自身 bundle」。驗證：新增測試——支援版本 1 的 reader 對 min_reader=2 bundle 回 UnsupportedVersion，且正常 open 成功。

## 3. case open 內容定址驗證（決策三：open_case 另外重算 bundle_hash 比對（僅 .case），mismatch → Corrupt）

- [x] 3.1 先寫 failing 測試：open_case 對 header.bundle_hash 與實際 payload hash 不符的 .case 回 Corrupt、相符則成功。驗證：cargo test 由紅轉綠。
- [x] 3.2 在 .case 開啟路徑（fcb-wasm open_case）解出 canonical payload 後 compute_bundle_hash 比對 header.bundle_hash，不符即 Corrupt，落地 `Verified content address on case open`；open_submission 不做此重算。驗證：3.1 測試轉綠且 submission round-trip 不受影響。

## 4. golden vector 重生與竄改測試（決策四：重生 golden vectors 並新增 header 竄改與 round-trip 測試）

- [x] 4.1 以新 codec 重生 vectors/frozen_case.hex 與 vectors/frozen_work.hex 並更新 crates/fcb/tests/vectors.rs 的 frozen 常數，使 byte-stable 測試重新鎖定通過。驗證：cargo test -p fcb --test vectors 全綠。
- [x] 4.2 新增竄改測試矩陣：翻動 KIND / container_version / header 任一 byte → open 回對應錯誤（Corrupt 或 KIND/version 既有錯誤），正常 round-trip 成功。驗證：cargo test -p fcb 新測試通過。

## 5. 驗證關卡

- [x] 5.1 全工作區綠燈：cargo test --workspace 全綠、cargo clippy --all-targets --all-features -- -D warnings 無警、cargo build -p fcb --target wasm32-unknown-unknown 成功。驗證：三道指令皆成功結束。
