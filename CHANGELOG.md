# Changelog

本檔記錄 `fcb-rs` 的重要變更，格式參考 [Keep a Changelog](https://keepachangelog.com/zh-TW/1.1.0/)，版本遵循 [Semantic Versioning](https://semver.org/lang/zh-TW/)。技術名詞保留英文。

## [Unreleased]

（尚無變更。）

## 📦 [0.1.0] crates.io 發佈 provenance — 2026-07-06

> `fcb` 0.1.0 正式發佈至 crates.io。crate tarball 由 commit `3fdd128` 打包，並以 annotated tag `fcb-v0.1.0` 標記其確切來源（`git checkout fcb-v0.1.0` 可重現 crates.io 上的 tarball）。本節補述發佈 provenance，codec 行為與下方 0.1.0 內容完全一致、未變。

### 📦 發佈致能 (Packaging / Release)

這些是最初 GitHub release（tag `v0.1.0` → commit `d84bbbb`，2026-06-29）到實際 crates.io 發佈（`3fdd128`）之間、讓套件「可被發佈」的變更，**均不改動 codec 行為**：

- **LICENSE 補齊歸屬**：`crates/fcb/LICENSE`、`crates/fcb-wasm/LICENSE` 的 `Copyright [yyyy] [name of copyright owner]` 佔位字填為 `Copyright 2026 The fcb-rs Authors`（這是實際打包出貨的檔；`d84bbbb` 仍帶佔位字，故不可直接發佈）。
- **crates.io metadata**：兩份 `Cargo.toml` 補 `keywords`／`categories`／`authors`／`documentation`，並宣告 `rust-version = "1.87"`（MSRV floor 由 `ruzstd 0.8.3` 決定）；`fcb-wasm` 的 `fcb` path dependency 補上 `version = "0.1.0"` 以利打包。
- **可重現 CI pipeline**：`.github/workflows/ci.yml` 升級為 pinned `wasm-pack 0.14.0` 產出 web + nodejs 產物並跑 wasm-bindgen 測試，另加以 pinned 1.87 toolchain 強制 MSRV 的 deterministic（committed-lock）gate。
- **文件對齊**：協定 docs 與整合指南對齊 0.1.0，README／整合指南的相依說明改用 `fcb = "0.1.0"`。

### 📝 備註 (Provenance)

- **兩個 0.1.0 座標**：crates.io tarball = `3fdd128`（tag `fcb-v0.1.0`）；git tag `v0.1.0` = `d84bbbb`（最初 GitHub release）。
- **codec 行為 100% 等價**：`d84bbbb` 與 `3fdd128` 之間 `crates/fcb/src` 的所有 `.rs`，在移除空白與 trailing comma 後 hash 完全相同——差異純為 `rustfmt` 排版，無任何 token／行為變更。
- **`v0.1.0` tag 刻意保留**：下游 `browser-arena` 的 submodule 釘在 `d84bbbb`，其 `codec-release-pinning` 規格要求所釘 commit 對應到某個 published release tag，故不移動 `v0.1.0`；改以新增 `fcb-v0.1.0` 標記 crate 來源，兩者並存、互不干擾。

## [0.1.0] - 2026-06-29

> `fcb-rs` 首個正式 release：從 `browser-arena` 數位鑑識教學平台抽離為獨立 repo 後的第一批 codec 補完，加上完整的 OSS 與使用者文件。

### 🔒 安全性 (Security)

- **[BREAKING] 明文 header 改由 AEAD AAD 認證。** 整段明文 container 前綴（magic、KIND、`container_version`、`hdr_len`、完整 header CBOR）綁進 XChaCha20-Poly1305 的 additional authenticated data；`seal`／`open` 改收 `aad: &[u8]`。竄改 header 任一欄位（含 task prompt、manifest、`case_id`、`bundle_hash`）開封都會失敗為 `Corrupt`（magic→`BadMagic`、未知 KIND→`Malformed` 仍先做結構性檢查）。沒有任何 legacy（無 AAD）bundle 仍可讀。
- **`.case` 開封驗證內容定址。** `open_case`（fcb-wasm）對解密後的 canonical payload 重算 `bundle_hash` 並和 header 值比對，不符即 `Corrupt`。此檢查僅限 `.case`：`.casework` 的 header `bundle_hash` 是綁回其 case 的參照、非 submission payload 的雜湊，故 `open_submission` 不重算。
- **[BREAKING for JS authors] pack 邊界數值確定性契約。** `packCase`／`packSubmission` 對記錄裡絕對值超過 `2^53 - 1`（= 9007199254740991）的整數——若以普通 JS `number` 提供——會以 `malformed` 拒絕（serde 會把它編成 CBOR float、和原生整數作者的雜湊發散）；請改用 `BigInt`（無損編成 CBOR 整數，最高到 `u64::MAX`）。safe-range 整數與真正的小數 float 照常接受、且確定。case stream 記錄與 submission 的 notes/report/activity 都套用。
- **`pack_case` 強制 manifest 與 payload 一致。** 封裝當下即驗證 manifest 宣告的 stream id 集合與 payload 雙向相等（無缺漏、無多餘、無重複 id），且每條 stream 的 `records` 計數相符；任一不符以 `Malformed` loud reject，把原本只在 decode 端單向才會浮現的不一致提前擋在 producer 端。

### ✨ 新增 (Added)

- **`fcb::case` 模組**：公開 `.case` 產出介面——`CasePayload { streams }` 信封型別、`to_canonical_bytes()` 單一序列化入口、`case_bundle_hash()`（凍結 canonical `bundle_hash = sha256(明文 payload bytes)`）、`pack_case(&CaseInput, passphrase)` 產出 helper（對齊 `pack_submission`，並拒絕空 manifest）。
- **`fcb.netflow.v1` 與 `fcb.json.v1` 記錄 schema**：定義並以 round-trip 測試凍結（5-tuple + bytes/packets + 時間區間；任意 CBOR map 通用容器）。
- **`Submission` byte-stable golden vector**：`FROZEN_SUBMISSION_HEX` + `submission_vector_is_byte_stable` 釘住真實 7 欄 `Submission` 的 on-disk 位元組；另加 `FROZEN_CASE_PAYLOAD_HEX` 釘住 canonical 明文 case payload。
- **fcb-wasm bridge**：完整 `#[wasm_bindgen]` surface——`peekHeader`、`openCase`、`openSubmission`、`packSubmission`、`packCase`、`computeBundleHash`、`verifyBinding`、`workKey`，供瀏覽器 workbench 直接消費。
- **標準 OSS 文件**：root `README.md`、`LICENSE`、`CONTRIBUTING.md`、`CODE_OF_CONDUCT.md`（Contributor Covenant v2.1）、`SECURITY.md`（GitHub private advisory 回報管道）。
- **使用者文件**：`docs/fcb-integration-guide.md`（Rust + WASM/JS 消費端整合指南，含安全須知）、`docs/fcb-cookbook.md`（常見任務 recipes）。
- **rustdoc**：`fcb` crate 新增 crate-level 可執行 doctest（`pack_case` → `open_bytes` round-trip）。
- **CI**：`.github/workflows/ci.yml` 在 push main 與每個 PR 跑 `cargo test --workspace` 與 `wasm32-unknown-unknown` build smoke。

### 🔄 變更 (Changed)

- **[BREAKING] `READER_VERSION` 由 `1` 升為 `2`，packed bundle 寫 `min_reader = 2`。** 因明文 header 改綁進 AEAD AAD，pre-AAD（v1）reader 沒有 AAD 步驟、開不了新 bundle，故新 bundle 宣告 `min_reader = 2`、讓舊 reader 以 `UnsupportedVersion` 優雅拒絕。`container_version` 維持 `1`（佈局不變）、`header_schema_ver` 維持 `1`。
- **授權**：採用 `ECL-2.0`（Educational Community License v2.0），貼合本專案教育情境；`Cargo.toml` 補 `repository` 與 `readme` metadata。
- **WASM bridge**：改用 `fcb` crate 公開的 `CasePayload`，消除生產／消費／測試三份重複的信封定義。
- **協定 docs**：`docs/fcb-*.md` 的「已知缺口（Known Gaps）」移除本批已關閉項目（pack_case、canonical bundle_hash、netflow/json schema、Submission 向量），並補上對應正式說明。
- **rustdoc**：修掉 `fcb-wasm` 模組註解對 `wasm_api`（`cfg(wasm32)`-gated）的 broken intra-doc link。

### 📝 備註 (Notes)

- **`Submission` 的 byte-exactness 自本版起由 golden vector（`FROZEN_SUBMISSION_HEX`）驗證**；先前僅 `FROZEN_WORK_HEX`（test-local 3 欄 `WorkPayload`）與隨機 salt 的 round-trip 覆蓋。非 Rust 重實作者請以本版起的向量為相容性基準。
- **golden vectors 因 AAD 變更已重新產生，但 canonical payload bytes 與 `bundle_hash` 維持不變。** 線上格式變動只有兩處：(a) `min_reader` byte 由 `01` 翻為 `02`；(b) 結尾 AEAD tag bytes 不同。所有長度皆不變——`.case` 仍 578 bytes（11 prefix + 476 header + 91 payload）、`.casework`／submission 長度不變、`hdr_len` 仍 476／293。
- **已知界線（誠實標註）**：reader 端 `decode_streams` 為 manifest-driven，不主動偵測 payload superset（對封過的 `.case`，多出的 bytes 會因 `open_case` 重算 `bundle_hash` 而 `Corrupt`）；`BigInt` 表示上限為 `u64::MAX`。

---

[Unreleased]: https://github.com/triage-lab/fcb-rs/compare/fcb-v0.1.0...HEAD
[fcb-v0.1.0]: https://github.com/triage-lab/fcb-rs/releases/tag/fcb-v0.1.0
[0.1.0]: https://github.com/triage-lab/fcb-rs/releases/tag/v0.1.0
