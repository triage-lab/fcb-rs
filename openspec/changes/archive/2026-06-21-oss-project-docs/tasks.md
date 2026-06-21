## 1. 授權與安全（標準文本優先）

- [x] 1.1 （Requirement: License file is present）新增根目錄 `LICENSE`（Educational Community License v2.0／ECL-2.0 標準全文，取自 SPDX canonical 正本）。完成定義：`LICENSE` 存在、文字為 ECL-2.0 標準全文、與 manifest 的 `license = "ECL-2.0"` 一致。驗證：`ls LICENSE`、人工核對首段為 "Educational Community License Version 2.0"。
- [x] 1.2 （Requirement: Security policy describes vulnerability reporting）新增 `SECURITY.md`：crypto 專案漏洞回報走 GitHub private security advisory（Security → Report a vulnerability），列支援版本（0.1.x）、揭露時程、不要走 public issue。完成定義：含私密回報管道、支援版本、揭露原則，無「請開 public issue」字樣。驗證：`rg "private|advisory|public issue" SECURITY.md` 反映正確、人工審閱。

## 2. 貢獻與行為準則

- [x] 2.1 （Requirement: Contributing guide states the workflow and invariants）新增 `CONTRIBUTING.md`：說明 Spectra SDD 流程（propose → apply → archive）、品質關卡四道指令（`cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --workspace`、`wasm-pack build crates/fcb-wasm --target nodejs`）、不可破壞既有 `*_vector_is_byte_stable` golden vectors 的鐵則、commit/PR 慣例。完成定義：涵蓋工作流程、品質關卡、golden-vector 鐵則。驗證：人工審閱、`rg "byte_stable|clippy|spectra" CONTRIBUTING.md` 命中。
- [x] 2.2 （Requirement: Code of conduct is published）新增 `CODE_OF_CONDUCT.md`：採 Contributor Covenant v2.1 標準文本，enforcement 聯絡走專案維護管道。完成定義：含行為期望與回報管道。驗證：人工核對為 Contributor Covenant v2.1。

## 3. README 與 metadata

- [x] 3.1 （Requirement: Root README orients new users）新增根目錄 `README.md`：FCB / fcb-rs 是什麼、repo 結構（crates/fcb、crates/fcb-wasm、docs/、openspec/）、Rust 與 WASM 兩條 quickstart（連結 docs/README.md 的可跑範例與 fcb::case::pack_case）、build/test 指令、CI badge（GitHub Actions `ci.yml`）、`ECL-2.0` 授權、交叉連結 docs/fcb-*.md。完成定義：涵蓋上述各節、不複製 docs 內容只入口連結。驗證：人工審閱、連結正確、`rg "crates/fcb-wasm|docs/fcb-|ECL-2.0" README.md` 命中。
- [x] 3.2 在 `crates/fcb/Cargo.toml` 與 `crates/fcb-wasm/Cargo.toml` 把 `license` 改為 `"ECL-2.0"`，並補 `repository = "https://github.com/triage-lab/fcb-rs"` 與 `readme`，消除 `wasm-pack` 缺 `repository` 警告、補齊套件 metadata。完成定義：兩 manifest 為 `license = "ECL-2.0"` 且含 repository/readme、workspace 仍能 build。驗證：`cargo metadata --no-deps -q` 成功、`cargo build -p fcb` 通過。

## 4. 驗證

- [x] 4.1 因本 phase 動到 `Cargo.toml`（影響 build metadata），補跑：`cargo build --workspace`（通過）、`cargo test --workspace`（既有測試全綠、golden vectors 不退）、`wasm-pack build crates/fcb-wasm --target nodejs`（通過、且不再有缺 LICENSE/repository 警告）。完成定義：build/test/wasm 全綠。驗證：三道指令各自 exit 0。
