## Context

v0.1.0 已釋出。`docs/` 文件在對應 code 落地前寫成、之後未同步，導致協定參考三大文件（`docs/fcb-reference.md`、`docs/fcb-wire-format.md`、`docs/fcb-data-model.md`）含事實錯誤、過時 Known-Gap，以及大量硬編 `path:line` 行號錨點——後者每次 refactor 都會整批失效，是反覆漂移的根因。同時 `README.md` 與 `CONTRIBUTING.md` 宣稱的 CI quality gate（format、clippy、`fcb-wasm` build）並未被 `.github/workflows/ci.yml` 實際強制。完整逐條證據在 `.spectra/analysis/2026-06-29-doc-audit.md`，本設計不重述、僅引用其結論。

## Goals / Non-Goals

**Goals:**

- 讓非 spec 的 OSS 文件與使用者手冊在內容上與 v0.1.0 的 code 一致（消除事實錯誤與過時敘述）。
- 以結構性手段（去錨點）根除 `path:line` cite 漂移，使文件於日後 refactor 不再失效。
- 讓 CI 真正強制 `README.md` / `CONTRIBUTING.md` 已宣稱的 quality gate。
- 補上 GitHub citation 入口與少量 OSS 一致性雜項。

**Non-Goals:**

- 不重新組織 `openspec/specs/` 架構，不合併或拆分既有 spec（僅就受影響 capability 增修 requirement delta）。
- 不做方向 C 的 SSOT 全面收斂（CBOR 表 / syslog schema / error catalog 多副本合併）——獨立排程。
- 不為核心 `fcb` crate 新增對稱的 `open_case`（Rust 端內容定址重算的 code 解法）——列為 future work，本次僅以文件說明既有不對稱。
- 不新增 ISSUE/PR template、CODEOWNERS 等其餘 OSS 鷹架。
- 不更動 codec 行為、on-disk 格式或公開 API 簽章。

## Decisions

### 去錨點 convention：以符號名與 test-pinned 錨點取代行號

`docs/` 內所有 `path/file.rs:NN` 與 `:NN-MM` 行號區間一律移除，改以「檔名 + 符號名」引用（例如 container 的 prefix 組裝函式、`case::pack_case`、`FcbError::Malformed`、evidence 的 stream 解碼函式）。精確數值（container 長度、`hdr_len`、版本常數）改由 named constants（`CONTAINER_VERSION`、`READER_VERSION`）與 `crates/fcb/tests/vectors.rs` 的 frozen golden vectors 背書，這些是測試釘住、refactor 不會位移的錨點。`docs/fcb-reference.md` 維持「精確數值之家」的定位，但其精確性自此建立在符號與 vector 上。
替代方案：保留行號但每次 refactor 重新產生（被否決——等於排程下次返工，且無工具強制，必再漂移）。

### CI gate 補強順序：先正規化格式，再加檢查

實測 clippy 已乾淨，但 cargo fmt 不乾淨（5 個 source 檔需重排）。因此順序固定為：先對全 workspace 跑一次格式化（純機械重排、無行為變更、不動 golden vectors），確認 workspace test 仍全綠後，才在 `.github/workflows/ci.yml` 加入 format 檢查、clippy（warnings 視為錯誤）、與 `fcb-wasm` 的 wasm build 三道 gate。若顛倒順序，CI 一加即因既有格式問題轉紅。

### spec delta 範圍：新 requirement 與 conformance 修正分流

只有「引入新規範」者寫成 spec delta：(1) 協定參考文件須準確且符號錨定 → `user-reference-and-changelog`；(2) CI 須實際強制已記載 gate、(3) 須發布 citation metadata → `oss-project-docs`；(4) 整合指南須說明 Rust 端內容定址重算與 `pack_case` manifest 一致性不變式、error-kind 須涵蓋 deserialize-time 無 kind 失敗 → `user-integration-guide`。其餘（修 license 字串、刪除矛盾段、改聯絡信箱、填 LICENSE 著作權、de-emoji、`.gitignore`）屬讓文件符合既有要求或 code 的 conformance 修正，僅落在 tasks，不增修 requirement。

