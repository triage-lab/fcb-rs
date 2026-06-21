## Why

瀏覽器 workbench（ba-case-builder）目前只能透過 fcb-wasm bridge 開啟（openCase）`.case` bundle，卻無法像 native CLI 一樣產生 `.case` bytes。bridge 已有 packSubmission，但缺少對應的 packCase；native-testable core 也只有 pack_work、沒有 pack_case wrapper。少了這個 export，workbench 端就得自行 reimplement codec／crypto（專案明令禁止），也無法與既有 producer 對齊。

## What Changes

- 在 fcb-wasm bridge 新增 packCase export（js_name = packCase），沿用既有 packSubmission 的兩層 pattern：native-testable core 函式 + cfg(target_arch="wasm32") 的 wasm-bindgen wrapper。
- 在 fcb crate 為 CaseInput 補上 Serialize、Deserialize derive，使 JS 物件能經 serde-wasm-bindgen 反序列化成 CaseInput。CaseInput 是「輸入用」builder struct，從不被序列化進容器 wire，因此不會動到任何 golden vector。
- 錯誤路徑沿用既有 to_js_error／error_kind 對應；空 manifest 會回 Malformed，對外 kind 為 "malformed"。
- 不新增正規化（normalization／coercion）層：已從 serde-wasm-bindgen 0.6.5 原始碼確認，JS safe integer 會走 visit_i64 → Value::Integer，與 native producer 的 canonical CBOR 完全一致；主動 coercion 反而有害（無法救回 f64 已遺失的精度，且會把合法的 5.0 浮點誤判成整數而自造漂移）。
- 新增雙層測試：native round-trip 測試（以 cargo test 執行）守住 wrapper 與 record 還原度；wasm-bindgen-test（以 wasm-pack test --node 執行）跨越真實 JS 邊界，斷言 JS-authored case 的 bundle_hash 等於既有 FROZEN_CASE_BUNDLE_HASH。
- 補強文件：在 fcb-wasm-bridge spec 與 integration guide 標明 JS 物件形狀與兩個 footgun（manifest 用鍵 type 而非 stream_type；records 整數需落在 JS safe-integer 範圍，超過 2^53-1 需以 BigInt 傳入）。

## Capabilities

### New Capabilities

（無）

### Modified Capabilities

- `fcb-wasm-bridge`: 新增「Case Authoring (Pack Case)」requirement——bridge 對外多一個 packCase 操作，並規範 JS-authored case 與 native producer 的 byte-stable／bundle_hash 等價契約，以及空 manifest 的錯誤對應。
- `user-integration-guide`: 補上 packCase 的 JS 物件形狀說明與兩個 caveat（type 鍵、safe-integer 整數）。

## Impact

- Affected specs: fcb-wasm-bridge（modified）、user-integration-guide（modified）
- Affected code:
  - Modified:
    - crates/fcb/src/case.rs（CaseInput 補 Serialize／Deserialize derive）
    - crates/fcb-wasm/src/lib.rs（native-core pack_case wrapper、packCase wasm binding、雙層測試）
    - crates/fcb-wasm/Cargo.toml（新增 wasm-bindgen-test dev-dependency）
  - New:（無；測試以 inline 方式置於 crates/fcb-wasm/src/lib.rs）
  - Removed:（無）
