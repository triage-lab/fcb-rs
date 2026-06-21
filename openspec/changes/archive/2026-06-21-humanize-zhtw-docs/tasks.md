## 1. 前置基準

- [x] 1.1 建立 Pass A 的大陸用語 firewall 對照清單（取材 `ai-slop-auditor` references/02），並擷取硬約束 token 基準快照——API/型別/函式名、`FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` / `FROZEN_SUBMISSION_HEX` / `FROZEN_CASE_BUNDLE_HASH` / `FROZEN_CASE_PAYLOAD_HEX`、CBOR marker、SPDX `ECL-2.0`、指令列、以及全 10 檔的 markdown 連結目標清單（對應 design 決策一：用語 firewall 與 humanization 拆成兩個正交 pass，支撐 spec 的 "Prose documentation uses Taiwan Traditional Chinese" 與 "Prose edits preserve hard-constraint tokens"）。完成時：清單與快照存在，且對全 10 檔跑過 baseline 大陸用語掃描。驗證：baseline 掃描輸出列出每檔命中數，連結目標清單逐一可解析到存在檔案。

## 2. 敘事級檔——完整 humanize（Pass B）+ 用語 firewall（Pass A）

- [x] 2.1 [P] `README.md`：完整跑 `humane-prose-audit`（含 Phase 2 persona 子代理）+ `ai-slop-auditor` 並實際套用 rewrite（design 決策五：每檔完整跑 humane-prose-audit + ai-slop-auditor 並實際套用改寫），同步做 Pass A 用語 firewall；針對其 9 處 `- **…**` 冒號清單破除平行句型。完成時：讀來人味自然、無大陸用語、事實與 API 名不變。驗證：audit 跑到 Phase 2 且 rewrite 已套用，git diff 比對 README.md 顯示硬約束 token 僅在未變動 context 行，逐詞大陸用語掃描回 0。
- [x] 2.2 [P] `CONTRIBUTING.md`：完整跑 `humane-prose-audit` + `ai-slop-auditor` 並套用 rewrite（決策五）+ firewall。完成時：散文更具人味、技術名詞保留英文、golden vector 名稱與品質關卡指令列原樣不動。驗證：rewrite 已套用，git diff 比對 CONTRIBUTING.md 無硬約束 token 變動，大陸用語掃描回 0。
- [x] 2.3 [P] `SECURITY.md`：完整跑 `humane-prose-audit` + `ai-slop-auditor` 並套用 rewrite（決策五）+ firewall；收斂其 6 處破折號 `—` 的節奏。完成時：揭露流程敘述自然、`bundle_hash`／`compute_bundle_hash` 等識別名不變。驗證：rewrite 已套用，git diff 比對 SECURITY.md 無硬約束 token 變動，大陸用語掃描回 0。
- [x] 2.4 [P] `docs/README.md`：完整跑 `humane-prose-audit` + `ai-slop-auditor` 並套用 rewrite（決策五）+ firewall。完成時：總覽散文人味化、可跑範例與連結目標不變。驗證：rewrite 已套用，git diff 比對 docs/README.md 無硬約束 token 變動且連結仍解析，大陸用語掃描回 0。
- [x] 2.5 [P] `docs/fcb-integration-guide.md`：完整跑 `humane-prose-audit` + `ai-slop-auditor` 並套用 rewrite（決策五）+ firewall；收斂其 16 處破折號密度。完成時：消費端敘述順暢、所有 Rust/JS code block 與 API 名（`peek_header`、`pack_case`、`verify_binding`…）一字不動。驗證：rewrite 已套用，git diff 比對 docs/fcb-integration-guide.md 顯示 code block 與 API 名未變，大陸用語掃描回 0。
- [x] 2.6 [P] `docs/fcb-cookbook.md`：完整跑 `humane-prose-audit` + `ai-slop-auditor` 並套用 rewrite（決策五）+ firewall。完成時：recipe 導引自然、`FROZEN_*` 引用與連結目標不變。驗證：rewrite 已套用，git diff 比對 docs/fcb-cookbook.md 無硬約束 token 變動，大陸用語掃描回 0。