### CITATION.cff 內容與 authors 預設

新增根目錄 `CITATION.cff`（CFF 1.2.0），欄位：title、type software、authors 預設 entity「The fcb-rs Authors」搭配聯絡信箱、repository-code 與 url 指向 GitHub repo、license ECL-2.0、version 0.1.0、date-released 2026-06-29、keywords（digital-forensics、evidence-bundle、codec、education）。authors 暫以 entity 表示；若日後要顯示真實姓名再補。

### 聯絡窗口與 OSS 一致性雜項

`SECURITY.md` 與 `CODE_OF_CONDUCT.md` 的聯絡/回報窗口統一改為 0826@fhsh.tp.edu.tw；移除 `CHANGELOG.md` 版本標題的裝飾 emoji（子標題 emoji 保留）；`crates/fcb-wasm/pkg/`（wasm-pack 生成物）加入 `.gitignore`。CoC 維持英文（Contributor Covenant 英文為常態）。

## Implementation Contract

- **可觀察結果（文件）**：`docs/` 各檔不再含任何 `path:line` 行號錨點；`docs/fcb-reference.md` 不再出現 MIT/Apache 字樣；三文件能力表與實際 `openspec/specs/`（11 dirs / 7 個 fcb-*）一致、不引用 plugin-protocol 或 query-model；`docs/fcb-data-model.md` 不再宣稱缺少 `pack_case`/`CasePayload`；`docs/fcb-integration-guide.md` 的 binding 段不再宣稱 header 未認證，且 Footgun 1 正確標示為 deserialize-time 無 kind 失敗。
- **可觀察結果（CI/build）**：`.github/workflows/ci.yml` 含 format 檢查、clippy（deny warnings）、`fcb-wasm` wasm build 三道 step；在乾淨 tree 上 cargo fmt 檢查通過、clippy 無 warning、workspace test 全綠。
- **新增介面/資料**：根目錄存在合法 `CITATION.cff`（可被 GitHub 解析出 Cite this repository）。
- **失敗模式**：CI 任一 gate 失敗即整體 red（fmt 不符、clippy warning、wasm build 失敗）。
- **驗收**：(a) 全文 grep `docs/` 無 `\.rs:[0-9]` 形式行號；(b) grep `docs/` 無 `MIT OR Apache`；(c) cargo fmt 全 workspace 檢查 exit 0；(d) clippy deny-warnings exit 0；(e) cargo test workspace 全綠（golden vectors 不變）；(f) `CITATION.cff` 通過 CFF schema（或 cffconvert 驗證）。
- **In scope**：§Impact 列出的文件、`.github/workflows/ci.yml`、5 個 src 檔（僅格式）、新增 `CITATION.cff`、`.gitignore`。**Out of scope**：codec 行為/格式/API、`openspec/specs/` 架構重整、核心 `open_case` 新增、其餘 OSS 鷹架。

## Risks / Trade-offs

- [cargo fmt 重排可能意外更動非預期檔] → 先跑 fmt 後以 git diff 確認僅 5 個預期檔、且皆為空白/換行層級變更，再跑 workspace test 確認行為不變。
- [去錨點後 `docs/fcb-reference.md` 喪失「逐行精確」賣點] → 以符號名 + golden vector 維持等價的可驗證性；reference 的價值改建立在 test-pinned 錨點而非揮發性行號。
- [新增 clippy/fmt gate 後未來 PR 更容易 red] → 屬刻意取捨；這正是 CONTRIBUTING 已承諾、本次要落實的品質門檻。
- [spec delta 與「specs 不在範圍」的使用者意圖張力] → 僅對既有 capability 增修 requirement、不動 spec 架構，符合 propose 正常流程且已於 discuss 階段告知。
