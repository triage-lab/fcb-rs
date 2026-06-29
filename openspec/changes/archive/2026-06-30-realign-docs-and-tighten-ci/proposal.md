## Why

v0.1.0 已釋出，但 `docs/` 多份文件在對應 code 落地前寫成、之後未回頭同步，造成三類問題：協定參考三大文件含事實錯誤（license 仍寫 MIT/Apache、能力表計數錯且引用不存在的 spec、`docs/fcb-data-model.md` 宣稱沒有已存在的 `pack_case`/`CasePayload`）、過時的 Known-Gap，以及大量會隨 refactor 失效的 `file:line` 硬編錨點。同時 `README.md` 與 `CONTRIBUTING.md` 宣稱送 PR 前要過的 CI gate（format、clippy、`fcb-wasm` build）實際未被 `.github/workflows/ci.yml` 強制。完整審視與逐條證據見 `.spectra/analysis/2026-06-29-doc-audit.md`。

## What Changes

- **修正文件事實錯誤（E1–E6）**：`docs/fcb-reference.md` 的 license 改 ECL-2.0；修正三文件的能力表（實際 11 specs / 7 個 fcb-* spec，移除對不存在的 plugin-protocol、query-model spec 的引用，以及對 README 不存在段落的 deferral）；`docs/fcb-data-model.md` 刪除「沒有 `pack_case`/`CasePayload`、請自行實作」的自相矛盾段落；修正 `docs/fcb-integration-guide.md` 的 binding 理由（header 自 v2 已被 AEAD 認證）與 Footgun 1 的 error kind 描述（欄位名寫錯為 deserialize 失敗、丟出無 kind 的字串例外）；修正 `docs/README.md` 指錯內容的行號引用。
- **去錨點（結構性、防復發）**：移除 `docs/` 內所有 `path:line` 硬編行號，改以「檔名 + 符號名」引用，精確數值改由 named constants 與 golden vectors（test-pinned）背書。根除系統性的 cite 漂移。
- **窄化過時 Known-Gap**：三文件的「WASM 綁定僅 `fcb_version`」改述為「消費面完整 surface 在 `fcb-wasm`，僅 `fcb` crate 內部為 stub」；「manifest.records / payload 多出 stream 未核對未測」窄化為「僅 reader 端 `decode_streams` 不偵測 superset」，並補述 `pack_case` 已強制 manifest 與 payload 雙向相等並有測試。
- **補齊缺漏（GAP）**：`docs/fcb-integration-guide.md` 補 Rust open 路徑的內容定址重算步驟（核心 crate 不自動重算，需呼叫 `case_bundle_hash` 自行比對）、`pack_case` 的 manifest 一致性不變式、`PeekInfo` 版本欄位的 version negotiation 說明；填實 `LICENSE` Appendix 著作權為 2026 / The fcb-rs Authors。
- **補強 CI（決議①）**：先以 cargo fmt 正規化 5 個未符合格式的 source 檔，再於 `.github/workflows/ci.yml` 加入 format 檢查、clippy（warnings 視為錯誤）、與 `fcb-wasm` 的 wasm build，使 CI 真正強制 `README.md` / `CONTRIBUTING.md` 已宣稱的 quality gate。
- **新增 `CITATION.cff`**：提供 GitHub「Cite this repository」引用入口，欄位對齊 ECL-2.0、version 0.1.0、date-released 2026-06-29、repository 與 keywords。
- **OSS 一致性雜項**：`SECURITY.md` 與 `CODE_OF_CONDUCT.md` 的聯絡窗口改為 0826@fhsh.tp.edu.tw；移除 `CHANGELOG.md` 版本 H2 的裝飾 emoji；將 wasm-pack 生成物 `crates/fcb-wasm/pkg/` 加入 `.gitignore`。

無 BREAKING：本變更僅涉及文件、CI/build 設定、純格式重排與新增 metadata 檔，不更動 codec 行為、on-disk 格式或公開 API 簽章；frozen golden vectors 與 byte-stable 測試不受影響。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `oss-project-docs`：新增「CI 實際強制已記載的 quality gate」與「發布 citation metadata 檔」兩條 requirement。
- `user-integration-guide`：Rust 消費路徑須說明內容定址重算與 `pack_case` 的 manifest 一致性不變式；error-kind 指引須涵蓋 deserialize-time 的無 kind 失敗。
- `user-reference-and-changelog`：新增「協定參考文件須準確且以符號（非行號）錨定」requirement，涵蓋 `docs/fcb-reference.md`、`docs/fcb-wire-format.md`、`docs/fcb-data-model.md` 的去錨點與正確性。

## Impact

- Affected specs: oss-project-docs, user-integration-guide, user-reference-and-changelog
- Affected code:
  - New:
    - CITATION.cff
  - Modified:
    - README.md
    - CONTRIBUTING.md
    - SECURITY.md
    - CODE_OF_CONDUCT.md
    - LICENSE
    - CHANGELOG.md
    - .gitignore
    - .github/workflows/ci.yml
    - docs/README.md
    - docs/fcb-integration-guide.md
    - docs/fcb-cookbook.md
    - docs/fcb-reference.md
    - docs/fcb-wire-format.md
    - docs/fcb-data-model.md
    - crates/fcb/src/bundle.rs
    - crates/fcb/src/case.rs
    - crates/fcb/src/container.rs
    - crates/fcb/src/crypto.rs
    - crates/fcb-wasm/src/lib.rs
  - Removed: (none)
