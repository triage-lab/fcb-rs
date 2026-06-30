## Why

CI 目前只用 `cargo build -p fcb-wasm --target wasm32-unknown-unknown` 當 wasm smoke——它證明能編，卻不產出可被下游消費的 wasm 產物、不執行 fcb-wasm 的 wasm-bindgen_test 套件（safe-integer 邊界、JS↔native bundle_hash 一致性、oversized hdr_len 不 panic），也不驗證宣告的 MSRV。下游 browser-arena 與 ba-case-builder 都在原地以 wasm-pack 重建 fcb-wasm 產物，卻沒有任何 CI 保證那條 build 路徑可重現且通過行為測試。本變更把 CI 的 wasm 關卡升級為「可重現、被測試」的 pipeline。

## What Changes

- CI 以 pinned 版本的 wasm-pack，對 fcb-wasm 做 release/最佳化 build，涵蓋 web 與 nodejs 兩個 target，取代現有單一 `cargo build` smoke，使 CI 產出與下游 refresh 流程一致、可重現的產物。
- CI 執行 fcb-wasm 的 wasm-bindgen_test 套件（`wasm-pack test --node`），讓 JS 邊界的行為保證每次 push/PR 都被驗證。
- CI 新增 MSRV 關卡：以 pinned toolchain（1.87，對齊各 crate 宣告的 rust-version；起草時的 1.74 已因 locked dependency ruzstd 0.8.3 提升至 1.87）對 committed Cargo.lock 編譯 workspace，驗證宣告的最低 Rust 版本確實可建（deterministic，僅依 committed lock）。

## Non-Goals

- 不實際發佈 npm 套件、不保留 npm 名稱。
- 不打磨 pkg/package.json 的 npm-publish 欄位（exports、module、keywords 等）——下游以 in-tree wasm-pack 消費，現階段不需要。
- 不修改 fcb-wasm 或 fcb 的 Rust 原始碼，亦不變更 codec 行為。
- 不在此 change 宣告 rust-version 欄位本身（該宣告屬 prep-crates-io-publish）；此處只負責「驗證」那個下限。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `oss-project-docs`: 擴充「CI enforces the documented quality gate」要求——CI 的 wasm 關卡 SHALL 從單純 build 升級為以 pinned wasm-pack 產出 web 與 nodejs 兩 target 的可重現產物、執行 fcb-wasm 的 wasm-bindgen_test 套件，並 SHALL 以 pinned toolchain 強制 workspace 的 MSRV 下限。

## Impact

- Affected specs: oss-project-docs（modified）
- Affected code:
  - Modified: .github/workflows/ci.yml
