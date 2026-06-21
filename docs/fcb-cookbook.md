# FCB Cookbook（常見任務 recipes）

任務導向的速查。每則 recipe 只給你兩樣東西：**目標**，以及**會用到哪些呼叫**。想看完整脈絡就翻 [`fcb-integration-guide.md`](./fcb-integration-guide.md)，想要逐位元的權威說法則查 [`fcb-reference.md`](./fcb-reference.md)、[`fcb-data-model.md`](./fcb-data-model.md)、[`fcb-wire-format.md`](./fcb-wire-format.md)。

> 動手前先記住一件事（整合指南「安全須知」講得更細）：明文 header 沒經過 AEAD 認證，所以**收件端一定要驗 binding**，而且能信的只有**解密後 payload** 裡的值。

---

## Recipe 1：收件——開封並驗 submission binding

**目標**：平台收到學生交來的 `.casework`，要先確認它對得上正確的題目和證物版本，才能入帳。

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

**目標**：教師改完證物後重發 `.case`，`bundle_hash` 也跟著變。這時要能抓出哪些學生還拿著舊版作答。

`bundle_hash` 其實是證物版本的內容位址。重發之後，新 `.case` 的 `header.bundle_hash` 和舊的就不一樣了。學生作答帶的若是舊 `bundle_hash`，`verify_binding` 會回 `EvidenceVersionMismatch`——`case_id` 相同、`bundle_hash` 不同。處理方式跟 Recipe 1 一樣，把這個狀態當成「需重做」的提示就好。

涉及：`binding::verify_binding` 的三態。延伸：[`fcb-data-model.md`](./fcb-data-model.md) §5（binding）。

---

## Recipe 3：解碼某個 stream type 的記錄

**目標**：開封 `.case` 之後，把某條 stream 的記錄撈出來，順便看看它有沒有內建 handler。

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

`is_builtin == false` 就表示沒有對應的內建 handler，這時消費端改走 generic table/timeline fallback。涉及：`evidence::{decode_streams, manifest_from_meta}`、`case::CasePayload`。延伸：[`fcb-data-model.md`](./fcb-data-model.md) §3（各 stream type schema）。

---

## Recipe 4：用 golden vector 驗跨實作相容性

**目標**：你用非 Rust 重寫了一份 codec，想確認它跟本實作逐位元相容。

先把 `crates/fcb/tests/vectors.rs` 裡的 `FROZEN_*_HEX`（`FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` / `FROZEN_SUBMISSION_HEX`）`hex::decode` 出來。接著有兩條路：一是用你的實作搭同一組固定 salt/nonce 與密碼（`"lab-pass"`）重建一份，再**逐位元比對**；二是反向解密回來，比對結構對不對得上。至於 `FROZEN_CASE_BUNDLE_HASH` / `FROZEN_CASE_PAYLOAD_HEX`，是拿來驗 canonical 序列化與雜湊的。最容易踩到的互通陷阱——ciborium 會把 `Vec<u8>` 編成 CBOR array-of-uint——在 [`fcb-reference.md`](./fcb-reference.md) 講得很清楚。

涉及：`crates/fcb/tests/vectors.rs` 的 frozen 向量。延伸：整合指南 §4。

---

## Recipe 5：分辨「密碼錯」與「檔案被竄改」

**目標**：開封失敗時，告訴使用者該怎麼辦——是重輸密碼，還是乾脆拒收這個檔案。

codec 靠明文 header 裡的 key-check value 把這兩種情況分開。`WrongPassphrase` 是 KCV 不符，代表密碼打錯；`Corrupt` 則是 KCV 對得上、但 AEAD 驗證沒過，表示內容被竄改或毀損了。

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

在 WASM/JS 這端，改看 bridge 的 `error_kind` 字串（`wrong-passphrase` / `corrupt` / …）來分流，範例在整合指南 §2.3。涉及：`FcbError`、bridge `error_kind`。延伸：整合指南 §3、[`fcb-reference.md`](./fcb-reference.md)（error 目錄）。

---

## Recipe 6：不需密碼先顯示題目資訊

**目標**：使用者還沒輸入密碼前，就先把這是哪個 case、有哪些 stream、題目敘述等資訊顯示出來。

```rust
use fcb::container::peek_header;
use fcb::evidence::manifest_from_meta;
use fcb::task::task_from_meta;

let header = peek_header(&bytes)?;              // 不需 passphrase
let manifest = manifest_from_meta(&header.meta)?;   // .case 才有
let task = task_from_meta(&header.meta)?;            // Option<TaskSpec>
```

WASM/JS 這邊呼叫 `peekHeader(bytes)` 就行。涉及：`container::peek_header`、`evidence::manifest_from_meta`、`task::task_from_meta`。延伸：整合指南 §1.2、§2.2。
