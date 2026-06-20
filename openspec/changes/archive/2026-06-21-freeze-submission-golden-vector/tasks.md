## 1. 凍結真實 7 欄 Submission 向量

- [x] 1.1 （Requirement: Submission has a byte-stable golden vector）在 crates/fcb/tests/vectors.rs 新增 `build_submission()`（用既有固定 salt/nonce 的 `build()` 路徑、`KIND=work`、payload = `cbor::encode` 真實 7 欄 `Submission`，header case_id/bundle_hash 與 Submission 一致）、常數 `FROZEN_SUBMISSION_HEX`（先佔位、由首跑填入實際 hex）、測試 `submission_vector_is_byte_stable()`。完成定義：`hex::encode(build_submission()) == FROZEN_SUBMISSION_HEX`。驗證：`cargo test -p fcb --test vectors submission_vector_is_byte_stable`。
- [x] 1.2 （Requirement: Submission has a byte-stable golden vector）新增 `frozen_submission_vector_decodes_to_expected_structure()`：`open_submission(FROZEN_SUBMISSION_HEX, PASS)` 還原並斷言 7 欄（含 `student.id`/`student.name`、`notes`/`activity` 內容、`exported_at`）皆等於輸入。完成定義：7 欄全數還原相符。驗證：`cargo test -p fcb --test vectors frozen_submission_vector_decodes_to_expected_structure`。

## 2. 凍結 canonical case payload 位元組（補強 Phase 1）

- [x] 2.1 在 crates/fcb/tests/vectors.rs 新增常數 `FROZEN_CASE_PAYLOAD_HEX`（先佔位）與測試 `case_canonical_payload_is_byte_stable()`：對 `fcb::case::CasePayload { streams: fixed_case_streams() }` 取 `to_canonical_bytes()`、斷言 `hex::encode(...) == FROZEN_CASE_PAYLOAD_HEX`，釘住生產／消費共用的 canonical 明文 payload 位元組（與 `case_canonical_bundle_hash_is_frozen` 互補）。完成定義：canonical payload bytes 逐位元釘住。驗證：`cargo test -p fcb --test vectors case_canonical_payload_is_byte_stable`。

## 3. 文件與品質關卡

- [x] 3.1 更新 docs：移除 docs/fcb-data-model.md §14「`Submission` 無 byte-stability 凍結」缺口條目並標記已關閉；docs/fcb-data-model.md §6 Submission 段補「已由 `FROZEN_SUBMISSION_HEX` 凍結」；docs/fcb-reference.md §8/§9 補 Submission golden vector、移除對應缺口備註。完成定義：三處不再把 Submission 列為未凍結缺口。驗證：`rg "Submission.*byte-stab|Submission.*無.*向量|Submission.*未凍" docs/` 反映已關閉、人工審閱。
- [x] 3.2 跑品質關卡並全過：`cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`（零警告）、`cargo test --workspace`（含 `case_vector_is_byte_stable`、`work_vector_is_byte_stable`、新增 submission/canonical 向量）、`wasm-pack build crates/fcb-wasm --target nodejs`。完成定義：四關全綠、既有向量不變。驗證：四道指令各自 exit 0。
