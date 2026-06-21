## Why

上一批過夜自動化（6 個 phase，已全數 merge 進 main）新增與編修了大量繁體中文散文文件，但 humanization 與台灣慣用語把關不足：`humane-prose-audit` 多半只跑到 Phase 1（deterministic 檢查）就停，沒有實際套用 Stage-5 改寫；「gap phase」對既有 docs 的大量編修則完全沒跑過 humanization。結果是這批散文 AI 味偏重、句構單調，且潛藏非台灣慣用的繁中詞彙。這個 change 專門收這個尾。

## What Changes

以**兩個正交 pass** 重修 10 個 in-scope 繁中散文檔，過程中不得改動任何事實或程式語意：

- **Pass A — 用語 firewall（全域、零容忍）**：對全部 10 檔（含三個規格級檔與其欄位表）掃除中國大陸用語、簡繁混用、翻譯腔；命中一律改成台灣慣用語，技術名詞（commit、cache、API、binding…）保留英文。此 pass **不吃**「精確度優先」的豁免。
- **Pass B — humanization（分級）**：
  - **敘事級（完整 humanize rewrite）**：`README.md`、`CONTRIBUTING.md`、`SECURITY.md`、`docs/README.md`、`docs/fcb-integration-guide.md`、`docs/fcb-cookbook.md`——降 AI 味、提升節奏與具體性。
  - **結構/規格級（輕量、保結構）**：`CHANGELOG.md`、`docs/fcb-wire-format.md`、`docs/fcb-data-model.md`、`docs/fcb-reference.md`——只拆明顯 AI-tell（如過長的 `；` 串接句），保留欄位表、句構與機器可解析的精確度。
- **稽核機制**：每檔完整跑 `humane-prose-audit`（含 Phase 2 persona 子代理）+ `ai-slop-auditor` 並**實際套用** rewrite findings；規格級檔的 MTLD/repetition 密度 flag 視為技術密度的預期現象，不為了降指標把規格改鬆。
- **驗收閘門**：以 git diff 確認硬約束 token 全數未動，且所有 markdown 連結仍指向存在的檔案。

為了不讓這個尾再被掃一次，本 change 把上述規則**沉澱成一條 normative 標準** `doc-language-standard`：往後任何繁中散文檔都受同一套用語 firewall 與硬約束保護，而非靠一次性人工把關。既有文件的內容與結構不變——這條 spec 規範的是「語言與用語」這個橫切面，不是各檔的功能要求。

## Non-Goals

- **不碰 `.rs` rustdoc**：crates 內 379 行 rustdoc 全為英文（中文 0 行），屬另一條軸線；且任何 `.rs` 改動都會觸發完整品質關卡，本 change 不納入。
- **不碰 canonical 正本**：`LICENSE`、`crates/fcb/LICENSE`、`crates/fcb-wasm/LICENSE`（ECL-2.0 SPDX 正本）、`CODE_OF_CONDUCT.md`（Contributor Covenant v2.1 官方正本）。
- **不碰 normative spec**：`openspec/specs/**`、`openspec/changes/**`（英文 SHALL/MUST 規格，非散文）。
- **不碰程式碼與測試**：任何 Rust 程式碼與測試，尤其 golden vector hex。
- **不重寫 `docs/fcb-reference.md` 的句構或欄位表**：它是機器可解析的權威規格，精確度優先於文采，只做用語與明確 AI-tell 修正。

## Capabilities

### New Capabilities

- `doc-language-standard`：橫切於全 repo 繁中散文文件的語言／用語標準。規範散文 SHALL 使用台灣繁體中文、SHALL NOT 含中國大陸用語或簡繁混用、技術名詞依台灣慣例保留英文，並把「prose 編修 SHALL NOT 改動硬約束 token（API 名、golden hex、CBOR marker、SPDX id、指令列、連結目標）」釘為可驗證的不變量。涵蓋範圍即本 change 的 10 個 in-scope 散文檔；排除 canonical 正本（LICENSE×3、CODE_OF_CONDUCT）、normative spec 與 rustdoc。

### Modified Capabilities

（無——既有 oss-project-docs、user-integration-guide、user-reference-and-changelog 三個 spec 治理這些文件的功能要求，本 change 不動其要求；只新增橫切的語言標準。）

## Impact

- Affected specs：新增 `doc-language-standard`（橫切語言／用語標準）。既有治理 spec（oss-project-docs、user-integration-guide、user-reference-and-changelog）的功能要求不變。
- Affected code：無（不動任何 `.rs` 或測試）。
- Modified（僅散文，全部相對於 repo root）：
  - 敘事級：`README.md`、`CONTRIBUTING.md`、`SECURITY.md`、`docs/README.md`、`docs/fcb-integration-guide.md`、`docs/fcb-cookbook.md`
  - 結構/規格級：`CHANGELOG.md`、`docs/fcb-wire-format.md`、`docs/fcb-data-model.md`、`docs/fcb-reference.md`
- New：無
- Removed：無
- 硬約束（humanization 不得改動）：API/型別/函式名、`FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` / `FROZEN_SUBMISSION_HEX` / `FROZEN_CASE_BUNDLE_HASH` / `FROZEN_CASE_PAYLOAD_HEX`、CBOR marker、常數值、SPDX id（ECL-2.0）、所有指令列、所有 markdown 連結目標。
