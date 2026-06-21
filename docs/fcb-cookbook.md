# FCB Cookbook（常見任務 recipes）

任務導向的速查：每則 recipe 給**目標**與**涉及的呼叫**。完整脈絡看 [`fcb-integration-guide.md`](./fcb-integration-guide.md)；逐位元權威看 [`fcb-reference.md`](./fcb-reference.md)、[`fcb-data-model.md`](./fcb-data-model.md)、[`fcb-wire-format.md`](./fcb-wire-format.md)。

> 安全前提（見整合指南「安全須知」）：明文 header 未經 AEAD 認證，**收件端必須驗 binding**，且要信任的是**解密後 payload** 的值。

---

## Recipe 1：收件——開封並驗 submission binding

**目標**：平台收到學生 `.casework`，確認它對應正確的題目與證物版本後才入帳。

```rust
use fcb::submission::open_submission;
use fcb::binding::{verify_binding, BindingCheck};

let work = open_submission(&work_bytes, passphrase)?;   // KIND-gated：拿 .case 餵會被拒
match verify_binding(&work.case_id, &work.bundle_hash, &case_id, &case_bundle_hash) {
    BindingCheck::Match => { /* 收件 */ }
    BindingCheck::CaseMismatch => { /* 退件：不同題目 */ }
    BindingCheck::EvidenceVersionMismatch => { /* 退件：舊版證物，給理由 */ }
}
```

涉及：`submission::open_submission`、`binding::verify_binding`。延伸：整合指南 §1.5–§1.6、§5。

---

## Recipe 2：重發 case 後偵測「版本不符」

**目標**：教師修正證物後重發 `.case`（`bundle_hash` 改變）；學生若拿舊版作答，要能偵測。

`bundle_hash` 是證物版本的內容位址。重發後新 `.case` 的 `header.bundle_hash` 與舊的不同；學生作答帶的是舊 `bundle_hash`，於是 `verify_binding` 回 `EvidenceVersionMismatch`（同 `case_id`、不同 `bundle_hash`）。處理同 Recipe 1，把該狀態當「需重做」提示。

涉及：`binding::verify_binding` 的三態。延伸：[`fcb-data-model.md`](./fcb-data-model.md) §5（binding）。

---

## Recipe 3：解碼某個 stream type 的記錄

**目標**：開封 `.case` 後，取出某條 stream 的記錄並判斷有沒有內建 handler。

```rust
use fcb::bundle::open_bytes;
use fcb::case::CasePayload;
use fcb::evidence::{decode_streams, manifest_from_meta};

let (_kind, header, payload) = open_bytes(&case_bytes, passphrase)?;
let manifest = manifest_from_meta(&header.meta)?;
let case: CasePayload = fcb::cbor::decode(&payload)?;
let streams = decode_streams(&manifest, &case.streams)?;

for s in streams.iter().filter(|s| s.stream_type == "fcb.netflow.v1") {
    // s.is_builtin == true（內建型別）；s.records 是 Vec<ciborium Value>
    for rec in &s.records { /* 依 fcb.netflow.v1 schema 讀欄位 */ }
}
```

`is_builtin == false` 代表沒有內建 handler，消費端走 generic table/timeline fallback。涉及：`evidence::{decode_streams, manifest_from_meta}`、`case::CasePayload`。延伸：[`fcb-data-model.md`](./fcb-data-model.md) §3（各 stream type schema）。

---

## Recipe 4：用 golden vector 驗跨實作相容性

**目標**：用非 Rust 重寫 codec，想確認與本實作逐位元相容。

拿 `crates/fcb/tests/vectors.rs` 的 `FROZEN_*_HEX`（`FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` / `FROZEN_SUBMISSION_HEX`）`hex::decode` 後，用你的實作以同一組固定 salt/nonce 與密碼（`"lab-pass"`）重建，**逐位元比對**；或反向解密後比對結構。`FROZEN_CASE_BUNDLE_HASH` / `FROZEN_CASE_PAYLOAD_HEX` 則驗 canonical 序列化與雜湊。最關鍵的互通陷阱（ciborium 把 `Vec<u8>` 編成 CBOR array-of-uint）見 [`fcb-reference.md`](./fcb-reference.md)。

涉及：`crates/fcb/tests/vectors.rs` 的 frozen 向量。延伸：整合指南 §4。

---

## Recipe 5：分辨「密碼錯」與「檔案被竄改」

**目標**：開封失敗時，給使用者正確的下一步——重輸密碼，還是拒收檔案。

codec 用明文 header 裡的 key-check value 把兩者分開：`WrongPassphrase`（KCV 不符，密碼錯）vs `Corrupt`（KCV 對但 AEAD 驗證失敗，內容被竄改/毀損）。

```rust
use fcb::FcbError;
match fcb::bundle::open_bytes(&bytes, passphrase) {
    Ok(v) => { /* … */ }
    Err(FcbError::WrongPassphrase) => { /* 請使用者重輸密碼 */ }
    Err(FcbError::Corrupt)         => { /* 拒收：檔案不可信 */ }
    Err(FcbError::BadMagic)        => { /* 不是 FCB 檔 */ }
    Err(e) => { /* Malformed / UnsupportedVersion */ }
}
```

WASM/JS 端用 bridge 的 `error_kind` 字串（`wrong-passphrase` / `corrupt` / …），分流範例見整合指南 §2.3。涉及：`FcbError`、bridge `error_kind`。延伸：整合指南 §3、[`fcb-reference.md`](./fcb-reference.md)（error 目錄）。

---

## Recipe 6：不需密碼先顯示題目資訊

**目標**：在使用者輸入密碼前，先顯示這是哪個 case、有哪些 stream、題目敘述。

```rust
use fcb::container::peek_header;
use fcb::evidence::manifest_from_meta;
use fcb::task::task_from_meta;

let header = peek_header(&bytes)?;              // 不需 passphrase
let manifest = manifest_from_meta(&header.meta)?;   // .case 才有
let task = task_from_meta(&header.meta)?;            // Option<TaskSpec>
```

WASM/JS 用 `peekHeader(bytes)`。涉及：`container::peek_header`、`evidence::manifest_from_meta`、`task::task_from_meta`。延伸：整合指南 §1.2、§2.2。
