## Why

抽離後的 repo 已有協定 reference（`docs/fcb-reference.md`）、整合指南與 OSS 文件，但還缺三塊讓「使用者層」完整的東西：(1) **沒有 `CHANGELOG.md`**——從 browser-arena 抽離與本批的 gap/doc 變更沒有對外可讀的歷史；(2) **沒有 cookbook**——常見任務（驗收 submission、重發 case、解碼特定 stream type、處理 wrong-passphrase vs corrupt）散在各文件，缺一頁可照抄的 recipes；(3) **rustdoc 偏薄且有一個 broken intra-doc link**（`fcb-wasm` 的模組註解連到 `cfg(wasm32)`-gated 的 `wasm_api`，native doc build 會壞），且全 crate 0 個 doctest。本 change 補齊這三塊，把使用者層收斂成一致的 reference 層。

## What Changes

- 新增 **`CHANGELOG.md`**（Keep a Changelog 格式）：記錄從 browser-arena 抽離、以及本批 5 個 phase（pack_case/bundle_hash、netflow/json schema、Submission 向量、OSS 文件含 ECL-2.0、整合指南）。
- 新增 **`docs/fcb-cookbook.md`**：常見任務 recipes（如：開封並驗 submission binding、重發 case 後偵測版本不符、解碼某 stream type、用 golden vector 驗相容、區分 wrong-passphrase vs corrupt），交叉連結整合指南與 reference。
- **rustdoc 補強**：修掉 `crates/fcb-wasm/src/lib.rs` 對 `wasm_api` 的 broken intra-doc link；在 `crates/fcb/src/lib.rs` 加一段 crate-level 可執行 doctest（`pack_case` → `open_bytes` round-trip），並補強關鍵公開項目的 doc comment。確保 `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` 零警告。
- **reference 層一致化**：`docs/README.md` 文件清單與 root `README.md` 文件入口納入 cookbook 與 CHANGELOG，確認既有三份協定 docs 與整合指南、cookbook 交叉連結一致、無 dangling。

## Non-Goals (optional)

- **不改 codec 行為 / 不動 golden vectors**：rustdoc 只加註解與 doctest（doctest 透過公開 API，不改格式）；既有 `*_vector_is_byte_stable` 全綠。
- **不重寫協定 reference 內容**：`fcb-reference.md` 仍為權威，cookbook 只做任務導向 recipes 與交叉連結。
- **不發版本號 / 不上 crates.io**：CHANGELOG 以 `Unreleased` 起頭（實際 tag/release 屬後續 release 工作）。

## Capabilities

### New Capabilities

- `user-reference-and-changelog`: 使用者層 reference 的文件契約——SHALL 具備 `CHANGELOG.md`、任務導向 cookbook、交叉一致的 reference 層，且公開 API 的 rustdoc 可零警告 build（含至少一個 doctest）。

### Modified Capabilities

(none)

## Impact

- Affected specs: 新增 `user-reference-and-changelog`。
- Affected code:
  - New: CHANGELOG.md, docs/fcb-cookbook.md
  - Modified: crates/fcb/src/lib.rs, crates/fcb-wasm/src/lib.rs, docs/README.md, README.md
  - Removed: (none)
