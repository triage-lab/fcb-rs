## Why

`fcb-rs` 剛從 browser-arena 抽出成獨立 repo，但缺少標準 OSS 專案文件：repo **根目錄沒有 `README.md`**（只有 `docs/README.md`），也**沒有 `LICENSE` 檔**（`Cargo.toml` 原宣告 `license = "MIT OR Apache-2.0"`，`wasm-pack` 因此每次 build 都警告缺 LICENSE 檔），更沒有 `CONTRIBUTING.md`、`CODE_OF_CONDUCT.md`、`SECURITY.md`。對一個密碼學 codec 而言，缺 `SECURITY.md`（漏洞回報管道）尤其需要補。本 change 補齊讓 repo 成為可發佈、可貢獻的 OSS 專案，並把授權改為適合教育情境的 **ECL-2.0**。

## What Changes

- 新增根目錄 **`README.md`**：說明 FCB / fcb-rs 是什麼、repo 結構（`crates/fcb`、`crates/fcb-wasm`、`docs/`、`openspec/`）、Rust 與 WASM 兩條消費 quickstart、build/test 指令、CI 狀態（GitHub Actions badge）、授權與文件入口；交叉連結既有 `docs/fcb-*.md`，不複製其內容。
- 新增根目錄 **`LICENSE`**（Educational Community License v2.0／**ECL-2.0** 標準全文，取自 SPDX canonical 正本），並把 `Cargo.toml` 改為 `license = "ECL-2.0"`，消除 `wasm-pack` 缺 LICENSE 警告。選 ECL-2.0 是因本專案屬教育情境，較雙 MIT/Apache 更貼切。
- 新增 **`CONTRIBUTING.md`**：開發流程（Spectra SDD：propose → apply → archive）、品質關卡（`cargo fmt`/`clippy -D warnings`/`cargo test --workspace`/`wasm-pack build`）、不可破壞 golden vectors 的鐵則、commit/PR 慣例。
- 新增 **`CODE_OF_CONDUCT.md`**：採 Contributor Covenant v2.1（標準文本）。
- 新增 **`SECURITY.md`**：crypto 專案的漏洞回報管道（GitHub private security advisory）、支援版本、揭露時程。
- 在兩個 crate 的 `Cargo.toml` 補 `repository` 與 `readme` metadata（消除 `wasm-pack` 缺 `repository` 警告、讓 crates.io/docs.rs metadata 完整）。

## Non-Goals (optional)

- **不改 codec / 測試行為**：除 `Cargo.toml` 的 metadata 欄位外不動 Rust；不影響任何 golden vector。
- **不重寫既有 `docs/fcb-*.md`**：root README 只做入口與交叉連結，避免 fork-and-drift（協定細節仍以 docs/ 為權威）。
- **不建立 crates.io 發佈流程**（屬後續 release 工作）。
- **不新增 CI job**（沿用既有 `.github/workflows/ci.yml`，只在 README 顯示其狀態）。

## Capabilities

### New Capabilities

- `oss-project-docs`: 本 repo SHALL 具備的標準 OSS 專案文件契約——root README、ECL-2.0 `LICENSE` 檔、CONTRIBUTING、CODE_OF_CONDUCT、SECURITY 各自 SHALL 涵蓋的主題。

### Modified Capabilities

(none)

## Impact

- Affected specs: 新增 `oss-project-docs`（文件契約）。
- Affected code:
  - New: README.md, LICENSE, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md
  - Modified: crates/fcb/Cargo.toml, crates/fcb-wasm/Cargo.toml
  - Removed: (none)
