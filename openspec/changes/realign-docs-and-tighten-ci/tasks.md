## 1. 格式基線（CI 補強的前置依賴）

- [x] 1.1 全 workspace 套用 rustfmt，使 `crates/fcb/src/bundle.rs`、`crates/fcb/src/case.rs`、`crates/fcb/src/container.rs`、`crates/fcb/src/crypto.rs`、`crates/fcb-wasm/src/lib.rs` 通過格式檢查。可觀察結果：cargo fmt 全 workspace 檢查 exit 0。驗證：cargo fmt 檢查 exit 0；git diff 僅涵蓋這 5 檔且皆為空白／換行層級變更；cargo test workspace 全綠（frozen golden vectors 與 byte-stable 測試不變）。〔對應 design 決策：CI gate 補強順序：先正規化格式，再加檢查〕

## 2. 協定參考三大文件：去錨點與正確性（user-reference-and-changelog）

> 本組交付 requirement: Protocol reference docs are accurate and symbol-anchored；採 design 決策：去錨點 convention：以符號名與 test-pinned 錨點取代行號。

- [x] 2.1 [P] `docs/fcb-reference.md`：license 改 ECL-2.0；能力表對齊實際 `openspec/specs/`（11 dirs、7 個 fcb-*）並移除對 plugin-protocol／query-model 的引用與對 README 不存在段落的 deferral；WASM 與 payload-superset 兩條 Known-Gap 分別窄化為「僅 `fcb` crate 內部 stub」「僅 reader 端」；移除全部 `path:line` 行號、改符號名與 golden-vector 錨點。交付 requirement: Protocol reference docs are accurate and symbol-anchored。驗證：grep 該檔無 `\.rs:[0-9]`、無 `MIT OR Apache`、無 `plugin-protocol`／`query-model`。
- [x] 2.2 [P] `docs/fcb-wire-format.md`：修正對 README「7 capability」段的失效 deferral（改內聯實際 spec 清單或指向真實錨點）；`manifest.records` 敘述改為「僅 `pack_bytes` 不核對；`pack_case` 已強制」；移除全部行號、改符號名。驗證：grep 該檔無 `\.rs:[0-9]`、無「7 capability」失效引用。
- [x] 2.3 [P] `docs/fcb-data-model.md`：刪除「`fcb` crate 沒有 `pack_case`／`CasePayload`、請自行於 evidence 補上」的矛盾段，改述為兩者皆為公開 API；修正能力表與 query-model 引用；`manifest.records` 與 WASM Known-Gap 窄化；移除全部行號、改符號名。驗證：grep 該檔無 `\.rs:[0-9]`、無「沒有.*pack_case」字樣；`pack_case`／`CasePayload` 呈現為可用。

## 3. 使用者手冊修正（user-integration-guide）

> 本組交付 requirement: Integration guide documents Rust-side content-address verification and the pack invariant 與 requirement: Integration guide documents kind-less bridge deserialization failures。

- [x] 3.1 `docs/fcb-integration-guide.md`：修正 binding 段不再宣稱 header 未認證（改為「AEAD 真實性不等於 binding 正確性」）；Footgun 1 改述為「欄位名寫錯為 deserialize-time 失敗、丟出無 kind 例外、查 kind 得 unknown」；§1.3 補 Rust open 路徑須以 `case_bundle_hash` 重算並比對 header `bundle_hash`；補 `pack_case` 的 manifest 一致性不變式說明；補 `PeekInfo` 版本欄位的 version negotiation 說明。交付 requirement: Integration guide documents Rust-side content-address verification and the pack invariant；交付 requirement: Integration guide documents kind-less bridge deserialization failures。驗證：grep 該檔無「header 沒被認證」；內容含 `case_bundle_hash` 比對步驟、manifest 不變式、deserialize 無 kind 說明。
- [x] 3.2 [P] `docs/README.md`：修正指錯內容的 stream_types 行號引用（改符號名）；WASM 與 payload-superset Known-Gap 窄化；將低階手動封裝路徑與其 CBOR key-order footgun 移至 `pack_case` 權威 helper 之後；移除全部行號。驗證：grep 該檔無 `\.rs:[0-9]`；Known-Gap 呈現 reader-only；`pack_case` 段落位於手動路徑之前。
- [x] 3.3 [P] `docs/fcb-cookbook.md`：將內聯的安全須知（confirmation oracle、re-pack 敏感性）改為一行指向 wire-format／data-model 權威段落以去重；移除任何行號。驗證：grep 該檔無 `\.rs:[0-9]`；各 recipe 的呼叫名稱仍對得上實際 public API。

## 4. OSS 治理與 metadata

> 本組交付 requirement: Project publishes citation metadata；conformance 修正部分採 design 決策：spec delta 範圍：新 requirement 與 conformance 修正分流。

- [x] 4.1 [P] 新增根目錄 `CITATION.cff`（CFF 1.2.0：title、type software、authors entity「The fcb-rs Authors」+ 聯絡信箱、repository-code 與 url、license ECL-2.0、version 0.1.0、date-released 2026-06-29、keywords）。交付 requirement: Project publishes citation metadata。驗證：以 cffconvert（或等價 CFF schema 驗證）通過；version 與 license 與 crate manifests／CHANGELOG 一致。〔對應 design 決策：CITATION.cff 內容與 authors 預設〕
- [x] 4.2 [P] `LICENSE` Appendix 著作權由範本填為「2026 / The fcb-rs Authors」，與 `README.md` 的著作權聲明一致。驗證：grep `LICENSE` 無 `[yyyy]` 或 `name of copyright owner` 殘留。
- [x] 4.3 [P] `SECURITY.md` 與 `CODE_OF_CONDUCT.md` 的回報／聯絡窗口統一改為 0826@fhsh.tp.edu.tw。驗證：grep 兩檔含新信箱、不再含舊的 claude@fhsh.tp.edu.tw。〔對應 design 決策：聯絡窗口與 OSS 一致性雜項〕
- [x] 4.4 [P] `CHANGELOG.md` 移除版本 H2 標題的裝飾 emoji（子分類標題 emoji 保留）。驗證：`[0.1.0]` 版本 H2 行不含裝飾 emoji。
- [x] 4.5 [P] 將 wasm-pack 生成物 `crates/fcb-wasm/pkg/` 加入 `.gitignore`。驗證：git check-ignore 對 `crates/fcb-wasm/pkg/README.md` 命中。

## 5. CI 補強（依賴 1.1）

> 本組交付 requirement: CI enforces the documented quality gate；採 design 決策：CI gate 補強順序：先正規化格式，再加檢查。

- [x] 5.1 在 `.github/workflows/ci.yml` 加入三道 gate：format 檢查、clippy（warnings 視為錯誤、涵蓋 workspace 全 target）、`fcb-wasm` 的 WebAssembly build，與既有 workspace test 並列。可觀察結果：任一 gate 失敗即 workflow red。交付 requirement: CI enforces the documented quality gate。驗證：本機 cargo fmt 檢查 exit 0、clippy deny-warnings exit 0、`fcb-wasm` wasm build 成功、cargo test workspace 全綠；workflow 檔含上述三 step。

## 6. 全量驗收

- [x] 6.1 依 design 的 Implementation Contract 驗收條件逐項確認。驗證：(a) grep 全 `docs/` 無 `\.rs:[0-9]` 行號錨點；(b) grep 全 `docs/` 無 `MIT OR Apache`；(c) cargo fmt 全 workspace 檢查 exit 0；(d) clippy deny-warnings exit 0；(e) cargo test workspace 全綠且 golden vectors 不變；(f) `CITATION.cff` 通過 CFF 驗證。
