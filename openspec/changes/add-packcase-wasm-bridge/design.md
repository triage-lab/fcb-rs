## Context

fcb-wasm 是 FCB codec 的 WASM／JS bridge，分兩層：crate root 的 native-testable core（回傳純 Rust 值、可用 cargo test 驗證，無需 wasm runtime），以及 cfg(target_arch="wasm32") 的 wasm_api 模組（用 serde-wasm-bindgen 與 JS 互轉）。目前 bridge 能開啟（openCase）`.case` bundle，卻不能產生它：對外少了 packCase，native core 也只有 pack_work、沒有 pack_case wrapper。producer 端的 fcb::case::pack_case 已存在且其輸出 bytes 已被 crates/fcb/tests/vectors.rs 的 FROZEN_CASE_HEX／FROZEN_CASE_PAYLOAD_HEX／FROZEN_CASE_BUNDLE_HASH 釘死。

關鍵限制：JS number 是 f64，若 JS 端的整數 record 反序列化成 Value::Float 而非 Value::Integer，canonical CBOR 與 bundle_hash 會與 native producer 不一致，破壞 byte-stability 與 binding 決定性。本設計在動工前已實測 serde-wasm-bindgen 0.6.5 原始碼以消解此風險。

## Goals / Non-Goals

**Goals:**

- 對外提供 packCase，沿用 packSubmission 的兩層 pattern，讓 JS-authored case 的 canonical payload 與 bundle_hash 和 native producer 一致（sealed bytes 因每次隨機 salt/nonce 而不同；零 FROZEN_* 漂移）。
- 以可在 cargo test 執行的 native wrapper 承載邏輯，並用跨 JS 邊界的 wasm 測試守住數值決定性。
- 文件化 JS 物件形狀與 footgun。

**Non-Goals:**

- 不新增 byte-stable golden vector：packCase 底層即同一條 fcb::case::pack_case，bytes 已被既有 vector 釘死。
- 不在 bridge 內做數值正規化／coercion。
- 不處理 builder（fcb-adapter）端的型別契約與 vitest（屬另一個 repo 的工作）。

## Decisions

### 決策一：CaseInput 直接加 Serialize／Deserialize derive，不用 wasm DTO

serde-wasm-bindgen 需要目標型別實作 Deserialize。CaseInput 目前只 derive Debug、Clone。選擇直接在 fcb crate 為 CaseInput 補 Serialize、Deserialize，而非在 fcb-wasm 內定義 mirror DTO 再 .into()。理由：CaseInput 的所有巢狀型別（CasePayload、StreamData、StreamManifest、TaskSpec、TaskStep、ReportMode）皆已 derive 兩者，補上後與兄弟型別一致；且 CaseInput 是 builder struct，pack_case 是另外組 CaseMeta + canonical payload bytes（不會把 CaseInput 本身序列化進 wire），因此補 derive 不影響任何 golden vector。DTO 方案多一層需同步維護的 boilerplate，無實質好處，故捨棄。

### 決策二：不做數值正規化，倚賴 serde-wasm-bindgen 的 safe-integer 行為

已讀 serde-wasm-bindgen 0.6.5 原始碼（de.rs 的 deserialize_any）：JS number 若 Number.isSafeInteger 為真，走 visit_i64 → Value::Integer；只有真正的非整數才走 visit_f64 → Value::Float。BigInt 也落在整數分支；StreamManifest.records（u64）走 deserialize_u64 → visit_u64。因此 JS 的 7 會成為 Value::Integer(7) → CBOR 0x07，與 native producer 一致。

捨棄主動 coercion（把整數值的 Float 改回 Integer），因為它有害：對超過 2^53-1 的整數，f64 早已遺失精度，coercion 救不回；對語意上是浮點的 5.0，coercion 會誤判成整數，反而與 native producer 產生新的漂移。殘餘風險（整數超過 2^53-1 須以 BigInt 傳入；真正的浮點兩側一致）改以文件約束處理。

