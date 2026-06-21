# 貢獻指南（Contributing）

感謝你願意為 `fcb-rs` 貢獻！本專案是 FCB（Forensic Case Bundle）協定的權威 Rust 實作，同時編譯為 native 與 WASM。協定相容性與位元組穩定性是第一優先，請先讀完本頁再動手。

## 開始之前

- 安裝 Rust stable 與 `wasm32-unknown-unknown` target、以及 [`wasm-pack`](https://drager.github.io/wasm-pack/)。
- 先讀 [`docs/README.md`](./docs/README.md) 與 [`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md) 建立協定脈絡。

## 鐵則：不可破壞既有 golden vectors

`crates/fcb/tests/vectors.rs` 內的 **frozen 向量**（`FROZEN_CASE_HEX`、`FROZEN_WORK_HEX`、`FROZEN_SUBMISSION_HEX`、`FROZEN_CASE_BUNDLE_HASH`、`FROZEN_CASE_PAYLOAD_HEX` …）是跨實作相容性與格式回歸的權威基準。對應的 `*_vector_is_byte_stable` 測試**必須持續綠**。

- **新增向量 OK；改既有向量的位元組不行。** 若你的變更會讓既有向量漂移，那通常代表你動到了 wire format——請先開 issue 討論，並把它當成 breaking change 處理。
- 同理，stream-type 的 `*_records_round_trip_byte_faithfully` 凍結測試也不得退。

## 品質關卡（送 PR 前一定要全過）

凡動到 `crates/**/*.rs`（含 rustdoc），請依序跑、全過才送：

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings   # 零警告
cargo test --workspace                                      # 全綠，golden vectors 不退
wasm-pack build crates/fcb-wasm --target nodejs             # WASM 仍能 build
```

純文件變更可略過上述四關，但若意外改到 Rust 請補跑。

## 開發流程：Spectra Spec-Driven Development

本 repo 用 Spectra 做 SDD（規格驅動開發），規格在 `openspec/specs/`、變更提案在 `openspec/changes/`。建議流程：

1. **propose**：`/spectra-propose <change>` 產出 proposal / spec / tasks。
2. **apply**：`/spectra-apply <change>` 依 tasks 實作（`.spectra.yaml` 設 `tdd: true`，請測試先行）。
3. **archive**：`/spectra-archive <change>` 收斂、把 delta spec sync 回 `openspec/specs/`。

小型修補不一定要走完整流程，但凡牽涉協定行為／新增 capability，請以 spec 為準。

## Commit 與 PR 慣例

- Commit 與 PR 描述使用**台灣繁體中文**，技術名詞保留英文。
- 一個 PR 聚焦一個目的；描述清楚動機（why）、變更（what）、如何驗證（how to test）。
- 確保 CI（`.github/workflows/ci.yml`：`cargo test --workspace` + `wasm32` build smoke）為綠。

## 授權

送出貢獻即表示你同意你的貢獻以本專案授權 **Educational Community License v2.0（ECL-2.0）** 釋出（見 [`LICENSE`](./LICENSE)）。

## 行為準則

參與本專案即受 [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md)（Contributor Covenant v2.1）約束。

## 回報安全問題

**請勿**用 public issue 回報漏洞；請依 [`SECURITY.md`](./SECURITY.md) 的私密管道回報。
