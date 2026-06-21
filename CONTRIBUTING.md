# 貢獻指南（Contributing）

謝謝你想替 `fcb-rs` 出一份力。這個專案是 FCB（Forensic Case Bundle）協定的權威 Rust 實作，一份程式碼同時編譯成 native 與 WASM 兩種目標。這裡最在意的兩件事，是協定相容性跟位元組穩定性——它們排在所有事情前面。動手之前，麻煩先把這頁讀完。

## 開始之前

- 裝好 Rust stable，補上 `wasm32-unknown-unknown` target，再裝 [`wasm-pack`](https://drager.github.io/wasm-pack/)。
- 花點時間翻過 [`docs/README.md`](./docs/README.md) 跟 [`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md)，把協定的來龍去脈搞清楚再開始。

## 鐵則：不可破壞既有 golden vectors

`crates/fcb/tests/vectors.rs` 裡那批 **frozen 向量**（`FROZEN_CASE_HEX`、`FROZEN_WORK_HEX`、`FROZEN_SUBMISSION_HEX`、`FROZEN_CASE_BUNDLE_HASH`、`FROZEN_CASE_PAYLOAD_HEX` 等等），是判斷跨實作能不能對得上、格式有沒有偷偷退步的權威基準。對應的 `*_vector_is_byte_stable` 測試，必須一直是綠的，沒有例外。

- **加新向量沒問題，但別去動既有向量的位元組。** 一旦你的變更讓既有向量漂掉，幾乎可以斷定你碰到了 wire format。這時先開 issue 把事情攤開來談，並當成 breaking change 來處理。
- stream-type 那邊的 `*_records_round_trip_byte_faithfully` 凍結測試同樣不能退，道理是一樣的。

## 品質關卡（送 PR 前一定要全過）

只要你碰了 `crates/**/*.rs`（rustdoc 也算），下面四關照順序跑一遍，全過了才送 PR：

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings   # 零警告
cargo test --workspace                                      # 全綠，golden vectors 不退
wasm-pack build crates/fcb-wasm --target nodejs             # WASM 仍能 build
```

如果你只改文件，這四關可以跳過；但要是不小心動到了 Rust，記得回頭補跑。

## 開發流程：Spectra Spec-Driven Development

這個 repo 用 Spectra 跑 SDD（規格驅動開發），規格放在 `openspec/specs/`，變更提案放在 `openspec/changes/`。一般會這樣走：

1. **propose**：`/spectra-propose <change>` 先把 proposal / spec / tasks 生出來。
2. **apply**：`/spectra-apply <change>` 照 tasks 一條條實作。`.spectra.yaml` 設了 `tdd: true`，所以測試要先寫。
3. **archive**：`/spectra-archive <change>` 收尾，把 delta spec sync 回 `openspec/specs/`。

小修小補不一定要把整套流程走完。不過只要碰到協定行為，或是要新增 capability，那就一切以 spec 為準。

## Commit 與 PR 慣例

- Commit 跟 PR 描述都用**台灣繁體中文**寫，技術名詞保留英文就好。
- 一個 PR 只做一件事。描述裡把動機（why）、改了什麼（what）、怎麼驗證（how to test）三件事交代清楚。
- 送出前確認 CI 是綠的（`.github/workflows/ci.yml`：跑 `cargo test --workspace` 加上 `wasm32` build smoke）。

## 授權

送出貢獻，就表示你同意讓它跟著本專案的授權一起釋出，也就是 **Educational Community License v2.0（ECL-2.0）**（細節見 [`LICENSE`](./LICENSE)）。

## 行為準則

只要你參與這個專案，就受 [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md)（Contributor Covenant v2.1）規範。

## 回報安全問題

發現漏洞**千萬別**開 public issue。請走 [`SECURITY.md`](./SECURITY.md) 裡的私密管道回報。
