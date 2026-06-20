# FCB 線上格式（wire format）

`.case` 與 `.casework` 共用同一個 container 信封。本文件描述 byte-level 佈局與密碼學管線，
權威來源為 `crates/fcb/src/{container,crypto,compress,bundle,cbor}.rs`，並以
`crates/fcb/tests/vectors.rs` 的 golden vectors 為驗證基準。

> **整數一律 little-endian。** 字串為 UTF-8。

---

## 1. Container 佈局

```text
magic(4) | KIND(u8) | container_version(u16 LE) | hdr_len(u32 LE)
         | header (hdr_len 個位元組，明文 CBOR)
         | payload (其餘全部；= AEAD(zstd(plaintext_payload)))
```

來源：`container.rs` 的 `write_container` / `read_container` / `peek_header`。

| 欄位 | 大小 | 值 | 說明 |
|------|------|-----|------|
| `magic` | 4 B | `0x89 0x46 0x43 0x42`（`\x89FCB`） | 首 byte `0x89` 仿 PNG，偵測 7/8-bit 傳輸損壞、避免文字碰撞。**版本不烤進 magic**。 |
| `KIND` | 1 B | `1`=`.case`、`2`=`.casework` | `BundleKind`。其他值 → `Malformed`。 |
| `container_version` | 2 B | 目前 `1`（`CONTAINER_VERSION`） | 保留給未來 parse-path 分派。 |
| `hdr_len` | 4 B | header CBOR 的位元組長度 | |
| `header` | `hdr_len` B | 明文 CBOR（見 §2） | KDF salt/params 與 AEAD nonce 必須在「還沒有 key」時就讀得到，故 header 刻意明文（這些不是秘密）。 |
| `payload` | 其餘 | `AEAD(zstd(plaintext))`（見 §3） | |

`peek_header` 可在**不提供 passphrase** 的情況下只讀 header（驗 magic / KIND / `min_reader`），
用來在解鎖前顯示 case 資訊。`min_reader > READER_VERSION`（目前 `1`）→ `UnsupportedVersion`。

### Golden vector 前綴拆解（`FROZEN_CASE_HEX`）

```text
89 46 43 42   magic
01            KIND = 1 (.case)
01 00         container_version = 1   (u16 LE)
dc 01 00 00   hdr_len = 0x000001dc = 476   (u32 LE)
a8 ...        header CBOR：map(8 entries) ...
```

`.casework`（`FROZEN_WORK_HEX`）前綴為 `89 46 43 42 | 02 | 01 00 | 25 01 00 00`（KIND=2、
hdr_len=0x125=293）。

---

## 2. Header（明文 CBOR）

`header` 是 `ciborium::into_writer` 對 `Header` struct 的序列化結果。ciborium 把 struct 編成
**以欄位名為 text key 的 CBOR map**，順序即宣告順序：

```text
Header = {
  "header_schema_ver": u16,     // 目前 1
  "min_reader":        u16,     // 目前 1
  "case_id":           text,    // 教師指派、穩定的題目識別碼
  "bundle_hash":       text,    // 綁定證物版本，格式 "sha256:<hex>"（見 §5 與 data-model）
  "kdf":  { "algo": text, "salt": [u8…], "m_cost": u32, "t_cost": u32, "p_cost": u32 },
  "aead": { "algo": text, "nonce": [u8…] },
  "key_check": [u8…],           // 32 bytes，見 §4
  "meta": <任意 CBOR>           // .case = {streams, task}；.casework = {}（空 map）。見 data-model
}
```

- `kdf.algo` = `"argon2id"`、`kdf.salt` = 16 bytes 隨機、預設 `m_cost=19456` / `t_cost=2` /
  `p_cost=1`（`bundle.rs` 的 `DEFAULT_*`）。
- `aead.algo` = `"xchacha20poly1305"`、`aead.nonce` = 24 bytes 隨機。

### ⚠️ 互通陷阱：`Vec<u8>` 編成 CBOR **array of uint**，不是 byte string

ciborium 對 serde 的 `Vec<u8>`（`salt` / `nonce` / `key_check`）走 `serialize_seq`，產生的是
**CBOR array（major type 4），每個 byte 是一個 unsigned integer**，**不是** CBOR byte string
（major type 2）。例如 16-byte salt 起頭是 `0x90`（array(16)）後接 16 個 uint。

非 Rust 的 encoder 若把這些欄位寫成 byte string，產出的 header 會與參考實作不相容
（ciborium 反序列化成 `Vec<u8>` 時預期 array）。**務必寫成 array of uint。**