### 決策三：雙層測試——native round-trip 加 wasm-bindgen-test

native round-trip 測試（cargo test）操作 Rust 建構的 Value，只能守住 wrapper 與 record 還原度，無法真正行經 serde-wasm-bindgen 的數值處理。因此另加一個 wasm-bindgen-test（以 wasm-pack test --node 執行），用 JSON.parse 建立真正的 JS 物件、跨越邊界，斷言 bundle_hash 等於既有 FROZEN_CASE_BUNDLE_HASH——這是唯一能驗證 JS 整數不退化成 Float 的測試。兩者並存：native 守 wrapper、wasm 守邊界決定性。

## Implementation Contract

**Behavior:** JS 端可呼叫 packCase(caseObject, passphrase)，得到 Uint8Array（sealed `.case` bytes）。該 bytes 經 openCase 還原後，case_id／streams／task 與輸入一致，且 bundle_hash 與相同邏輯輸入的 native producer 完全相同。

**Interface／data shape:** 新增 wasm export packCase（js_name = packCase），簽章為 pack_case(input: JsValue, passphrase: &str) -> Result<Vec<u8>, JsValue>，內部呼叫 native-core crate::pack_case(input: &CaseInput, passphrase: &str) -> Result<Vec<u8>, FcbError>（mirror pack_work）。JS 物件形狀：

- case_id: string
- manifest: 陣列，每筆 { id, type, records }——鍵為 type（非 stream_type），records 為非負整數
- task（可選）: { report_mode: "steps" | "freeform"（小寫）, instructions, steps: [{ id, prompt, answer_type }] }
- payload: { streams: [{ id, records: [...CBOR-able 值] }] }

**Failure modes:** 空 manifest → FcbError::Malformed → JS Error 帶 kind "malformed"（沿用 error_kind）；serde 反序列化失敗 → JsValue 字串錯誤訊息。

**Acceptance criteria:**

- native 測試 case_round_trips_through_bridge 通過：pack_case → open_case，case_id／streams[0].stream_type／records（含 syslog raw 欄位）／task.report_mode 一致，且兩次 pack 的 bundle_hash 相同。
- wasm 測試以 wasm-pack test --node 通過：JS-authored case（payload streams 鏡射 fixed_case_streams：s0 為 evt1／evt2 兩筆文字、s1 為整數 7）之 bundle_hash 等於 sha256:376d586b42b0e800a6e78fea8bfb9a68cb569d033cc324b7b9b1800fc508eccf。
- cargo test --workspace 全綠且所有 FROZEN_* 零漂移；cargo clippy --all-targets --all-features -- -D warnings 無警告；wasm-pack build crates/fcb-wasm --target nodejs 綠燈。

**Scope boundaries（In scope）:** CaseInput derive、native-core pack_case wrapper、packCase wasm binding、雙層測試、wasm-bindgen-test dev-dependency、fcb-wasm-bridge 與 user-integration-guide 的文件。**（Out of scope）:** 不改 fcb::case::pack_case 既有邏輯、不新增 golden vector、不做數值正規化、不碰 fcb-adapter／builder 端。

## Risks / Trade-offs

- [JS 整數退化成 Value::Float 破壞 bundle_hash] → 已從 serde-wasm-bindgen 0.6.5 原始碼確認 safe integer 走 visit_i64；並以 wasm-bindgen-test 斷言 bundle_hash 等於 FROZEN_CASE_BUNDLE_HASH 把關。
- [整數超過 2^53-1 以普通 JS number 傳入會被靜默轉成 Float] → 以文件約束：超過範圍須以 BigInt 傳入；此為取證 record 的邊界情境。
- [為 fcb public 型別 CaseInput 加 derive 擴大 API surface] → 與所有兄弟型別一致、純加性、不影響 wire／vector，風險極低。
- [serde-wasm-bindgen 未來版本改變數值行為] → wasm-bindgen-test 會在升級時立即抓出 bundle_hash 不一致。
