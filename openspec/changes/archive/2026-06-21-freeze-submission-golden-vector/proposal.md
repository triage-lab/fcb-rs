## Why

`crates/fcb/tests/vectors.rs` 的 `FROZEN_WORK_HEX` 凍結的是一個 **test-local 3 欄 `WorkPayload`**（`case_id`/`bundle_hash`/`report`），**不是** library 實際寫入 `.casework` 的 7 欄 `Submission`（`crates/fcb/src/submission.rs`：`case_id`、`bundle_hash`、`student`、`notes`、`report`、`activity`、`exported_at`）。因此目前**沒有任何 golden vector 釘住真實 `Submission` 的 on-disk 位元組**；`Submission` 只有 `submission_random_round_trip`（隨機 salt/nonce）覆蓋，證 round-trip 但**不證 byte-stability**。非 Rust 重實作者無從驗證 `Submission` 的逐位元相容性。這是 docs 的 Known Gap（`fcb-data-model.md §14`）。

## What Changes

- 在 `crates/fcb/tests/vectors.rs` 以既有的固定 salt/nonce `build()` 路徑，新增 **`FROZEN_SUBMISSION_HEX`** 與 `submission_vector_is_byte_stable()`：用真實 7 欄 `Submission` 當 payload、`KIND=work`，凍結其完整 on-disk 位元組；另加一個解碼測試斷言 7 欄全數還原。
- 同樣以 canonical 路徑新增 **`FROZEN_CASE_PAYLOAD_HEX`** 與 `case_canonical_payload_is_byte_stable()`：凍結 `fcb::case::CasePayload::to_canonical_bytes()` 對固定 streams 的明文 payload 位元組，證明生產／消費共用的 canonical 序列化 byte-stable（與 Phase 1 的 `case_canonical_bundle_hash_is_frozen` 互補：一個釘 hash、一個釘 bytes）。
- 更新 docs：`fcb-data-model.md §14`（移除 Submission 無 byte-stable 向量的 Known Gap）、`fcb-reference.md §8/§9`（補 Submission golden vector、移除對應缺口備註）、`fcb-data-model.md §6`（Submission 段補「已凍結」註記）。

## Non-Goals (optional)

- **不動既有向量**：`FROZEN_CASE_HEX`、`FROZEN_WORK_HEX` 及其測試逐位元不變（新向量為 additive；`FROZEN_WORK_HEX` 作為 test-local `WorkPayload` 的歷史向量保留）。
- **不改 wire format / container layout / crypto**：只新增凍結向量與測試。
- **不引入新的 `Submission` 欄位或語意**：凍結的是現行 7 欄 `Submission` 的序列化。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `fcb-submission`: 新增「`Submission` 具 byte-stable golden vector」requirement——真實 7 欄 `Submission` 的 on-disk 位元組由凍結向量釘住、可作跨實作相容性基準。

## Impact

- Affected specs: 修改 `fcb-submission`。
- Affected code:
  - Modified: crates/fcb/tests/vectors.rs, docs/fcb-data-model.md, docs/fcb-reference.md
  - New: (none)
  - Removed: (none)