其他 ciborium 慣例：
- enum + `#[serde(rename_all = "lowercase")]`（如 `ReportMode`）→ 編成小寫 text（`"steps"` /
  `"freeform"`）。
- `Option<T>` + `skip_serializing_if = "Option::is_none"`（如 task meta 的 `task`）→ `None` 時整個
  key 省略。
- struct field rename（如 `StreamManifest.stream_type` 標 `#[serde(rename = "type")]`）→ CBOR key
  為 `"type"`。

---

## 3. Payload 管線：compress-then-encrypt

來源：`compress.rs`（`pack_payload` / `unpack_payload`）。順序重要——密文近似隨機、壓不動，所以
**先 zstd 壓縮、再 AEAD 加密**：

```text
pack:    plaintext --zstd--> zstd_frame --XChaCha20-Poly1305--> payload
open:    payload --decrypt--> zstd_frame --decompress--> plaintext
```

- **壓縮**：標準 zstd frame（magic `0x28 0xB5 0x2F 0xFD`）。後端是**純 Rust `ruzstd` 0.8**
  （無 C/FFI，native 與 `wasm32-unknown-unknown` 同一份程式碼）。`ruzstd` 0.8 編碼**目前只實作
  `Fastest`**（與 `Uncompressed`）；更高等級在 `ruzstd` 0.8 尚未實作（不要使用）。Fastest 對重複性高的 log 已有好壓縮比，
  且 frame 格式與更高等級相容、未來可無縫升級。產出標準 zstd frame，與 C zstd reader 互通。
- **加密**：XChaCha20-Poly1305（`chacha20poly1305` 0.10）。key = §4 推導的 32 bytes，nonce =
  `header.aead.nonce`（24 bytes）。**沒有 AAD**（`crypto.rs` 的 `seal` 只傳 nonce + plaintext）。

> **安全特性（務必知道）：** AEAD 只認證 **payload**。明文 header（含 `case_id`、`bundle_hash`、
> `meta` 裡的 manifest/task）**未被 AEAD 認證**——竄改 header 不會被 AEAD 偵測。`key_check`
> 只認證「key 正確」、payload AEAD 只認證「payload 未被動」。證物版本綁定靠 `bundle_hash`
> （見 §5、data-model），但**codec 不會驗證 `bundle_hash` 是否真的等於 payload 的 hash**，那是
> 生產端的責任。

---

## 4. 密碼學

來源：`crypto.rs`。

### KDF：Argon2id

```text
key(32 B) = Argon2id(
    password = passphrase 的 UTF-8 bytes,
    salt     = header.kdf.salt,
    m_cost   = header.kdf.m_cost,   // KiB
    t_cost   = header.kdf.t_cost,   // 迭代
    p_cost   = header.kdf.p_cost,   // 平行度
    version  = 0x13 (Argon2 v1.3),
    out_len  = 32,
)
```

`derive_key` 只接受 `kdf.algo == "argon2id"`，其餘回 `Malformed`。注意：**`aead.algo` 不會被驗證**
（`seal`/`open` 完全忽略它），它只是描述性欄位；不要假設它和 `kdf.algo` 一樣有檢查。

### Key-Check Value（KCV）

```text
key_check = SHA256( "FCB-key-check-v1" || key )     // 32 bytes
```

（`KCV_DOMAIN = b"FCB-key-check-v1"`，`key_check_value`。）開檔時：

- KCV 不符 → `WrongPassphrase`（密碼錯）。
- KCV 相符但 AEAD 驗證失敗 → `Corrupt`（檔案被竄改／毀損）。

比較用 constant-time（`ct_eq`）。發佈 key 的 hash 不會實質幫助攻擊者，因為 Argon2id 仍是暴力門檻。

### AEAD：XChaCha20-Poly1305

key = 上面 32 bytes、nonce = 24 bytes（`header.aead.nonce`）、無 AAD。tag 由 AEAD 附在密文尾端
（`chacha20poly1305` 的標準輸出）。

---

## 5. `bundle_hash` 與 binding（摘要）

`compute_bundle_hash(bytes) -> "sha256:<hex>"`（`binding.rs`，小寫 hex）。**注意：codec 不規定
`bytes` 是什麼**——golden vector 用假值 `"sha256:deadbeef"`。生產端必須自訂並固定其涵蓋範圍。

