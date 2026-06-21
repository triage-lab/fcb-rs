# doc-language-standard Specification

## Purpose

TBD - created by archiving change 'humanize-zhtw-docs'. Update Purpose after archive.

## Requirements

### Requirement: Prose documentation uses Taiwan Traditional Chinese

Traditional Chinese prose documentation files in scope SHALL be written in Taiwan-idiomatic Traditional Chinese. They SHALL NOT contain mainland-China vocabulary, simplified characters, or mixed simplified/traditional text. When a flagged term is genuinely Taiwan-idiomatic in its context, it SHALL be retained rather than mechanically substituted.

#### Scenario: Mainland-China vocabulary is rejected

- **WHEN** a prose documentation file in scope contains a mainland-China term
- **THEN** the term SHALL be replaced with its Taiwan-idiomatic equivalent before the change is accepted

##### Example: terminology firewall substitutions

| Input (mainland / mixed) | Expected (Taiwan) | Notes |
| ------------------------ | ----------------- | ----- |
| 軟件 | 軟體 | software |
| 代碼 | 程式碼 | code |
| 默認 | 預設 | default |
| 用戶 | 使用者 | user |
| 接口 | 介面 | interface |
| 優化 | 最佳化 | optimize |
| 視頻 | 影片 | video |
| 文件夾 | 資料夾 | folder |
| 對象（指 OOP 物件） | 物件 | object (OOP sense) |
| 對象（指對應目標） | 對象 | retained — Taiwan-idiomatic for "target/subject" |
| 通過 `cargo test` | 通過 `cargo test` | retained — Taiwan-idiomatic for "pass a test" |


<!-- @trace
source: humanize-zhtw-docs
updated: 2026-06-21
code:
  - SECURITY.md
  - docs/README.md
  - docs/fcb-data-model.md
  - CONTRIBUTING.md
  - docs/fcb-integration-guide.md
  - docs/fcb-wire-format.md
  - docs/fcb-cookbook.md
  - README.md
-->

---
### Requirement: Technical terms retain their English form

Where Taiwan convention keeps a technical term in English, prose documentation SHALL retain the English form rather than translating it into a localized coinage. This applies to terms such as commit, PR, deploy, cache, API, binding, codec, payload, and header.

#### Scenario: English technical term is preserved

- **WHEN** prose references a technical term that Taiwan convention keeps in English
- **THEN** the English form SHALL be kept and SHALL NOT be replaced by a translated equivalent


<!-- @trace
source: humanize-zhtw-docs
updated: 2026-06-21
code:
  - SECURITY.md
  - docs/README.md
  - docs/fcb-data-model.md
  - CONTRIBUTING.md
  - docs/fcb-integration-guide.md
  - docs/fcb-wire-format.md
  - docs/fcb-cookbook.md
  - README.md
-->

---
### Requirement: Prose edits preserve hard-constraint tokens

Editing prose for language or humanization SHALL NOT alter any hard-constraint token. Hard-constraint tokens are: API, type, and function names; golden vector hex constants (FROZEN_CASE_HEX, FROZEN_WORK_HEX, FROZEN_SUBMISSION_HEX, FROZEN_CASE_BUNDLE_HASH, FROZEN_CASE_PAYLOAD_HEX); CBOR markers; constant values; the SPDX identifier ECL-2.0; command-line invocations; and every markdown link target. After editing, every such token SHALL appear only on unchanged context lines of the diff, and every markdown link target SHALL resolve to an existing file.

#### Scenario: Hard-constraint token is left unchanged

- **WHEN** a prose file is edited for language or humanization
- **THEN** every API name, golden vector hex constant, CBOR marker, constant, SPDX identifier, command line, and markdown link target SHALL be byte-identical to its pre-edit form

#### Scenario: Markdown links still resolve

- **WHEN** a prose file edit is complete
- **THEN** every markdown link target in that file SHALL point to a file that exists in the repository


<!-- @trace
source: humanize-zhtw-docs
updated: 2026-06-21
code:
  - SECURITY.md
  - docs/README.md
  - docs/fcb-data-model.md
  - CONTRIBUTING.md
  - docs/fcb-integration-guide.md
  - docs/fcb-wire-format.md
  - docs/fcb-cookbook.md
  - README.md
-->

---
### Requirement: Tiered humanization preserves machine-parseable specs

Humanization SHALL be applied in two tiers. Narrative documents MAY be fully rewritten for human voice and rhythm. Structural and reference-grade documents SHALL be edited only to remove clear AI-tells, and SHALL preserve field-table ordering, field names, and semantics. Reducing an AI-tell metric SHALL NOT justify altering a reference document's field tables or technical precision.

#### Scenario: Reference document keeps its field tables

- **WHEN** a reference-grade document is humanized
- **THEN** its field-table ordering, field names, and field semantics SHALL be unchanged, and only sentence-level AI-tells (such as overlong semicolon-chained sentences) MAY be revised


<!-- @trace
source: humanize-zhtw-docs
updated: 2026-06-21
code:
  - SECURITY.md
  - docs/README.md
  - docs/fcb-data-model.md
  - CONTRIBUTING.md
  - docs/fcb-integration-guide.md
  - docs/fcb-wire-format.md
  - docs/fcb-cookbook.md
  - README.md
-->

---
### Requirement: Standard scope excludes canonical and normative sources

This standard SHALL apply to Traditional Chinese prose documentation files. It SHALL NOT apply to canonical upstream sources (license files and the Contributor Covenant code of conduct), to normative specification files under the spec directory, or to rustdoc and source code. Edits made under this standard SHALL NOT change the functional content or structure of a document, only its language, terminology, and prose quality.

#### Scenario: Canonical source is left untouched

- **WHEN** the standard is applied across the repository
- **THEN** license files, the Contributor Covenant code of conduct, normative spec files, and rustdoc SHALL NOT be modified by language or humanization edits

<!-- @trace
source: humanize-zhtw-docs
updated: 2026-06-21
code:
  - SECURITY.md
  - docs/README.md
  - docs/fcb-data-model.md
  - CONTRIBUTING.md
  - docs/fcb-integration-guide.md
  - docs/fcb-wire-format.md
  - docs/fcb-cookbook.md
  - README.md
-->