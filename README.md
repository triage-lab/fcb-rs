# fcb-rs

[![ci](https://github.com/triage-lab/fcb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/triage-lab/fcb-rs/actions/workflows/ci.yml)
[![license: ECL-2.0](https://img.shields.io/badge/license-ECL--2.0-blue.svg)](./LICENSE)

**FCB（Forensic Case Bundle）協定的權威 Rust 實作。** 說穿了，它就是一個 codec：把證物（evidence）封裝成可攜、加密、又能驗證的 `.case` / `.casework` 檔。這套程式碼原本長在 browser-arena 數位鑑識教學平台裡，後來抽出來自成一個獨立 repo。

同一份 codec，編兩個目標。native 那一側餵的是出題 CLI 和教師審閱平台，WASM 那一側則跑在瀏覽器 workbench。核心相依清一色純 Rust——`ruzstd`、`argon2`、`chacha20poly1305`、`sha2`——一個 C FFI 都沒有，所以它才編得到 `wasm32`。

## FCB 是什麼？

- **`.case`**（KIND=case）是教師端出的題目。裡頭塞著 N 條具型別的證物 stream（syslog、netflow、json 等等），外加一份 task spec——而且這份 spec **不含答案**。
- 學生交回來的作答則是 **`.casework`**（KIND=work），裝著筆記、報告和活動紀錄。它靠 `case_id` 加 `bundle_hash` 綁回某一道題、某一個證物版本，跑不掉。

容器長這樣：`magic ‖ KIND ‖ container_version ‖ header(明文 CBOR) ‖ payload(= AEAD(zstd(明文)))`。金鑰是拿 passphrase 過一遍 Argon2id 派生出來的。想看逐位元、不留模糊空間的權威定義，翻 [`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md) 和 [`docs/fcb-reference.md`](./docs/fcb-reference.md)。

## Repo 結構

| 路徑 | 內容 |
| ---- | ---- |
| `crates/fcb` | codec 本體：container、crypto、compression、evidence/stream 模型、task、submission、`case`（`pack_case`）。 |
| `crates/fcb-wasm` | WASM / JS bridge，也就是瀏覽器 workbench 的進出口（`peekHeader`、`openCase`、`packSubmission` 等）。 |
| `docs/` | 協定的權威文件：[`fcb-wire-format`](./docs/fcb-wire-format.md)、[`fcb-data-model`](./docs/fcb-data-model.md)、[`fcb-reference`](./docs/fcb-reference.md)、[`docs/README`](./docs/README.md)。 |
| `openspec/` | Spectra 規格（`specs/`）與變更提案（`changes/`）。 |

## Quickstart

### Rust（相依本 crate）

```toml
# Cargo.toml
[dependencies]
fcb = { git = "https://github.com/triage-lab/fcb-rs", package = "fcb" }
```

下面這段一次走完兩端：producer 打包一份 `.case`，consumer 再把它開封。

```rust
use ciborium::value::Value;
use fcb::case::{pack_case, CaseInput, CasePayload};
use fcb::evidence::{StreamData, StreamManifest};

// 1) manifest 宣告每條 stream 的 id / type / 筆數；payload 帶記錄。
//    這裡用一個第三方範例 type；內建 type（fcb.syslog.v1 等）的記錄 schema 見 docs/fcb-data-model.md §3。
let manifest = vec![StreamManifest {
    id: "s0".into(), stream_type: "example.note.v1".into(), records: 1,
}];
let payload = CasePayload {
    streams: vec![StreamData {
        id: "s0".into(),
        records: vec![Value::Text("an event".into())],
    }],
};
// 2) pack_case：自動算 canonical bundle_hash、組 meta、封裝。
let input = CaseInput { case_id: "demo".into(), manifest, task: None, payload };
let bytes = pack_case(&input, "passphrase").unwrap();

// 3) 開封。
let (kind, header, _payload) = fcb::bundle::open_bytes(&bytes, "passphrase").unwrap();
assert_eq!(kind, fcb::container::BundleKind::Case);
assert_eq!(header.case_id, "demo");
```

想要更完整、而且真的會過測試的範例，去看 [`docs/README.md`](./docs/README.md) 和 `crates/fcb/tests/`。

### WASM / JS（瀏覽器或 Node）

```bash
wasm-pack build crates/fcb-wasm --target web      # 或 --target nodejs / bundler
```

build 完的 `pkg/` 會把這幾支匯出去：`peekHeader(bytes)`、`openCase(bytes, passphrase)`、`openSubmission(bytes, passphrase)`、`packSubmission(submission, passphrase)`、`packCase(caseObject, passphrase)`、`computeBundleHash(bytes)`、`verifyBinding(...)` 還有 `workKey(caseId)`。出錯時它不會只丟個含糊的錯誤，而是回一個你認得出來的 error kind——`bad-magic`、`wrong-passphrase`、`corrupt` 之類的。整合上的細節都在 [`crates/fcb-wasm/src/lib.rs`](./crates/fcb-wasm/src/lib.rs)。

## Build / Test

```bash
cargo build --workspace
cargo test --workspace      # 含 golden vectors 與 round-trip 套件
cargo clippy --all-targets --all-features -- -D warnings
wasm-pack build crates/fcb-wasm --target nodejs
```

CI 設定在 [`.github/workflows/ci.yml`](./.github/workflows/ci.yml)。每次 push 進 `main`、每個 PR，它都會跑一遍 `cargo test --workspace`，再加一道 `wasm32` build smoke 把關。

## 文件入口

- 第一次串接、想從消費端起步：**整合指南** [`docs/fcb-integration-guide.md`](./docs/fcb-integration-guide.md)
- 手邊有具體任務、想照抄 recipe：**Cookbook** [`docs/fcb-cookbook.md`](./docs/fcb-cookbook.md)
- 協定的 **wire format**：[`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md)
- **資料模型與 stream schema**：[`docs/fcb-data-model.md`](./docs/fcb-data-model.md)
- 要對到逐位元、golden vectors、error 目錄：**reference** [`docs/fcb-reference.md`](./docs/fcb-reference.md)
- 想先看總覽、跑跑可執行範例：[`docs/README.md`](./docs/README.md)
- 歷次改了什麼：**變更歷史** [`CHANGELOG.md`](./CHANGELOG.md)

## 貢獻與安全

- 想送 PR，先看 [`CONTRIBUTING.md`](./CONTRIBUTING.md) 把流程和品質關卡摸熟；和大家相處的那一套規矩寫在 [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md)。
- 發現安全漏洞，請走 [`SECURITY.md`](./SECURITY.md) 列的**私密**管道回報，千萬別開成 public issue。

## 授權

本專案採 **Educational Community License, Version 2.0（ECL-2.0）**，完整條款在 [`LICENSE`](./LICENSE)。© 2026 The fcb-rs Authors。
