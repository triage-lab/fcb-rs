## Why

消費端（瀏覽器 workbench、教師出題工具）要產出 `.case` bundle 時，`fcb` crate 沒有提供公開的 case payload 信封建構 helper：`CasePayload { streams: Vec<StreamData> }` 目前只在測試本地（crates/fcb/tests/vectors.rs、crates/fcb/tests/stream_types.rs）與 WASM bridge（crates/fcb-wasm/src/lib.rs）各自重複定義，每個消費者都得自己手組 `{streams:[...]}` 信封，序列化方式無單一權威。同時 `binding::compute_bundle_hash(bytes)` 接受任意 bytes，但「canonical bundle_hash 該涵蓋哪些 bytes」並未凍結——docs/fcb-wire-format.md §5 已建議 canonical = sha256(明文 payload bytes)，但 codec 未提供、也未凍結這個保證。這兩點是 docs 的 Known Gaps #1、#2。

## What Changes

- 新增公開型別 `CasePayload { streams: Vec<StreamData> }`，作為 `.case` payload 的權威信封。
- 新增單一 canonical 序列化函式（生產端與消費端共用），其輸出等價於現行 `cbor::encode(&CasePayload { streams })`，使 case payload 的明文 bytes 有單一權威來源。
- 凍結 canonical `bundle_hash` helper：定義 `bundle_hash = compute_bundle_hash(canonical_payload_bytes)`，依 docs §5 即 sha256(明文 payload bytes)，並以固定 streams 的已知 sha256 值寫成回歸測試凍結。
- 新增 `pack_case(...)`：組 `{streams:[StreamData]}` 信封 → 由 canonical 序列化算出 bundle_hash → 組 manifest（與選用 task）為 header meta → 交給 `bundle::pack_bytes` 封裝；風格對齊既有 `submission::pack_submission`。
- WASM bridge（crates/fcb-wasm/src/lib.rs）改用 crate 公開的 `CasePayload`，消除重複定義。
- 測試 crates/fcb/tests/vectors.rs 的 `build_case()` 改走新公開 canonical helper 組 payload。

## Non-Goals (optional)

- **不改既有 wire format / container layout**：既有 golden vectors `FROZEN_CASE_HEX`、`FROZEN_WORK_HEX` 必須逐位元不變（`case_vector_is_byte_stable`、`work_vector_is_byte_stable` 持續綠）。
- **不保證整包 sealed bundle 逐位元穩定**：`bundle::pack_bytes` 內部隨機產生 salt/nonce，故 `pack_case` 的整包輸出本就不可重現；byte-stability 只保證 canonical 明文 payload 序列化這一層。
- **不在此 change 處理** `fcb.netflow.v1` / `fcb.json.v1` 記錄 schema（屬後續 change）與 `Submission` byte-stable 向量（屬後續 change）。
- **不擴充 WASM 對外 JS API 表面**：僅讓 bridge 重用公開型別，不新增 `openBundle` / `packCase` 類 JS 綁定。

## Capabilities

### New Capabilities

- `fcb-case-builder`: `.case` payload 的權威建構介面——`CasePayload` 信封型別、canonical 明文 payload 序列化、凍結的 canonical `bundle_hash` 定義，以及把 evidence streams（與選用 task）封裝為密封 `.case` bundle 的 `pack_case` 產出函式。

### Modified Capabilities

(none)

## Impact

- Affected specs: 新增 `fcb-case-builder`。
- Affected code:
  - New: crates/fcb/src/case.rs
  - Modified: crates/fcb/src/lib.rs, crates/fcb/src/binding.rs, crates/fcb/tests/vectors.rs, crates/fcb-wasm/src/lib.rs, docs/README.md, docs/fcb-wire-format.md, docs/fcb-reference.md
  - Removed: (none — 測試本地與 bridge 的重複 `CasePayload` 定義改為重用公開型別)