## 3. 結構/規格級檔——輕量保結構（Pass B 輕量）+ 用語 firewall（Pass A）

- [x] 3.1 [P] `CHANGELOG.md`：依 design 決策二做輕量處理——只修用語與明顯 AI-tell，保留 Keep-a-Changelog 條列結構與版本標記；跑 `ai-slop-auditor` firewall。完成時：條列事實與 `FROZEN_SUBMISSION_HEX` 等名稱不變、無大陸用語。驗證：git diff 比對 CHANGELOG.md 顯示僅 prose 字句變動、結構標題與 token 不變，大陸用語掃描回 0。
- [x] 3.2 [P] `docs/fcb-wire-format.md`：依決策二輕量處理，拆解過長 `；` 串接句（baseline 57 處）但不動欄位表順序、欄位名與語意；跑 firewall。完成時：句子層 AI-tell 降低、wire 定義精確度不變。驗證：git diff 比對 docs/fcb-wire-format.md 顯示欄位表行未變、僅句子重組，大陸用語掃描回 0。
- [x] 3.3 [P] `docs/fcb-data-model.md`：依決策二輕量處理，收斂 `；`（baseline 86）與破折號（baseline 83）密度但保留 stream schema 欄位表與語意；跑 firewall。完成時：schema 表與型別名不變、句子更易讀。驗證：git diff 比對 docs/fcb-data-model.md 顯示欄位表未變、僅句子層調整，大陸用語掃描回 0。
- [x] 3.4 [P] `docs/fcb-reference.md`：依決策二/三以最保守幅度處理——精確度優先，只拆最明顯的 `；` 串接句（baseline 123）與修用語，欄位表、golden vector hex、CBOR marker、常數一律凍結。完成時：機器可解析精確度不變、無大陸用語。驗證：git diff 比對 docs/fcb-reference.md 顯示所有 `FROZEN_*` hex 與欄位表未變，大陸用語掃描回 0。

## 4. 驗收閘門與標準化

- [x] 4.1 硬約束 git-diff 閘門 + 連結完整性（design 決策四：硬約束 token 以 git-diff 閘門逐項驗證，落實 spec 的 "Prose edits preserve hard-constraint tokens" 與 "Technical terms retain their English form"）：對全 10 檔的 working tree diff 逐項確認硬約束 token 只出現在未變動 context 行，且每個 markdown 連結目標解析到存在檔案。完成時：閘門全過。驗證：diff 檢查回報 0 個被改的 `FROZEN_*`/CBOR/SPDX/指令/API 名，連結檢查回報 0 broken link；任一不過即回退對應檔重做。
- [x] 4.2 全域用語 firewall 終掃（design 決策三：用語 firewall 零容忍、不吃精確度豁免，落實 spec 的 "Prose documentation uses Taiwan Traditional Chinese"）：對全 10 檔重跑逐詞大陸用語掃描。完成時：0 命中（context-idiomatic 的 `對象`／`通過` 經人工確認為台灣慣用而保留）。驗證：掃描輸出全 10 檔皆 0 命中。
- [x] 4.3 規格檔精確度複查（design 決策二：依檔案角色分級——敘事級完整改寫、規格級輕量保結構，落實 spec 的 "Tiered humanization preserves machine-parseable specs"）：複查 `docs/fcb-wire-format.md`、`docs/fcb-data-model.md`、`docs/fcb-reference.md` 的欄位表順序、欄位名與語意未因 humanization 改變。完成時：三檔欄位表凍結確認。驗證：欄位表區段 diff 僅含 prose 行變動、無欄位增刪改序。
- [x] 4.4 排除清單複查 + 標準一致性（design 決策六：把語言／用語規則沉澱為 doc-language-standard spec，落實 spec 的 "Standard scope excludes canonical and normative sources"）：確認 rustdoc/`.rs`/`LICENSE`×3/`CODE_OF_CONDUCT.md`/`openspec/specs/**` 未被本批改動，並核對 `doc-language-standard` spec 的 firewall 與硬約束規則與實際套用一致。完成時：排除清單零改動、spec 與實作對齊。驗證：git diff 不含上述排除路徑，spectra validate humanize-zhtw-docs 通過。
