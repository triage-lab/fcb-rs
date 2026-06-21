## Context

`fcb` crate 目前能 `bundle::pack_bytes`（底層封裝）與 `submission::pack_submission`（`.casework` 產出），但沒有對應的 `.case` 產出 helper。`CasePayload { streams: Vec<StreamData> }` 信封在 crates/fcb/tests/vectors.rs、crates/fcb/tests/stream_types.rs、crates/fcb-wasm/src/lib.rs 各有一份重複定義；canonical `bundle_hash` 的涵蓋範圍（要 hash 哪些 bytes）只在 docs/fcb-wire-format.md §5 以建議形式存在，未由 codec 提供也未凍結。

約束：既有 golden vectors（crates/fcb/tests/vectors.rs 的 `FROZEN_CASE_HEX`、`FROZEN_WORK_HEX`）是跨實作相容性與格式回歸的權威基準，本 change 不得使其位移。專案 `.spectra.yaml` 設 `tdd:true`、`audit:true`、`locale:tw`。

## Goals / Non-Goals

**Goals**
- 提供單一權威的 `.case` payload 信封型別與 canonical 序列化。
- 凍結 canonical `bundle_hash = sha256(canonical 明文 payload bytes)`。
- 提供 `pack_case` 產出函式，風格對齊 `submission::pack_submission`。
- 讓 WASM bridge 重用公開型別，消除重複定義。

**Non-Goals**
- 不更動 wire format / container layout；既有 frozen vectors 逐位元不變。
- 不保證 `pack_case` 的整包 sealed bundle 逐位元穩定（salt/nonce 隨機）。
- 不處理 netflow/json schema 或 Submission 向量（後續 change）。
- 不新增對外 WASM JS API。

## Decisions

**D1：新模組 `crates/fcb/src/case.rs`，承載 `CasePayload` + `pack_case` + canonical helpers。**
- 理由：對齊既有 `submission.rs`（型別 + packer 同檔）。`evidence.rs` 維持讀取/解碼語意（`decode_streams`、manifest），`case.rs` 承載產出/建構語意。
- 替代方案：把 `CasePayload` 放 `evidence.rs`。否決理由：會把「資料模型」與「建構器」混在一個模組，且 `pack_case` 還需依賴 container/binding/task，放 evidence 反而耦合更多。

**D2：canonical 序列化 = `CasePayload` 以 ciborium 經 `cbor::encode` 編出的 bytes。**
- `CasePayload { streams: Vec<StreamData> }`，欄位 `streams` 加 `#[serde(default)]`（與 bridge 既有寬鬆解碼一致）。`#[serde(default)]` 只影響反序列化，不改編碼 bytes。
- 提供 `CasePayload::to_canonical_bytes(&self) -> Result<Vec<u8>>`（內部即 `cbor::encode(self)`）作為唯一序列化入口。
- 理由：型別 shape 與現行測試本地 `CasePayload` 完全相同，ciborium 對相同 struct shape 的輸出具決定性 ⇒ 將 vectors.rs 的 payload 改走此入口後，bytes 逐位元不變。

**D3：canonical `bundle_hash` helper = `case::case_bundle_hash(payload) = binding::compute_bundle_hash(&payload.to_canonical_bytes()?)`。**
- `binding::compute_bundle_hash`（通用 SHA-256 primitive）維持不動；case 層 helper 把它與 canonical 序列化組合，凍結「hash 明文 payload bytes」這個語意。
- 理由：保留低階 primitive 的泛用性，同時提供高階凍結保證。

**D4：`pack_case` 簽章對齊 `pack_submission`。**
- 形如 `pack_case(input: &CaseInput, passphrase: &str) -> Result<Vec<u8>>`，其中 `CaseInput { case_id, manifest: Vec<StreamManifest>, task: Option<TaskSpec>, payload: CasePayload }`（KDF 用預設，必要時提供覆寫欄位）。
- 流程：canonical bytes → bundle_hash → 組 header meta（`{ streams, task? }`，與既有 `CaseMeta` shape 相容，確保 peek/open 仍可讀）→ `BundleParams::new(Case, case_id, bundle_hash, meta)` → `bundle::pack_bytes(&params, &canonical_bytes, passphrase)`。
- 理由：manifest 帶 per-stream `type`（`StreamData` 沒有），故型別資訊由 `manifest` 提供、records 由 `payload` 提供，兩者由呼叫端一致給入。