**建議慣例：`bundle_hash = compute_bundle_hash(.case 的明文 payload bytes)`**，亦即壓縮／加密**之前**
的證物序列化位元組（見 data-model 的 `.case` payload 信封）。如此同一份證物無論 salt/nonce 為何都
得到相同 hash，學生作品（`.casework`）才能可靠綁回特定證物版本。binding 規則見
[`fcb-data-model.md`](./fcb-data-model.md)。

---

## 6. 錯誤語意（`error.rs`）

| 變體 | 意義 |
|------|------|
| `BadMagic` | 前 4 bytes 不是 FCB magic。 |
| `UnsupportedVersion { min_reader, supported }` | bundle 要求的 reader 版本比本 reader 新。 |
| `Malformed(String)` | 結構性錯誤（長度前綴壞、CBOR 壞、未知 KIND、未知 KDF、nonce 長度錯…）。 |
| `WrongPassphrase` | KCV 不符（密碼錯）。 |
| `Corrupt` | KCV 相符但 AEAD 失敗，或 zstd frame 壞。 |

---

## 7. 相依套件版本（`crates/fcb/Cargo.toml`）

| 套件 | 版本 | 用途 |
|------|------|------|
| `ciborium` | 0.2 | CBOR 編解碼（header、meta、payload）。 |
| `argon2` | 0.5 | Argon2id KDF（`Algorithm::Argon2id`, `Version::V0x13`）。 |
| `chacha20poly1305` | 0.10 | XChaCha20-Poly1305 AEAD。 |
| `ruzstd` | 0.8 | 純 Rust zstd（編碼僅 `Fastest`）。 |
| `sha2` | 0.10 | SHA-256（KCV、bundle_hash）。 |
| `getrandom` | 0.2（wasm 加 `js` feature） | salt / nonce 隨機來源。 |

非 Rust 實作只要產出的位元組能通過 `cargo test -p fcb`（特別是 `case_vector_is_byte_stable` /
`frozen_case_vector_decodes_to_expected_structure`），即視為相容。

---

## 8. 打包一個 `.case` 的完整順序（end-to-end）

把前面各節串成一條可照做的流程。資料結構細節見 [`fcb-data-model.md`](./fcb-data-model.md)。

1. **準備證物**：每條 stream 整成 `StreamData { id, records }`；`records` 的每筆形狀依該 stream 的
   `type`（如 `fcb.syslog.v1`，見 data-model §3.1）。
2. **組 manifest**：每條 stream 一筆 `StreamManifest { id, type, records = len(records) }`。
3. **組 task spec**：`TaskSpec`（**零答案**，見 data-model §6）。
4. **meta**（明文）= CBOR `{ "streams": [manifest…], "task": TaskSpec }`。
5. **payload_plain** = CBOR `{ "streams": [StreamData…] }`。
6. **bundle_hash**：建議 = `compute_bundle_hash(payload_plain)`（= `"sha256:" + lower_hex(SHA256(payload_plain))`）。
   ⚠️ 正規涵蓋範圍尚未凍結（見 §5 與 data-model §7），確定後再定案。
7. **隨機**產生 `salt`(16 B) 與 `nonce`(24 B)。
8. **key** = Argon2id(passphrase UTF-8, salt, m/t/p, version 0x13, out 32 B)。
9. **key_check** = SHA256(`"FCB-key-check-v1"` ‖ key)（32 B）。
10. **compressed** = zstd `Fastest`(payload_plain)。
11. **ciphertext** = XChaCha20-Poly1305 seal(key, nonce, compressed)（**無 AAD**）。
12. **header**（struct）= `{ header_schema_ver: 1, min_reader: 1, case_id, bundle_hash,
    kdf: {algo:"argon2id", salt, m_cost, t_cost, p_cost}, aead: {algo:"xchacha20poly1305", nonce},
    key_check, meta }`。
13. **hdr** = CBOR(header)（記得 §2 的 `Vec<u8>`→array、欄位順序、key 名等規則）。
14. **輸出位元組** = `magic(89 46 43 42) ‖ KIND(0x01) ‖ container_version(01 00) ‖
    len(hdr) as u32 LE ‖ hdr ‖ ciphertext`。

> `.casework` 相同，差別只在：`KIND = 0x02`、`meta = {}`（空 map）、`payload_plain = CBOR(Submission)`
> （見 data-model §4）。

順序要點：`salt`/`nonce` 在推 key 前就要產生；`key_check` 在加密前算好；`header` 在 `salt`/`nonce`/
`key_check`/`bundle_hash`/`meta` 都備齊後才能序列化求 `hdr_len`（`header` 不含 `ciphertext`）。
