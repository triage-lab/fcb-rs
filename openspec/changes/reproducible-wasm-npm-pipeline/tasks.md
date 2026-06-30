## 1. CI 以 pinned wasm-pack 產出可重現的 wasm 產物

- [x] 1.1 （Requirement: CI enforces the documented quality gate）在 .github/workflows/ci.yml 以 pinned 方式安裝 wasm-pack 0.14.0（如 `cargo install wasm-pack --version 0.14.0 --locked`），並執行 `wasm-pack build crates/fcb-wasm --release --target web` 與 `wasm-pack build crates/fcb-wasm --release --target nodejs`，取代既有的 `cargo build -p fcb-wasm --target wasm32-unknown-unknown` smoke。行為：每次 push/PR 都以固定 wasm-pack 版本、release 模式產出 web 與 nodejs 兩 target 的 fcb-wasm 產物。驗證：CI run log 兩個 wasm-pack build 步驟皆綠；本機以同指令重跑可重現產出對應 out-dir。

## 2. CI 執行 fcb-wasm 的 wasm-bindgen 測試

- [x] 2.1 （Requirement: CI enforces the documented quality gate）在 .github/workflows/ci.yml 加入 `wasm-pack test --node crates/fcb-wasm`，執行 fcb-wasm 的 wasm_bindgen_test 套件（safe-integer 邊界、JS↔native bundle_hash 一致、oversized hdr_len 不 panic）。行為：JS 邊界的行為保證每次 CI 都被執行，任一失敗即擋 merge。驗證：CI run log 顯示 wasm-pack test 步驟執行並通過；本機 `wasm-pack test --node crates/fcb-wasm` 綠。

## 3. CI 加入 MSRV gate

- [x] 3.1 （Requirement: CI enforces the documented quality gate）在 .github/workflows/ci.yml 加入獨立 job/step：以 `dtolnay/rust-toolchain@1.87` 安裝 pinned toolchain 並執行 `cargo build --workspace`，使宣告的 MSRV 長期被驗證、不漂移。注意：本任務起草時對齊 prep-crates-io-publish 宣告的 `rust-version = "1.74"`，但程式碼已將各 crate 的 `rust-version` 提升至 1.87（locked dependency ruzstd 0.8.3 所致，見 crates/fcb/Cargo.toml 註解）；spec 要求 pin「matching the crates' declared minimum supported Rust version」，故 gate 改 pin 1.87（pin 1.74 會因 declared 1.87 而 hard-error、永遠紅燈）。行為：workspace 在 1.87 下能 build，否則 CI fail。驗證：CI run log 顯示 1.87 job 綠；本機 `cargo +1.87 build --workspace` 通過。

## 4. 驗證整體 CI gate 完整且不破壞既有關卡

- [x] 4.1 確認既有 fmt、clippy、`cargo test --workspace` 關卡保留，且新增的 wasm build、wasm test、MSRV build 三類 gate 任一失敗都會 fail workflow，YAML 無語法錯誤。驗證：人工審閱 .github/workflows/ci.yml 含上述各類 gate，且 `actionlint .github/workflows/ci.yml` 無 error（或同等 YAML/Actions 語法檢查通過）。
