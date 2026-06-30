## 1. 修正 publishable crate 的 LICENSE 著作權歸屬

- [x] 1.1 [P] （Requirement: License file is present）將 crates/fcb/LICENSE 第 87 行的 `Copyright [yyyy] [name of copyright owner]` 填為 `Copyright 2026 The fcb-rs Authors`，使 fcb 打包出的 LICENSE 與根 LICENSE 一致、無範本殘留。驗證：`grep -nE "yyyy|name of copyright owner" crates/fcb/LICENSE` 無輸出。
- [x] 1.2 [P] （Requirement: License file is present）將 crates/fcb-wasm/LICENSE 第 87 行同樣填為 `Copyright 2026 The fcb-rs Authors`。驗證：`grep -nE "yyyy|name of copyright owner" crates/fcb-wasm/LICENSE` 無輸出。
- [x] 1.3 （Requirement: License file is present）確認「打包後」的 LICENSE 不再含佔位字（這是真正會出貨的檔）。執行 `cargo package -p fcb --allow-dirty` 後，驗證：`grep -nE "yyyy|name of copyright owner" target/package/fcb-0.1.0/LICENSE` 無輸出。

## 2. 讓 fcb-wasm 可被打包（補 path dependency 版本）

- [x] 2.1 （Requirement: Publishable crate manifests carry crates.io metadata）在 crates/fcb-wasm/Cargo.toml 將 `fcb = { path = "../fcb" }` 改為 `fcb = { path = "../fcb", version = "0.1.0" }`，使 fcb-wasm 不再因相依缺 version 而無法打包（fcb 需先行發佈才能跑完整 verify build）。驗證：`cargo package --workspace --no-verify --allow-dirty` 成功（連 fcb 一起打包，未發佈的 fcb sibling 由 workspace 本地解析），且不再出現 "does not specify a version" 錯誤。注意：fcb 尚未發佈到 crates.io 前，單獨跑 `cargo package -p fcb-wasm --no-verify --allow-dirty` 仍會失敗——`--no-verify` 只略過 compile、不略過相依的 registry 解析，故會回報 "no matching package named `fcb` found"；必須與 fcb 同批（`--workspace`）打包才能讓 sibling 由 workspace 本地解析。

## 3. 補齊 fcb 的 crates.io metadata

- [x] 3.1 [P] （Requirement: Publishable crate manifests carry crates.io metadata）在 crates/fcb/Cargo.toml 的 [package] 補上以下欄位，使 fcb 可被發現且宣告 MSRV：`keywords = ["forensics", "cbor", "codec", "evidence", "encryption"]`、`categories = ["encoding", "cryptography"]`（不重複宣告 "wasm"——JS/WASM 入口由專責的 fcb-wasm crate 持有，fcb 本身是 dual-target 原生 library）、`authors = ["The fcb-rs Authors"]`、`documentation = "https://docs.rs/fcb"`、`rust-version = "1.87"`（真實下限：以 Cargo.lock 已解析相依的最高已宣告 MSRV 為準——ruzstd 0.8.3 為 1.87，twox-hash 2.1.2 為 1.81，wasm-bindgen 0.2 為 1.77，thiserror 2 為 1.68，argon2 0.5 為 1.65；舊值 1.74 漏算 ruzstd/twox-hash 故為不實宣告；長期防漂移的 pinned-toolchain build gate 仍由 change reproducible-wasm-npm-pipeline 負責）。驗證：`cargo metadata --no-deps --format-version 1` 中 fcb 的 keywords/categories/rust_version 非空，且 `cargo package -p fcb --allow-dirty` verify build 仍通過。

## 4. 補齊 fcb-wasm 的 crates.io metadata

- [x] 4.1 （Requirement: Publishable crate manifests carry crates.io metadata）在 crates/fcb-wasm/Cargo.toml 的 [package] 補上：`keywords = ["forensics", "wasm", "cbor", "codec", "evidence"]`、`categories = ["wasm", "encoding", "cryptography"]`、`authors = ["The fcb-rs Authors"]`、`documentation = "https://docs.rs/fcb-wasm"`、`rust-version = "1.87"`（與 fcb 一致）。另補上 `[package.metadata.docs.rs]`（`default-target` 與 `targets` 皆設為 `wasm32-unknown-unknown`），否則整個 JS-facing API 因 `#[cfg(target_arch = "wasm32")]` 而被 docs.rs 預設的 x86_64 build 排除、docs.rs 頁面近乎空白。驗證：`cargo metadata --no-deps --format-version 1` 中 fcb-wasm 對應欄位非空，且 `cargo package --workspace --no-verify --allow-dirty` 成功（fcb 尚未發佈，須與 fcb 同批打包讓 sibling 由 workspace 解析；單獨 `-p fcb-wasm` 會在 registry 解析階段失敗）。

## 5. 整體回歸與發佈前置驗證

- [x] 5.1 確認 manifest 與 LICENSE 變更未造成回歸：`cargo build --workspace`、`cargo test --workspace`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`cargo fmt --all --check` 全數通過。
- [x] 5.2 確認發佈前置達成且三份 LICENSE 一致：`cargo package -p fcb --allow-dirty` verify build 通過、`cargo package --workspace --no-verify --allow-dirty` 通過（fcb 尚未發佈，fcb-wasm 須與 fcb 同批打包讓 sibling 由 workspace 解析；單獨 `-p fcb-wasm` 會在 registry 解析階段失敗），且 root LICENSE、crates/fcb/LICENSE、crates/fcb-wasm/LICENSE 的著作權行皆為 `Copyright 2026 The fcb-rs Authors`、無佔位字（`grep -REn "yyyy|name of copyright owner" LICENSE crates/fcb/LICENSE crates/fcb-wasm/LICENSE` 無輸出）。
