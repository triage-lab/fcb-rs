## Why

既有 `docs/` 是**協定權威**（wire format、data model、逐位元 reference），但缺一份**消費端視角的整合指南**：想用 `fcb-rs` 開封 `.case`、產出 `.casework`、或在瀏覽器接 WASM bridge 的人，目前得自己從 reference 拼出呼叫序。新增一份 getting-started 風格的整合指南，把「相依 → 開封/打包 → 處理 error → 驗 binding → 端到端」串成可照抄的可跑範例，並交叉連結既有權威文件（不複製其內容、不 fork-and-drift）。

## What Changes

- 新增 **`docs/fcb-integration-guide.md`**，涵蓋：
  - **Rust 路徑**：以 Cargo git dependency 相依 `fcb`；開封 `.case`（`bundle::open_bytes` / `peek_header` / `decode_streams`）、產出 `.case`（`case::pack_case`）、產出/開封 `.casework`（`submission::pack_submission` / `open_submission`）、驗 binding（`binding::verify_binding`）的可跑片段。
  - **WASM/JS 路徑**：`wasm-pack build` 產 `pkg/`，JS 端呼叫 `peekHeader` / `openCase` / `openSubmission` / `packSubmission` / `computeBundleHash` / `verifyBinding` / `workKey`；示範一個薄 adapter（對齊 `crates/fcb-wasm` 風格）。
  - **Error kind 處理**：`FcbError` 五種變體與 bridge 的 `error_kind` 字串（`bad-magic` / `unsupported-version` / `malformed` / `wrong-passphrase` / `corrupt`）對照與處理建議。
  - **Golden-vector 契約**：消費端如何用 `crates/fcb/tests/vectors.rs` 的 frozen 向量驗證自家（尤其非 Rust）實作的相容性。
  - **端到端流程**：teacher 出題 → student 開封作答 → 平台收件驗 binding 的完整時序。
- 在 `docs/README.md` 與根 `README.md` 的文件入口加一條指向整合指南的連結。

## Non-Goals (optional)

- **不改 codec / 測試 / 任何 `.rs`**：純文件。
- **不複製協定 reference**：細節仍以 `docs/fcb-wire-format.md` / `fcb-data-model.md` / `fcb-reference.md` 為權威，指南只做消費端串接與交叉連結。
- **不涵蓋發佈到 crates.io**（屬後續 release 工作）。

## Capabilities

### New Capabilities

- `user-integration-guide`: 消費端整合指南的文件契約——SHALL 涵蓋 Rust 與 WASM/JS 兩條路、可跑範例、error kind 處理、golden-vector 契約、端到端流程，並交叉連結既有權威 docs。

### Modified Capabilities

(none)

## Impact

- Affected specs: 新增 `user-integration-guide`（文件契約）。
- Affected code:
  - New: docs/fcb-integration-guide.md
  - Modified: docs/README.md, README.md
  - Removed: (none)
