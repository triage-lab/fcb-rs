## 1. TDD 紅燈：先以測試固定契約

- [x] 1.1 在 crates/fcb/tests/vectors.rs 新增 `case_canonical_bundle_hash_is_frozen` 測試：對固定兩條 streams 的 `CasePayload` 呼叫 `fcb::case::case_bundle_hash`，斷言等於一個寫死的 `sha256:` 值（值先留 `todo` 佔位，1.2 實作後填入實算結果）。完成定義：測試存在且先紅（編譯失敗或斷言失敗）。驗證：`cargo test -p fcb --test vectors case_canonical_bundle_hash_is_frozen`。
- [x] 1.2 在 crates/fcb/tests/vectors.rs 新增 `pack_case_round_trips_and_binds_hash` 測試：以 manifest（含一個 built-in 與一個第三方 type）、選用 task、`CasePayload` 呼叫 `fcb::case::pack_case`，再 `bundle::open_bytes` 還原；斷言 kind=Case、`manifest_from_meta` 與 `task_from_meta` 可讀回、`decode_streams` 還原相同 records 且 built-in 旗標正確、header `bundle_hash == case::case_bundle_hash(&payload)`。完成定義：測試存在且先紅。驗證：`cargo test -p fcb --test vectors pack_case_round_trips_and_binds_hash`。

## 2. 實作 fcb-case-builder（crates/fcb/src/case.rs）

- [x] 2.1 新增模組 crates/fcb/src/case.rs 並在 crates/fcb/src/lib.rs 以 `pub mod case;` 掛載；定義公開 `CasePayload { pub streams: Vec<StreamData> }`（`#[serde(default)]` on streams；derive `Serialize, Deserialize, Debug, Clone, PartialEq`）。完成定義：`fcb::case::CasePayload` 可在 crate 外建構與序列化。驗證：`cargo build -p fcb` 通過、`cargo test -p fcb` 可解析新型別。
- [x] 2.2 （Requirement: Authoritative case payload envelope）實作 `CasePayload::to_canonical_bytes(&self) -> Result<Vec<u8>>`（內部 `cbor::encode(self)`）作為唯一序列化入口。完成定義：對相同 `CasePayload` 多次呼叫輸出相同 bytes，且解碼回相同 streams。驗證：1.1/1.2 測試與既有 `frozen_case_vector_decodes_to_expected_structure` 綠。
- [x] 2.3 （Requirement: Frozen canonical bundle hash）實作 `case::case_bundle_hash(payload: &CasePayload) -> Result<String>` = `binding::compute_bundle_hash(&payload.to_canonical_bytes()?)`，回傳 `sha256:<64 hex>`。完成定義：與直接對 canonical bytes 取 `compute_bundle_hash` 一致、與隨機 salt/nonce 無關。驗證：`case_canonical_bundle_hash_is_frozen` 綠（填入實算值後）。
- [x] 2.4 （Requirement: Case bundle production）定義 `CaseInput { case_id: String, manifest: Vec<StreamManifest>, task: Option<TaskSpec>, payload: CasePayload }` 與 `case::pack_case(input: &CaseInput, passphrase: &str) -> Result<Vec<u8>>`：算 canonical bytes → `bundle_hash` → 組 header meta（`{ streams, task? }`，與既有 `CaseMeta` shape 相容、可被 `manifest_from_meta` 與 `task_from_meta` 讀回）→ `BundleParams::new(Case, case_id, bundle_hash, meta)` → `bundle::pack_bytes`。完成定義：`pack_case` 產出可被 `open_bytes` 還原。驗證：`pack_case_round_trips_and_binds_hash` 綠。

## 3. 收斂重複定義與既有護欄

- [x] 3.1 [P] 將 crates/fcb/tests/vectors.rs 的本地 `struct CasePayload` 移除，`build_case()` 的 payload 改用 `fcb::case::CasePayload` + `to_canonical_bytes`。完成定義：payload 組裝走公開型別。驗證：`case_vector_is_byte_stable` 綠且 `FROZEN_CASE_HEX` 未改動（`git diff` 無 hex 變更）。
- [x] 3.2 [P] 將 crates/fcb/tests/stream_types.rs 的本地 `CasePayload` 改用 `fcb::case::CasePayload`。完成定義：syslog round-trip 測試仍走公開型別。驗證：`cargo test -p fcb --test stream_types` 綠。
- [x] 3.3 [P] （Requirement: Shared envelope across producer and consumer）將 crates/fcb-wasm/src/lib.rs 的本地 `struct CasePayload` 移除、改用 `fcb::case::CasePayload`，並更新原註解（不再是 known gap）。完成定義：bridge 重用公開型別、`open_case` 行為不變。驗證：`cargo test -p fcb-wasm` 綠（含 `open_case_decodes_streams_and_preserves_raw`）。

## 4. 品質關卡與文件

- [x] 4.1 跑品質關卡並全過：`cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`（零警告）、`cargo test --workspace`（含所有 `*_vector_is_byte_stable`）、`wasm-pack build crates/fcb-wasm --target nodejs`。完成定義：四關全綠。驗證：四道指令各自 exit 0。
- [x] 4.2 更新 docs：移除 docs/README.md §已知缺口 與 docs/fcb-wire-format.md §9 的 pack_case（#1）與 canonical bundle_hash（#2）兩條，並補上 `pack_case` / canonical `bundle_hash` 的正式說明；docs/fcb-reference.md §9 表格對應列同步移除。完成定義：兩條 Known Gap 不再出現、新增章節描述新 API。驗證：`rg "pack_case|bundle_hash" docs/` 內容反映新狀態、人工審閱章節正確。
