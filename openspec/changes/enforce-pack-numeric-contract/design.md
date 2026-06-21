## Context

fcb-wasm-bridge spec 的「Case Authoring (Pack Case)」已敘明：safe-integer 範圍內整數其 canonical payload 與 bundle_hash 不依賴 JS number 表示，範圍外整數 MUST 以 BigInt 提供。但 fcb-wasm 的 pack 邊界（crates/fcb-wasm/src/lib.rs 的 pack_case / pack_submission，經 serde_wasm_bindgen::from_value 反序列化成 CaseInput / Submission）**未強制**此契約：超範圍整數若以普通 JS number 傳入，會在 JS 端先精度流失或被編成 CBOR float，與 native 的 u64/i64 整數編碼分歧，造成 bundle_hash 不一致且靜默。submission 的 record 值同樣有風險。既有測試只覆蓋整數 7。

## Goals / Non-Goals

**Goals:**

- 在 pack 邊界把既有 BigInt 契約變成強制不變式：歧義／會分歧的數值在 authoring 時 loud fail。
- 保留 safe-range 整數與真正浮點數的接受與確定性；BigInt 超範圍整數無損編成 CBOR 整數。
- 以邊界測試矩陣鎖定行為，消除靜默分歧。

**Non-Goals:**

- 不觸碰 container AEAD/AAD 工作（authenticate-header-as-aad change）。
- 不更新使用者文件散文（另一個 docs change）。
- 不改變 safe-range 內既有且正確的數值行為（不重新編碼、不正規化）。
- 不為超範圍整數提供自動「Number→BigInt」轉換（會掩蓋作者意圖；改以拒絕讓作者顯式處理）。

## Decisions

### 決策一：先以 characterization 測試實證 serde-wasm-bindgen 的邊界編碼

apply 的第一步是一支 wasm characterization 測試：把 2^53-1、2^53、2^53+1、一個 > 2^53 的大整數、一個真浮點（3.14）、以及對應的 BigInt 各自當 record 值 pack，觀察 (a) 進到 ciborium 後是 Value::Integer 還是 Value::Float、(b) bundle_hash 是否與 native 對應值一致。此實證結果釘死決策三的偵測述詞，並驗證它確實涵蓋所有會分歧的情境。

替代方案：不實證、直接假設 serde 行為。否決原因：偵測述詞的正確性完全取決於 serde-wasm-bindgen 對 Number 與 BigInt 的實際編碼，臆測會留下漏網或誤殺。

### 決策二：拒絕會造成 hash 分歧的歧義數值，保留 safe int、真浮點與 BigInt

pack 邊界對「整數值但因超出 safe-integer 範圍而以 float 形式進入、會與 native 整數編碼分歧」的 record 值，以可辨識錯誤拒絕；safe-range 整數（serde 已編為 CBOR 整數）與真正非整數浮點（如 3.14）照常接受且確定性；以 BigInt 提供的超範圍整數無損編成 CBOR 整數而通過。

替代方案：靜默接受並文件化「請用 BigInt」。否決原因：留下靜默 hash 分歧，正是本 change 要消除的核心風險；唯有在決策一實證顯示「拒絕在技術上不可行」時，才退回以測試鎖定文件化行為，但仍須讓分歧可被偵測。

### 決策三：偵測實作於反序列化後的 ciborium Value 樹，以 Malformed 系錯誤拒絕

主要作法：在 pack_case / pack_submission 反序列化得到 CaseInput / Submission 後，遞迴走訪其 record 值的 ciborium Value 樹，對「整數值的 Value::Float 且絕對值 ≥ 2^53」回傳 discriminable error（沿用既有 malformed kind，訊息指明「out-of-safe-range integer must be supplied as BigInt」）。此偵測落在 native-testable core，可用建構好的 Value 樹做 native 單元測試。

退路：若決策一實證顯示 serde 把超範圍 Number 與 BigInt 編成無法區分的同一 CBOR 整數，則改在 wasm marshaling 層走訪原始 JsValue（typeof number 且 Number.isInteger 且 !Number.isSafeInteger 即拒絕），以 wasm_bindgen_test 覆蓋。

替代方案：自動把超範圍 Number 轉成整數編碼。否決原因：JS 端早已精度流失，轉了也是錯值，反而掩蓋問題。

### 決策四：契約同時套用 packCase 與 packSubmission

同一套偵測同時用於 pack_case 與 pack_submission 的 record 走訪，讓「數值確定性」成為整個 pack 邊界的不變式，而非只在 packCase。

替代方案：只修 packCase。否決原因：submission 的 notes / report / activity 同樣帶任意數值、同樣影響其 bundle_hash 與 binding，漏掉等於留半個洞。

## Implementation Contract

**Behavior（可觀察行為）：**
- 以普通 JS number 傳入「整數值但超出 safe-integer 範圍」的 record 值 → packCase / packSubmission 回傳 discriminable error（malformed kind），不產生 bytes。
- safe-range 整數：JS 與 native 的 canonical payload 與 bundle_hash 相同。
- 真正浮點數（3.14）：被接受、round-trip 確定性。
- 以 BigInt 提供的超範圍整數：無損編成 CBOR 整數，native reader 正確解出。

**Interface / data shape：**
- crates/fcb-wasm/src/lib.rs：新增一個遞迴走訪 ciborium Value（含 Map / Array 巢狀）的偵測 helper，回傳 Result；pack_case 與 pack_submission 在 seal 前呼叫它。錯誤經既有 to_js_error → JS Error 帶 kind。
- 不新增公開 wasm API，不改變既有函式簽章（僅在內部前置驗證）。

**Failure modes：**
- 歧義數值 → malformed（loud，authoring 時即報）。
- 既有錯誤（empty manifest、wrong passphrase 等）行為不變。

**Acceptance criteria：**
- characterization 測試（決策一）證實偵測述詞涵蓋所有分歧情境。
- 邊界測試矩陣：safe-range 整數 hash 一致、真浮點被接受且確定、超範圍 Number 被 malformed 拒絕、BigInt 超範圍整數無損通過。
- cargo test --workspace 全綠、cargo clippy --all-targets --all-features -- -D warnings 無警、cargo build -p fcb --target wasm32-unknown-unknown 過、wasm_bindgen_test 通過。

**Scope boundaries：**
- In scope：pack_case / pack_submission 的數值前置驗證與偵測 helper、邊界測試矩陣、characterization 測試。
- Out of scope：container AEAD/AAD、使用者文件、manifest.records 驗證、safe-range 內既有數值行為。

## Risks / Trade-offs

- [決策三的 native-path 述詞若與 serde 實際行為不符會誤殺真浮點或漏掉分歧值] → 決策一的 characterization 測試先行釘死述詞，並保留 JS 端走訪退路。
- [整數值的大浮點（如 1e300）會被一併拒絕] → 視為可接受：此類值本就無法當 u64 整數確定性編碼，要求作者顯式（BigInt 或字串）反而安全。
- [遞迴走訪深巢狀 record 的成本] → record 值通常淺且小，走訪為線性且只在 pack 路徑一次。
