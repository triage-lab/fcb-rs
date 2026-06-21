# Changelog

本檔記錄 `fcb-rs` 的重要變更，格式參考 [Keep a Changelog](https://keepachangelog.com/zh-TW/1.1.0/)，版本遵循 [Semantic Versioning](https://semver.org/lang/zh-TW/)。技術名詞保留英文。

## [Unreleased]

從 `browser-arena` 數位鑑識教學平台抽離為獨立 repo 後的第一批 codec 補完與使用者／OSS 文件。

### Added

- **`fcb::case` 模組**：公開 `.case` 產出介面——`CasePayload { streams }` 信封型別、`to_canonical_bytes()` 單一序列化入口、`case_bundle_hash()`（凍結 canonical `bundle_hash = sha256(明文 payload bytes)`）、`pack_case(&CaseInput, passphrase)` 產出 helper（對齊 `pack_submission`，並拒絕空 manifest）。
- **`fcb.netflow.v1` 與 `fcb.json.v1` 記錄 schema**：定義並以 round-trip 測試凍結（5-tuple + bytes/packets + 時間區間；任意 CBOR map 通用容器）。
- **`Submission` byte-stable golden vector**：`FROZEN_SUBMISSION_HEX` + `submission_vector_is_byte_stable` 釘住真實 7 欄 `Submission` 的 on-disk 位元組；另加 `FROZEN_CASE_PAYLOAD_HEX` 釘住 canonical 明文 case payload。
- **標準 OSS 文件**：root `README.md`、`LICENSE`、`CONTRIBUTING.md`、`CODE_OF_CONDUCT.md`（Contributor Covenant v2.1）、`SECURITY.md`（GitHub private advisory 回報管道）。
- **使用者文件**：`docs/fcb-integration-guide.md`（Rust + WASM/JS 消費端整合指南，含安全須知）、`docs/fcb-cookbook.md`（常見任務 recipes）。
- **rustdoc**：`fcb` crate 新增 crate-level 可執行 doctest（`pack_case` → `open_bytes` round-trip）。

### Changed

- **授權**：由 `MIT OR Apache-2.0` 改為 **`ECL-2.0`**（Educational Community License v2.0），更貼合本專案教育情境；`Cargo.toml` 補 `repository` 與 `readme` metadata。
- **WASM bridge**：改用 `fcb` crate 公開的 `CasePayload`，消除生產／消費／測試三份重複的信封定義。
- **協定 docs**：`docs/fcb-*.md` 的「已知缺口（Known Gaps）」移除本批已關閉項目（pack_case、canonical bundle_hash、netflow/json schema、Submission 向量），並補上對應正式說明。
- **rustdoc**：修掉 `fcb-wasm` 模組註解對 `wasm_api`（`cfg(wasm32)`-gated）的 broken intra-doc link。

### Notes

- **`Submission` 的 byte-exactness 自本批起才由 golden vector（`FROZEN_SUBMISSION_HEX`）驗證**；先前僅 `FROZEN_WORK_HEX`（test-local 3 欄 `WorkPayload`）與隨機 salt 的 round-trip 覆蓋。非 Rust 重實作者請以本版起的向量為相容性基準。
- 既有 golden vectors（`FROZEN_CASE_HEX`、`FROZEN_WORK_HEX`）在本批維持**逐位元不變**；wire format 未變動。

---

[Unreleased]: https://github.com/triage-lab/fcb-rs/commits/main
