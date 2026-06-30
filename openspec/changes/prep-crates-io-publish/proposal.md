## Why

fcb-rs 目前無法乾淨發佈到 crates.io，且存在一個會實際出貨的缺陷：兩個 publishable crate（fcb、fcb-wasm）各自打包進套件的 LICENSE 第 87 行仍是未填的範本佔位字 "Copyright [yyyy] [name of copyright owner]"，與根 LICENSE（已填 "2026 The fcb-rs Authors"）不一致。同時 fcb-wasm 對 fcb 的 path dependency 未帶 version，使打包 fcb-wasm 直接失敗，而兩個 manifest 都缺 crates.io 可發現性與相容性 metadata。不管最終是否上架，先把套件弄成「可發、可被發現」狀態，方便 browser-arena、ba-case-builder 等下游日後以正式版本相依。

## What Changes

- 修正 crates/fcb/LICENSE 與 crates/fcb-wasm/LICENSE 第 87 行的著作權佔位字為 "Copyright 2026 The fcb-rs Authors"，與根 LICENSE 一致，使每個 publishable crate 打包出的 LICENSE 都帶完整歸屬、無範本殘留。
- 在 crates/fcb-wasm/Cargo.toml 為 fcb path dependency 補上 version = "0.1.0"，使 fcb-wasm 可被打包與發佈（fcb 需先發）。
- 在 crates/fcb/Cargo.toml 與 crates/fcb-wasm/Cargo.toml 補上 crates.io metadata：keywords、categories、authors、documentation、以及 rust-version（MSRV，依相依 crate 的已知下限保守宣告）。
- 驗證打包 fcb 與（補 version 後）打包 fcb-wasm 皆通過。

## Non-Goals

- 不修改任何 .rs 原始碼、不變更 codec 行為。
- 不收 cdylib、不刪 crates/fcb/src/wasm.rs：核心 fcb 的 fcb_version() 經 fcb-wasm 聚合洩漏並被 browser-arena 的 bridge.version() 消費，移除會靜默破壞下游消費者。
- 不把 README 的 git dependency 改成 crates.io 版本相依——那是實際上架時才做的動作。
- 不執行實際發佈、也不在 registry 占位保留名稱。
- 不含 wasm/npm build pipeline，亦不含 MSRV 的 CI 驗證 gate——兩者歸 reproducible-wasm-npm-pipeline（change B）。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `oss-project-docs`: 擴充「License file is present」要求，使每個 publishable crate（fcb、fcb-wasm）打包進套件的 LICENSE 也必須帶完整著作權歸屬、無範本佔位字並與根一致；並新增要求：publishable crate 的 manifest 必須帶可發佈/可被發現的 crates.io metadata（含可打包的 intra-workspace dependency 版本宣告，以及 keywords、categories、rust-version）。

## Impact

- Affected specs: oss-project-docs（modified）
- Affected code:
  - Modified: crates/fcb/Cargo.toml, crates/fcb-wasm/Cargo.toml, crates/fcb/LICENSE, crates/fcb-wasm/LICENSE