## Implementation Contract

**對外行為與介面（apply 必須交付）**
- `fcb::case::CasePayload`：公開、`Serialize + Deserialize + Debug + Clone + PartialEq`，欄位 `pub streams: Vec<StreamData>`（`#[serde(default)]`）。
- `fcb::case::CasePayload::to_canonical_bytes(&self) -> fcb::Result<Vec<u8>>`：等價 `cbor::encode(self)`。
- `fcb::case::case_bundle_hash(payload: &CasePayload) -> fcb::Result<String>`：回傳 `sha256:<64 hex>`，等於 `compute_bundle_hash(payload.to_canonical_bytes())`。
- `fcb::case::pack_case(input: &CaseInput, passphrase: &str) -> fcb::Result<Vec<u8>>`：產出 `KIND=Case` 的密封 bundle；其 header `bundle_hash` 欄位等於 `case_bundle_hash(&input.payload)`；header meta 同時可被 `evidence::manifest_from_meta` 與 `task::task_from_meta` 讀回。
- WASM bridge（crates/fcb-wasm/src/lib.rs）移除本地 `struct CasePayload`，改用 `fcb::case::CasePayload`。

**Acceptance（可驗證）**
- 既有測試 `case_vector_is_byte_stable`、`work_vector_is_byte_stable`、`frozen_case_vector_decodes_to_expected_structure` 全綠且 `FROZEN_CASE_HEX` 未改動：把 vectors.rs `build_case()` 的 payload 組裝改用 `CasePayload::to_canonical_bytes` 後仍逐位元相同。
- 新增測試（凍結 canonical bundle_hash）：對固定 streams 的 `case_bundle_hash` 等於一個寫死的已知 `sha256:` 值。
- 新增 round-trip 測試：`pack_case` 產出的 bundle 經 `bundle::open_bytes` / bridge `open_case` 能還原出相同 manifest、task 與 stream records，且 header `bundle_hash` == `case_bundle_hash(payload)`。
- `cargo clippy --all-targets --all-features -- -D warnings` 零警告；`wasm-pack build crates/fcb-wasm --target nodejs` 通過。

**Scope**
- In scope：crates/fcb/src/case.rs（新）、crates/fcb/src/lib.rs（掛模組）、crates/fcb/tests/vectors.rs（payload 改走公開型別 + 新凍結測試）、crates/fcb-wasm/src/lib.rs（重用公開型別）。
- Out of scope：container/crypto/compress 既有實作、netflow/json schema、Submission 向量、WASM JS API 表面。

## Risks / Trade-offs

- [canonical 序列化若與舊 bytes 有任何差異 → FROZEN_CASE_HEX 失敗] → Mitigation：型別 shape 完全沿用、序列化走同一 `cbor::encode`；以 `case_vector_is_byte_stable` 作為硬性護欄，紅就回退。
- [`#[serde(default)]` 影響編碼] → Mitigation：serde default 僅作用於反序列化路徑，不影響 encode 輸出；由 byte-stable 測試實證。
- [pack_case 的 meta 組裝與既有 CaseMeta shape 不符，導致 peek/open 讀不到 manifest/task] → Mitigation：round-trip 測試覆蓋 peek manifest + task_from_meta。

## Migration Plan

1. 新增 `case.rs` 與公開型別/函式（先寫紅測試，TDD）。
2. 將 vectors.rs / stream_types.rs / bridge 的本地 `CasePayload` 改為重用公開型別。
3. 跑 `cargo test --workspace` 確認 frozen vectors 全綠。
4. 更新 docs（移除 Known Gaps #1/#2，補 canonical bundle_hash 與 pack_case 章節）。
