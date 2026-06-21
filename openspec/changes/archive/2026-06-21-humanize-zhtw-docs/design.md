## Context

上一批 6 個 phase（已全數 merge 進 main）產出與編修了 10 個繁中散文檔，但兩道把關都沒做足：`humane-prose-audit` 多半停在 Phase 1，沒有實際套用 Stage-5 改寫；「gap phase」對既有 docs 的大量編修完全沒跑過 humanization。

實掃現況（grep 全 10 檔）給了兩個對行動有用的事實：

1. **用語幾乎已乾淨**：逐詞掃 50+ 大陸用語 = 0 真實命中。`對象`（精修對象）、`通過`（通過 `cargo test`）在台灣語境皆為慣用，`觸發`／`解析` 亦為台灣慣用。所以 Pass A 在多數檔是「掃過確認」，但紅線照立。
2. **AI 味是結構性的**：真正的 AI-tell 是過長的全形 `；` 串接句與破折號 `—` 密度，集中在三個大規格檔（`；`：reference 123 / data-model 86 / wire-format 57）。

另一個約束來自硬資料：`crates/**` 內 379 行 rustdoc **全為英文（中文 0 行）**，且改 `.rs` 會觸發完整品質關卡——因此 rustdoc 不納入本 change。

## Goals / Non-Goals

**Goals:**

- 對 10 個 in-scope 繁中散文檔做徹底 humanization（降 AI 味、提升節奏與具體性），不破壞任何事實或程式語意。
- 對同一批檔做零容忍的台灣用語 firewall（含三個規格檔與欄位表）。
- 把語言／用語規則沉澱為可重複套用的 `doc-language-standard` spec，讓往後的散文檔不必再靠一次性人工把關。
- 留下可驗證的硬約束閘門：prose 編修後，硬約束 token 與所有 markdown 連結目標逐項未動。

**Non-Goals:**

- 不碰 `.rs` rustdoc（全英文、改動觸發品質關卡）、`LICENSE`×3 與 `CODE_OF_CONDUCT.md`（canonical 正本）、`openspec/**`（normative spec）、任何 Rust 程式碼與測試。
- 不為了降 AI-tell 指標而改動 `docs/fcb-reference.md` 的欄位表或句構（精確度優先）。
- 不改任何文件的功能內容或結構——本 change 只動「語言／用語／散文品質」這個橫切面。

## Decisions

### 決策一：用語 firewall 與 humanization 拆成兩個正交 pass

把工作拆成兩個範圍不同的 pass，避免「精確度優先」這條規格檔豁免污染到用語把關：

- **Pass A（用語 firewall）**：全域、零容忍，套用到**全部 10 檔**，含規格檔與欄位表。命中大陸用語／簡繁混用／翻譯腔一律改台灣慣用語，技術名詞保留英文。
- **Pass B（humanization）**：分級套用（見決策二），只處理散文節奏與 AI-tell。

替代方案：單一 pass 同時做用語＋改寫。否決——會讓「規格檔輕量」的豁免不小心也放過用語問題；兩者範圍不同，必須分開。

### 決策二：依檔案角色分級——敘事級完整改寫、規格級輕量保結構

Pass B 依檔案性質分兩級：

- **敘事級（完整 humanize rewrite）**：`README.md`、`CONTRIBUTING.md`、`SECURITY.md`、`docs/README.md`、`docs/fcb-integration-guide.md`、`docs/fcb-cookbook.md`。
- **結構/規格級（輕量、保結構）**：`CHANGELOG.md`（Keep-a-Changelog 條列）、`docs/fcb-wire-format.md`、`docs/fcb-data-model.md`、`docs/fcb-reference.md`。只拆明顯 AI-tell（如過長 `；` 串接句拆成兩句），不動欄位表順序、欄位名與語意。

替代方案：全檔一律完整改寫。否決——會破壞機器可解析規格的精確度與穩定度。

### 決策三：用語 firewall 零容忍、不吃精確度豁免

決策二的「規格檔輕量」只豁免 humanization，**不豁免用語**。即使在 `docs/fcb-reference.md`，只要出現一個大陸用語也必須改；差別只在「改法受限於不動欄位表與語意」。實務上目前掃描顯示規格檔已乾淨，所以此處多為「確認」而非「大改」，但紅線在 spec 中釘為 SHALL NOT。

### 決策四：硬約束 token 以 git-diff 閘門逐項驗證

