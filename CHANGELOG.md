# Changelog

本檔記錄 `fcb-rs` 的重要變更，格式參考 [Keep a Changelog](https://keepachangelog.com/zh-TW/1.1.0/)，版本遵循 [Semantic Versioning](https://semver.org/lang/zh-TW/)。技術名詞保留英文。

## [Unreleased]

從 `browser-arena` 數位鑑識教學平台抽離為獨立 repo 後的第一批 codec 補完與使用者／OSS 文件。

### Security

- **🔒 [BREAKING] 明文 header 改由 AEAD AAD 認證。** 整段明文 container 前綴（magic、KIND、`container_version`、`hdr_len`、完整 header CBOR）綁進 XChaCha20-Poly1305 的 additional authenticated data；`seal`／`open` 改收 `aad: &[u8]`。竄改 header 任一欄位（含 task prompt、manifest、`case_id`、`bundle_hash`）開封都會失敗為 `Corrupt`（magic→`BadMagic`、未知 KIND→`Malformed` 仍先做結構性檢查）。**沒有任何 legacy（無 AAD）bundle 仍可讀**。
- **🔒 `.case` 開封驗證內容定址。** `open_case`（fcb-wasm）對解密後的 canonical payload **重算 `bundle_hash` 並和 header 值比對**，不符即 `Corrupt`。此檢查**僅限 `.case`**：`.casework` 的 header `bundle_hash` 是綁回其 case 的參照、非 submission payload 的雜湊，故 `open_submission` 不重算。
- **🔒 [BREAKING for JS authors] pack 邊界數值確定性契約。** `packCase`／`packSubmission` 對記錄裡**絕對值超過 `2^53 - 1`（= 9007199254740991）的整數**——若以普通 JS `number` 提供——會以 `malformed` **拒絕**（serde 會把它編成 CBOR float、和原生整數作者的雜湊發散）；請改用 `BigInt`（無損編成 CBOR 整數，最高到 `u64::MAX`）。safe-range 整數與真正的小數 float 照常接受、且確定。case stream 記錄與 submission 的 notes/report/activity 都套用。

### Added

- **`fcb::case` 模組**：公開 `.case` 產出介面——`CasePayload { streams }` 信封型別、`to_canonical_bytes()` 單一序列化入口、`case_bundle_hash()`（凍結 canonical `bundle_hash = sha256(明文 payload bytes)`）、`pack_case(&CaseInput, passphrase)` 產出 helper（對齊 `pack_submission`，並拒絕空 manifest）。
- **`fcb.netflow.v1` 與 `fcb.json.v1` 記錄 schema**：定義並以 round-trip 測試凍結（5-tuple + bytes/packets + 時間區間；任意 CBOR map 通用容器）。
- **`Submission` byte-stable golden vector**：`FROZEN_SUBMISSION_HEX` + `submission_vector_is_byte_stable` 釘住真實 7 欄 `Submission` 的 on-disk 位元組；另加 `FROZEN_CASE_PAYLOAD_HEX` 釘住 canonical 明文 case payload。
- **標準 OSS 文件**：root `README.md`、`LICENSE`、`CONTRIBUTING.md`、`CODE_OF_CONDUCT.md`（Contributor Covenant v2.1）、`SECURITY.md`（GitHub private advisory 回報管道）。
- **使用者文件**：`docs/fcb-integration-guide.md`（Rust + WASM/JS 消費端整合指南，含安全須知）、`docs/fcb-cookbook.md`（常見任務 recipes）。
- **rustdoc**：`fcb` crate 新增 crate-level 可執行 doctest（`pack_case` → `open_bytes` round-trip）。

### Changed

- **🔒 [BREAKING] `READER_VERSION` 由 `1` 升為 `2`，packed bundle 寫 `min_reader = 2`。** 因明文 header 改綁進 AEAD AAD，pre-AAD（v1）reader 沒有 AAD 步驟、開不了新 bundle，故新 bundle 宣告 `min_reader = 2`、讓舊 reader 以 `UnsupportedVersion` **優雅拒絕**。`container_version` 維持 `1`（佈局不變）、`header_schema_ver` 維持 `1`。
- **授權**：由 `MIT OR Apache-2.0` 改為 **`ECL-2.0`**（Educational Community License v2.0），更貼合本專案教育情境；`Cargo.toml` 補 `repository` 與 `readme` metadata。
- **WASM bridge**：改用 `fcb` crate 公開的 `CasePayload`，消除生產／消費／測試三份重複的信封定義。
- **協定 docs**：`docs/fcb-*.md` 的「已知缺口（Known Gaps）」移除本批已關閉項目（pack_case、canonical bundle_hash、netflow/json schema、Submission 向量），並補上對應正式說明。
- **rustdoc**：修掉 `fcb-wasm` 模組註解對 `wasm_api`（`cfg(wasm32)`-gated）的 broken intra-doc link。

### Notes

- **`Submission` 的 byte-exactness 自本批起才由 golden vector（`FROZEN_SUBMISSION_HEX`）驗證**；先前僅 `FROZEN_WORK_HEX`（test-local 3 欄 `WorkPayload`）與隨機 salt 的 round-trip 覆蓋。非 Rust 重實作者請以本版起的向量為相容性基準。
- **golden vectors 因 AAD 變更已重新產生，但 canonical payload bytes 與 `bundle_hash` 維持不變。** 線上格式只有兩處變：(a) `min_reader` byte 由 `01` 翻為 `02`；(b) 結尾 AEAD tag bytes 不同。所有長度皆不變——`.case` 仍 578 bytes（11 prefix + 476 header + 91 payload）、`.casework`／submission 長度不變、`hdr_len` 仍 476／293。non-Rust 重實作者請以本版起的向量為相容性基準。

---

[Unreleased]: https://github.com/triage-lab/fcb-rs/commits/main
