## 1. fcb crate：讓 CaseInput 可反序列化

- [x] 1.1 [P] 為 crates/fcb/src/case.rs 的 CaseInput 補上 Serialize、Deserialize derive，使 serde-wasm-bindgen 能將 JS 物件反序列化成 CaseInput（對應 決策一：CaseInput 直接加 Serialize／Deserialize derive，不用 wasm DTO）。驗證：cargo test --workspace 全綠，所有 FROZEN_* 零漂移（CaseInput 不入 wire，故 vector 不動）。

## 2. fcb-wasm：native-core wrapper 與 packCase binding

- [x] 2.1 在 crates/fcb-wasm/src/lib.rs crate root 新增 native-core pack_case wrapper（mirror pack_work），簽章 pack_case(input: &CaseInput, passphrase: &str) -> Result<Vec<u8>, FcbError>，並擴充 use 引入 CaseInput 與 aliased fcb_pack_case。行為：native 端可直接產出 sealed .case bytes。驗證：被 3.1 round-trip 測試覆蓋並通過 cargo test。
- [x] 2.2 在 crates/fcb-wasm/src/lib.rs 的 wasm_api 模組新增 packCase（js_name = packCase）binding：serde_wasm_bindgen::from_value 反序列化 JsValue → CaseInput 後呼叫 crate::pack_case，錯誤走 to_js_error；空 manifest 回 kind "malformed"（對應 Requirement: Case Authoring (Pack Case)）。行為：JS 可呼叫 packCase(caseObject, passphrase) 取得 .case bytes。驗證：wasm-pack build crates/fcb-wasm --target nodejs 綠燈。

## 3. 測試：native round-trip 與跨 JS 邊界決定性

- [x] 3.1 在 crates/fcb-wasm/src/lib.rs 的 #[cfg(test)] mod tests 新增 sample_case_input() 與 case_round_trips_through_bridge：pack_case → open_case 斷言 case_id／streams[0].stream_type／records（含 syslog raw 欄位）／task.report_mode 一致，且兩次 pack 的 bundle_hash 相同（對應 決策三：雙層測試——native round-trip 加 wasm-bindgen-test 的 native 半部）。驗證：cargo test --workspace 通過。
- [x] 3.2 新增 wasm-bindgen-test（#[cfg(all(test, target_arch="wasm32"))]，sibling of wasm_api）：以 js_sys::JSON::parse 建立 JS case，payload streams 鏡射 fixed_case_streams（s0 為 evt1／evt2、s1 records 為整數 7），packCase → openCase，經 js_sys::Reflect 讀 bundle_hash 斷言等於 sha256:376d586b42b0e800a6e78fea8bfb9a68cb569d033cc324b7b9b1800fc508eccf（對應 決策二：不做數值正規化，倚賴 serde-wasm-bindgen 的 safe-integer 行為）。行為：證明 JS 整數走 Value::Integer、不退化成 Float。驗證：wasm-pack test --node crates/fcb-wasm 通過。
- [x] 3.3 [P] 在 crates/fcb-wasm/Cargo.toml 的 [target.'cfg(target_arch="wasm32")'.dev-dependencies] 新增 wasm-bindgen-test = "0.3"。行為：wasm 測試 harness 可用。驗證：wasm-pack test --node crates/fcb-wasm 能編譯並執行 3.2。

## 4. 文件

- [x] 4.1 [P] 更新 integration guide 的 WASM/JS 消費路徑，將 packCase 加入 bridge surface 列表，並記錄兩個 footgun：manifest 用鍵 type（非 stream_type）、records 整數須落在 JS safe-integer 範圍（超過 2^53-1 以 BigInt 傳入），附 JS case 物件形狀範例（對應 Requirement: Integration guide covers the WASM/JS consumption path）。驗證：人工審閱 docs/fcb-integration-guide.md 與 README.md 含 packCase 範例與 caveat。