每檔改完後，對 working tree 跑驗收：硬約束 token（API/型別/函式名、`FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` / `FROZEN_SUBMISSION_HEX` / `FROZEN_CASE_BUNDLE_HASH` / `FROZEN_CASE_PAYLOAD_HEX`、CBOR marker、常數、SPDX `ECL-2.0`、所有指令列）在 diff 中**只能出現在未變動的 context 行**；且所有 markdown 連結目標仍指向存在的檔案。任一條不過就回退該檔重做。

替代方案：靠人眼複查。否決——hex 與連結目標是最容易在改寫中被默默動到的東西，必須機械驗證。

### 決策五：每檔完整跑 humane-prose-audit + ai-slop-auditor 並實際套用改寫

對每個 in-scope 檔跑**完整** `humane-prose-audit`（含 Phase 2 persona 子代理抓 AI-tell/voice），不只 Phase 1；併用 `ai-slop-auditor`（其 references/02 的台灣 vs 大陸用語清單供 Pass A 參照），並**實際套用** rewrite findings。規格檔的 MTLD/repetition 密度 flag 視為技術密度的預期現象，不為降指標把規格改鬆——這類 flag 記為 acknowledged 而非待修。

### 決策六：把語言／用語規則沉澱為 doc-language-standard spec

新增一條橫切 spec `doc-language-standard`，把 Pass A 的用語規則與硬約束不變量釘成 normative（SHALL／SHALL NOT），涵蓋範圍為繁中散文文件、排除正本與 spec 與 rustdoc。這讓 firewall 從「一次性人工把關」變成可重複引用的標準，正面回應「上一批就是因為沒有標準才漏掉」這個根因。

## Implementation Contract

**Behavior（可觀察結果）**：10 個 in-scope 散文檔讀起來更像人寫、無大陸用語；所有程式事實、API 名、hex、連結、欄位表語意與改動前一致。`doc-language-standard/spec.md` 存在並通過 `spectra validate`。

**改動介面／資料形狀**：純文字編修，無程式介面變動。唯一新增的結構是 `openspec/changes/humanize-zhtw-docs/specs/doc-language-standard/spec.md`（英文 normative delta spec）。

**Failure modes**：

- 若改寫動到硬約束 token → 驗收閘門（決策四）必須擋下並回退該檔。
- 若連結目標被改壞 → 連結檢查失敗，回退。
- 若為降 AI 指標而改動規格檔欄位表 → 違反決策二/三，視為退件。

**Acceptance criteria**：

- `git diff` 顯示硬約束 token 僅出現在未變動 context；無 `FROZEN_*` hex、CBOR marker、SPDX、指令列、API 名被修改。
- 全部 markdown 連結目標解析到存在的檔案（含 repo 內相對連結）。
- 逐詞大陸用語掃描在全 10 檔回 0 命中。
- 每個 in-scope 檔都有跑過完整 `humane-prose-audit` + `ai-slop-auditor` 並套用其 rewrite（規格檔的密度 flag 標為 acknowledged）。
- `spectra validate humanize-zhtw-docs` 通過。

**Scope boundaries**：

- In scope：10 個散文檔的語言／用語／散文品質；新增 `doc-language-standard` spec。
- Out of scope：rustdoc、`.rs`、測試、`LICENSE`×3、`CODE_OF_CONDUCT.md`、`openspec/specs/**` 既有檔、各檔的功能內容與結構。

## Risks / Trade-offs

- [改寫時誤動 hex / 連結 / API 名] → 決策四的 git-diff 閘門逐項機械驗證，任一不過即回退。
- [規格檔被過度改寫、損及機器可解析精確度] → 決策二/三明確限制規格檔只做輕量與用語修正，欄位表凍結。
- [為降 MTLD/repetition 指標而把技術密度改鬆] → 決策五規定規格檔密度 flag 記為 acknowledged，不強制改寫。
- [Pass A 在已乾淨的檔變成空轉] → 接受；零容忍標準的價值在於把紅線釘進 spec，供往後重複套用，而非本批一定要抓到很多。
- [humanization 屬主觀，過度改寫反增 AI 味] → 以 persona 子代理（Phase 2）交叉檢查 voice，敘事檔以「人味/具體性」為準，不追求華麗。

## Open Questions

（無——範圍、分級、用語準則與硬約束已在 discuss 階段確認。）
