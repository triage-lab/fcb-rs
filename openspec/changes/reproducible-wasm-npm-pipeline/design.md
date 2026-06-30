## Context

CI 現況（.github/workflows/ci.yml）：單一 job，stable toolchain 加 wasm32 target，跑 fmt、clippy、`cargo test --workspace`、以及 `cargo build -p fcb-wasm --target wasm32-unknown-unknown`。三個缺口：wasm 只 build 不測、不產出 wasm-pack 形態的可消費產物、無 MSRV 驗證。下游 browser-arena（submodule + `wasm-pack build fcb-rs/crates/fcb-wasm --target nodejs|web`）與 ba-case-builder（scripts/refresh-fcb-wasm.sh 跑 `wasm-pack build crates/fcb-wasm --target <T>`）都倚賴這條 wasm-pack build 路徑，但 CI 沒有護到它。本機 wasm-pack 為 0.14.0。

## Goals / Non-Goals

Goals：CI 以 pinned wasm-pack 產出 web 與 nodejs 兩 target 的可重現產物、執行 fcb-wasm 的 wasm-bindgen_test 套件、並加一道 MSRV gate。
Non-Goals：不發佈 npm、不保留名稱、不打磨 pkg/package.json、不修改任何 .rs、不在此宣告 rust-version 欄位（屬 prep-crates-io-publish）。

## Decisions

- **wasm-pack 版本 pin = 0.14.0**（對齊本機現用版，固定版安裝避免 "latest" 漂移破壞可重現性）。Alternative：浮動 latest → 否決。
- **Target = web 與 nodejs 兩者**：browser-arena 兩者都建、ba-case-builder 用 nodejs；web target 的 async init 路徑與 nodejs 不同，需各自建以保證兩條都可重現。Alternative：只 nodejs → 否決（漏掉 browser-arena 的 web 路徑）。
- **Build 模式 = release**（wasm-pack release 會跑 wasm-opt 最佳化）：產出貼近真實消費、體積最佳化的 wasm。Alternative：dev build → 否決（未最佳化、與下游期望不符）。
- **wasm-bindgen 測試 runner = `wasm-pack test --node`**：node runner 無需 headless 瀏覽器，CI 最簡且穩定，且已覆蓋 fcb-wasm 的 wasm_bindgen_test 邏輯。Alternative：headless chrome（`--headless --chrome`）→ 暫不採（增加 CI 相依與 flakiness）。
- **MSRV gate = 獨立 job**：`dtolnay/rust-toolchain@1.87` 加 `cargo build --workspace`，與主 stable job 分離，語意清楚。Pin 值對齊 crate 實際宣告的 `rust-version`：本 change 起草時各 crate 宣告 1.74，但程式碼已隨 locked dependency（ruzstd 0.8.3）將 MSRV 提升至 1.87（見 crates/fcb/Cargo.toml 註解），故 gate 改 pin 1.87 以「matching the crates' declared minimum supported Rust version」（spec 要求）。若仍 pin 1.74，`cargo build` 會因 declared rust-version=1.87 直接 hard-error，使 gate 永遠紅燈、自相矛盾。Alternative：併進主 job 切 toolchain → 否決（混淆）；pin 過時的 1.74 → 否決（與宣告值不符且無法 build）。
- **既有 `cargo build -p fcb-wasm --target wasm32` 由 wasm-pack build 取代**：wasm-pack 內部即編 wasm32 cdylib，涵蓋面更廣（含 JS glue 與 .d.ts 產出）。

## Implementation Contract

- 修改 .github/workflows/ci.yml：
  - 既有 quality gate（fmt、clippy、`cargo test --workspace`）保留不變。
  - wasm 關卡：以 pinned 方式安裝 wasm-pack 0.14.0（如 `cargo install wasm-pack --version 0.14.0 --locked`），執行 `wasm-pack build crates/fcb-wasm --release --target web` 與 `wasm-pack build crates/fcb-wasm --release --target nodejs`，再執行 `wasm-pack test --node crates/fcb-wasm`。
  - MSRV 關卡：以 `dtolnay/rust-toolchain@1.87` 安裝 toolchain（pin 值對齊 crate 宣告的 `rust-version` = 1.87；起草時的 1.74 已因 ruzstd 0.8.3 提升至 1.87），執行 `cargo build --workspace`。
- 任一 gate 失敗 SHALL fail workflow。
- 不修改任何 .rs；不 commit pkg/（仍 gitignored，CI 產物為暫態）。
