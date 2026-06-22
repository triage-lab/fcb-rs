## 1. 實作 producer 不變式

實作 spec 需求 **Pack rejects manifest inconsistent with payload**（`fcb-case-builder`）：

- [x] 1. 在 `crates/fcb/src/case.rs::pack_case` 緊接 `manifest.is_empty()` 檢查之後、計算 bundle_hash 之前，加入 consume-map 不變式：payload id → records.len() 映射（payload 重複 id 即錯）；逐 manifest entry `remove` 對應 id 並比對 `records.len() as u64 == manifest.records`（找不到→缺漏或 manifest 重複 id）；迴圈後若有剩餘→payload 多餘 stream。任一不符回 `FcbError::Malformed`（含 id 與兩邊數字）。合法輸入位元組不變。

## 2. 測試（TDD：先寫會紅的測試）

- [x] 2. 新增 reject 單元測試（mirror `pack_case_rejects_empty_manifest`）：records 計數不符、payload 多出未宣告 stream、payload 缺 manifest 宣告的 stream、重複 id（manifest 端與 payload 端）皆斷言 `Err(FcbError::Malformed(_))`。
- [x] 3. 確認合法 happy-path 不受影響：既有 `pack_case_round_trips_and_binds_hash` 與 `crates/fcb/tests/vectors.rs` 的 `case_vector_is_byte_stable`、`frozen_case_*` 全綠（位元組與 bundle_hash 不變）。

## 3. 驗證

- [x] 4. 跑 `cargo test --workspace`、`cargo clippy --all-targets --all-features -- -D warnings`（exit 0）、`cargo build -p fcb -p fcb-wasm --target wasm32-unknown-unknown`（OK）；確認 fcb-wasm `js_authored_case_hashes_identically_to_native` 仍綠（core 不變式自動繼承到 wasm pack）。
