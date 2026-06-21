## 1. characterization 實證（決策一：先以 characterization 測試實證 serde-wasm-bindgen 的邊界編碼）

- [ ] 1.1 寫 wasm characterization 測試：把 2^53-1、2^53、2^53+1、一個 > 2^53 的大整數、真浮點 3.14、以及對應 BigInt 各當 record 值 pack，斷言並記錄每個值進到 ciborium 後是 Value::Integer 還是 Value::Float、bundle_hash 是否與 native 對應值一致，據此釘死偵測述詞。驗證：wasm_bindgen_test 執行並輸出各值的編碼與 hash 比對結果。

## 2. 偵測 helper 與拒絕（決策二：拒絕會造成 hash 分歧的歧義數值，保留 safe int、真浮點與 BigInt；決策三：偵測實作於反序列化後的 ciborium Value 樹，以 Malformed 系錯誤拒絕）

- [ ] 2.1 先寫 failing native 單元測試：對含「整數值 Value::Float 且絕對值 ≥ 2^53」的 ciborium Value 樹，偵測 helper 回 malformed；對 safe-range 整數、真浮點、BigInt 對應整數則通過。驗證：cargo test -p fcb-wasm 由紅轉綠。
- [ ] 2.2 實作遞迴走訪 ciborium Value（含 Map / Array 巢狀）的偵測 helper，落地 `Deterministic numeric encoding at the pack boundary`：歧義數值回 discriminable malformed error（沿用既有 kind）。若 1.1 實證顯示須在 JS 端區分 Number 與 BigInt，則依決策三退路改於 wasm marshaling 層走訪 JsValue。驗證：2.1 測試轉綠。

## 3. 套用兩條 pack 路徑（決策四：契約同時套用 packCase 與 packSubmission）

- [ ] 3.1 在 pack_case 與 pack_submission 於 seal 前呼叫偵測 helper，使超範圍 Number record 在 authoring 時 loud reject、不產生 bytes。驗證：新增測試——packCase 與 packSubmission 對含超範圍 Number 的 record 各回 malformed kind。

## 4. 邊界測試矩陣

- [ ] 4.1 新增邊界測試矩陣：safe-range 整數 JS↔native bundle_hash 一致、真浮點被接受且 byte-stable、超範圍 Number 被 malformed 拒絕、BigInt 超範圍整數無損 round-trip 並由 native reader 正確解出。驗證：cargo test --workspace 與 wasm_bindgen_test 全綠。

## 5. 驗證關卡

- [ ] 5.1 全工作區綠燈：cargo test --workspace 全綠、cargo clippy --all-targets --all-features -- -D warnings 無警、cargo build -p fcb --target wasm32-unknown-unknown 成功。驗證：三道指令皆成功結束。
