# fcb-rs

[![ci](https://github.com/triage-lab/fcb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/triage-lab/fcb-rs/actions/workflows/ci.yml)
[![license: ECL-2.0](https://img.shields.io/badge/license-ECL--2.0-blue.svg)](./LICENSE)

**FCB（Forensic Case Bundle）協定的權威 Rust 實作**——一個把證物（evidence）封裝成可攜、加密、可驗證的 `.case` / `.casework` 檔的 codec。原為 browser-arena 數位鑑識教學平台的一部分，現抽出為獨立 repo。

一份 codec、兩個編譯目標：**native**（出題 CLI、教師審閱平台）與 **WASM**（瀏覽器 workbench）。核心相依全為純 Rust（`ruzstd` / `argon2` / `chacha20poly1305` / `sha2`），無 C FFI，因此能編到 `wasm32`。

## FCB 是什麼？

- **`.case`**（KIND=case）：教師端出的題目，裝著 N 條具型別的證物 stream（syslog / netflow / json …）與一份**不含答案**的 task spec。
- **`.casework`**（KIND=work）：學生端的作答，裝著筆記、報告、活動紀錄，並以 `case_id` + `bundle_hash` 綁回特定題目與證物版本。

容器格式：`magic ‖ KIND ‖ container_version ‖ header(明文 CBOR) ‖ payload(= AEAD(zstd(明文)))`，以 passphrase 經 Argon2id 派生金鑰。逐位元的權威定義見 [`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md) 與 [`docs/fcb-reference.md`](./docs/fcb-reference.md)。

## Repo 結構

| 路徑 | 內容 |
| ---- | ---- |
| `crates/fcb` | codec 本體：container、crypto、compression、evidence/stream 模型、task、submission、`case`（`pack_case`）。 |
| `crates/fcb-wasm` | WASM / JS bridge——瀏覽器 workbench 的進出點（`peekHeader` / `openCase` / `packSubmission` …）。 |
| `docs/` | 協定的權威文件：[`fcb-wire-format`](./docs/fcb-wire-format.md)、[`fcb-data-model`](./docs/fcb-data-model.md)、[`fcb-reference`](./docs/fcb-reference.md)、[`docs/README`](./docs/README.md)。 |
| `openspec/` | Spectra 規格（`specs/`）與變更提案（`changes/`）。 |

## Quickstart

### Rust（相依本 crate）

```toml
# Cargo.toml
[dependencies]
fcb = { git = "https://github.com/triage-lab/fcb-rs", package = "fcb" }
```

打包一份 `.case`（producer）與開封（consumer）：

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

完整、會過測試的範例見 [`docs/README.md`](./docs/README.md) 與 `crates/fcb/tests/`。

### WASM / JS（瀏覽器或 Node）

```bash
wasm-pack build crates/fcb-wasm --target web      # 或 --target nodejs / bundler
```

產出的 `pkg/` 匯出 `peekHeader(bytes)`、`openCase(bytes, passphrase)`、`openSubmission(bytes, passphrase)`、`packSubmission(submission, passphrase)`、`computeBundleHash(bytes)`、`verifyBinding(...)`、`workKey(caseId)`，並以可辨識的 error kind（`bad-magic` / `wrong-passphrase` / `corrupt` …）回報。整合細節見 [`crates/fcb-wasm/src/lib.rs`](./crates/fcb-wasm/src/lib.rs)。

## Build / Test

```bash
cargo build --workspace
cargo test --workspace      # 含 golden vectors 與 round-trip 套件
cargo clippy --all-targets --all-features -- -D warnings
wasm-pack build crates/fcb-wasm --target nodejs
```

CI（[`.github/workflows/ci.yml`](./.github/workflows/ci.yml)）會在每次 push 到 `main` 與每個 PR 跑 `cargo test --workspace` 與 `wasm32` build smoke。

## 文件入口

- **協定 wire format**：[`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md)
- **資料模型 / stream schema**：[`docs/fcb-data-model.md`](./docs/fcb-data-model.md)
- **逐位元 reference / golden vectors / error 目錄**：[`docs/fcb-reference.md`](./docs/fcb-reference.md)
- **總覽與可跑範例**：[`docs/README.md`](./docs/README.md)

## 貢獻與安全

- 貢獻流程與品質關卡見 [`CONTRIBUTING.md`](./CONTRIBUTING.md)；社群準則見 [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md)。
- 安全漏洞請依 [`SECURITY.md`](./SECURITY.md) 的**私密**管道回報，勿開 public issue。

## 授權

採 **Educational Community License, Version 2.0（ECL-2.0）**，見 [`LICENSE`](./LICENSE)。© 2026 The fcb-rs Authors。
