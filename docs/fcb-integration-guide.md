# FCB 整合指南（Integration Guide）

從**消費端**角度把 `fcb-rs` 接起來：開封 `.case`、產出 `.casework`、在瀏覽器接 WASM bridge。協定的逐位元細節不在這裡重述——需要時請看權威文件：

- 容器與封裝：[`fcb-wire-format.md`](./fcb-wire-format.md)
- 資料模型與 stream schema：[`fcb-data-model.md`](./fcb-data-model.md)
- 逐位元 reference / golden vectors / error 目錄：[`fcb-reference.md`](./fcb-reference.md)
- 總覽與最小範例：[`README.md`](./README.md)

兩個角色檔案：**`.case`**（KIND=case，教師出的題目＋證物）與 **`.casework`**（KIND=work，學生作答）。`.casework` 以 `case_id` + `bundle_hash` 綁回特定題目與證物版本。

> ### ⚠️ 安全須知（接 codec 前先讀）
>
> - **明文 header 未被 AEAD 認證。** container 的 header（含 `case_id`、`bundle_hash`、manifest、task）是**明文、未簽章**——只有 payload 經 AEAD 驗證。任何能產檔的人都能在 header 填任意值。因此**收件端的 binding 檢查是必要的、不是選用的**（§1.6 / §5）；要信任的是「**解密後 payload** 裡的值」與「對 payload 重算的 canonical `bundle_hash`」，不是 header 上的字串。
> - **每次封裝都產新的隨機 salt/nonce。** `pack_case` / `pack_submission` 內部各自產生 fresh salt（16 B）與 nonce（24 B）。**不要跨 bundle 快取或重用 key／nonce**——這份隨機性就是安全邊界。
> - **passphrase 是使用者輸入的字串。** 任意 UTF-8 皆可，codec **不強制**長度或強度（由 Argon2id 拖慢暴力破解，但弱密碼仍可被猜）。請在 UI 引導使用者選足夠熵的密碼。`.case` 與 `.casework` 的密碼**彼此獨立**。

---

## 1. Rust 路徑

### 1.1 相依

`fcb-rs` 尚未上 crates.io，用 Cargo git dependency：

```toml
# Cargo.toml
[dependencies]
fcb = { git = "https://github.com/triage-lab/fcb-rs", package = "fcb" }
ciborium = "0.2"   # 操作 CBOR Value 時需要
```

### 1.2 不需密碼先看 header（peek）

`peek_header` 只讀明文 header，**不需 passphrase**——適合在解鎖前先顯示是哪個 case、哪個證物版本、有哪些 stream：

```rust
use fcb::container::{peek_header, BundleKind};
use fcb::evidence::manifest_from_meta;
use fcb::task::task_from_meta;

let header = peek_header(&bytes)?;            // bytes: &[u8]，整個 .case/.casework
println!("case_id = {}", header.case_id);
println!("bundle_hash = {}", header.bundle_hash);

// manifest 與 task 都在明文 meta 裡（.case 才有；.casework 的 meta 是空 map）。
let manifest = manifest_from_meta(&header.meta)?;   // Vec<StreamManifest>
let task = task_from_meta(&header.meta)?;           // Option<TaskSpec>
```

### 1.3 開封 `.case`（解密 + 解碼 streams）

```rust
use fcb::bundle::open_bytes;
use fcb::case::CasePayload;
use fcb::container::BundleKind;
use fcb::evidence::{decode_streams, manifest_from_meta};

let (kind, header, payload) = open_bytes(&bytes, "passphrase")?;
assert_eq!(kind, BundleKind::Case);

let manifest = manifest_from_meta(&header.meta)?;
let case: CasePayload = fcb::cbor::decode(&payload)?;        // { streams: [StreamData] }
let streams = decode_streams(&manifest, &case.streams)?;    // Vec<DecodedStream>

for s in &streams {
    // is_builtin == false 代表沒有內建 handler，消費端走 generic table/timeline fallback。
    println!("{} ({}) records={} builtin={}", s.id, s.stream_type, s.records.len(), s.is_builtin);
}
```

### 1.4 產出 `.case`（`pack_case`）

`case::pack_case` 是 `.case` 的權威產出 helper：自動以 canonical 序列化算 `bundle_hash`、組 `{streams, task?}` meta、產生隨機 salt/nonce、用預設 Argon2id cost 封裝。

```rust
use ciborium::value::Value;
use fcb::case::{pack_case, CaseInput, CasePayload};
use fcb::evidence::{StreamData, StreamManifest};
use fcb::task::{ReportMode, TaskSpec, TaskStep};

let manifest = vec![StreamManifest {
    id: "s0".into(), stream_type: "fcb.syslog.v1".into(), records: 1,
}];
let payload = CasePayload {
    streams: vec![StreamData {
        id: "s0".into(),
        records: vec![Value::Map(vec![
            (Value::Text("ts".into()),   Value::Text("2026-01-01T00:00:00Z".into())),
            (Value::Text("host".into()), Value::Text("h1".into())),
            (Value::Text("msg".into()),  Value::Text("hello".into())),
        ])],
    }],
};
let task = TaskSpec {
    report_mode: ReportMode::Steps,
    instructions: "Investigate the host.".into(),
    steps: vec![TaskStep { id: "q1".into(), prompt: "source IP?".into(), answer_type: "ip".into() }],
};

let input = CaseInput { case_id: "demo-2026-01".into(), manifest, task: Some(task), payload };
let case_bytes = pack_case(&input, "passphrase")?;   // 寫成 .case 檔
```

> stream 記錄要符合對應 type 的 schema（syslog / netflow / json 見 data-model §3）。`pack_case` 不驗證記錄形狀——schema 由生產端負責。

### 1.5 產出 / 開封 `.casework`（`Submission`）

```rust
use ciborium::value::Value;
use fcb::submission::{pack_submission, open_submission, Student, Submission};

let work = Submission {
    case_id: "demo-2026-01".into(),
    bundle_hash: "sha256:…".into(),   // 來自題目 .case 的 header.bundle_hash
    student: Student { id: "s1234567".into(), name: "Lin".into() },
    notes: vec![Value::Text("pinned auth.log line 42".into())],
    report: Value::Text("freeform report body".into()),
    activity: vec![Value::Text("search: failed login".into())],
    exported_at: "2026-06-20T10:00:00Z".into(),
};
let work_bytes = pack_submission(&work, "student-pass")?;

// 收件端開封（KIND-gated：拿 .case 餵會被拒）。
let back = open_submission(&work_bytes, "student-pass")?;
assert_eq!(back, work);
```

### 1.6 驗 binding（題目 ↔ 作答）

收件平台**必須**確認學生作答對應的是同一個 case 與同一份證物版本（header 未認證，這步是安全關鍵，見上方「安全須知」）：

```rust
use fcb::binding::{verify_binding, BindingCheck};

match verify_binding(&work.case_id, &work.bundle_hash, &case_id, &case_bundle_hash) {
    BindingCheck::Match => { /* 同 case、同證物版本，收 */ }
    BindingCheck::CaseMismatch => { /* 不同題目，退件 */ }
    BindingCheck::EvidenceVersionMismatch => { /* 同題目但證物被重發過，提示版本不符 */ }
}
```

---

## 2. WASM / JS 路徑

### 2.1 build

```bash
wasm-pack build crates/fcb-wasm --target web       # 瀏覽器 ESM
# 或 --target nodejs / --target bundler
```

產出在 `crates/fcb-wasm/pkg/`，含 `.wasm`、JS glue、`.d.ts`。

### 2.2 bridge 函式

`pkg/` 匯出（名稱對齊 `crates/fcb-wasm/src/lib.rs` 的 `#[wasm_bindgen(js_name = …)]`）：

| 函式 | 作用 |
| ---- | ---- |
| `peekHeader(bytes)` | 不需密碼讀 header（kind、versions、case_id、bundle_hash、manifest、task）。 |
| `openCase(bytes, passphrase)` | 解密 `.case`、回 `{ case_id, bundle_hash, task, streams }`。 |
| `openSubmission(bytes, passphrase)` | 解密 `.casework`、回 `Submission`。 |
| `packSubmission(submission, passphrase)` | 把 `Submission` 封成 `.casework` bytes。 |
| `computeBundleHash(bytes)` | `sha256:<hex>` 內容雜湊。 |
| `verifyBinding(workCaseId, workBundleHash, caseId, caseBundleHash)` | 回 `"match"` / `"case-mismatch"` / `"evidence-version-mismatch"`。 |
| `workKey(caseId)` | 本地儲存分割鍵 `fcb:work:<caseId>`。 |

### 2.3 薄 adapter（對齊 bridge 風格）

把 bridge 的「丟出帶 `kind` 的 JS error」收斂成好處理的形狀：

```js
import init, * as fcb from "../crates/fcb-wasm/pkg/fcb_wasm.js";

await init();   // 載入 .wasm（--target web）

export async function openCase(bytes, passphrase) {
  try {
    return { ok: true, value: fcb.openCase(bytes, passphrase) };
  } catch (e) {
    // bridge 把 FcbError 映成帶穩定 kind 的 error（見 §3）。
    return { ok: false, kind: e.kind ?? "unknown", message: String(e.message ?? e) };
  }
}

export function peek(bytes) {
  return fcb.peekHeader(bytes);   // 不需密碼，失敗才丟（bad-magic / malformed）
}
```

依 error kind 分流成可給使用者的訊息（注意：`wrong-passphrase` 要重輸密碼，`corrupt` 要拒收，兩者**不可混為一談**）：

```js
export function openCaseUx(bytes, passphrase) {
  try {
    return { ok: true, case: fcb.openCase(bytes, passphrase) };
  } catch (e) {
    switch (e?.kind) {
      case "wrong-passphrase":     return { ok: false, retry: true,  msg: "密碼錯誤，請重新輸入。" };
      case "bad-magic":
      case "malformed":            return { ok: false, retry: false, msg: "這不是有效的 FCB 檔。" };
      case "corrupt":              return { ok: false, retry: false, msg: "檔案已毀損或被竄改，拒絕開啟。" };
      case "unsupported-version":  return { ok: false, retry: false, msg: "此檔需要較新版本的 reader。" };
      default:                     return { ok: false, retry: false, msg: String(e?.message ?? e) };
    }
  }
}
```

---

## 3. Error kind 處理

codec 的 `FcbError` 有五種變體；bridge 以穩定字串（`error_kind`）回報，JS 端據此分流：

| `FcbError` | bridge kind | 意義 / 處理 |
| ---------- | ----------- | ----------- |
| `BadMagic` | `bad-magic` | 不是 FCB 檔（magic 不符）。當作「選錯檔」處理。 |
| `UnsupportedVersion { … }` | `unsupported-version` | bundle 要求的 reader 版本比目前高。提示升級。 |
| `Malformed(_)` | `malformed` | 結構壞了 / 不該出現的內容（如把 `.case` 當 `.casework` 開）。退件。 |
| `WrongPassphrase` | `wrong-passphrase` | 密碼錯（由 key-check value 區分，**先於**解密判定）。**請使用者重輸入密碼**。 |
| `Corrupt` | `corrupt` | 密碼對、但 AEAD 驗證失敗 → 內容被竄改或毀損。**拒收該檔**，不要當密碼錯。 |

關鍵在於 **`wrong-passphrase` 與 `corrupt` 是兩件事**：前者請使用者重輸密碼，後者代表檔案不可信。codec 用 plaintext header 裡的 key-check value 把兩者分開（細節見 [`fcb-wire-format.md`](./fcb-wire-format.md) §4）。

---

## 4. Golden-vector 契約

`crates/fcb/tests/vectors.rs` 內的 **frozen 向量**是跨實作相容性的權威基準。用固定 salt/nonce 建出、逐位元釘住：

| 向量 | 釘住什麼 |
| ---- | -------- |
| `FROZEN_CASE_HEX` | 一個 `.case`（2 streams + task）的完整 sealed bytes。 |
| `FROZEN_WORK_HEX` | 一個 `.casework`（test-local 3 欄 `WorkPayload`，歷史向量）。 |
| `FROZEN_SUBMISSION_HEX` | 真實 7 欄 `Submission` 的 `.casework` sealed bytes。 |
| `FROZEN_CASE_BUNDLE_HASH` | 固定 streams 的 canonical `bundle_hash`。 |
| `FROZEN_CASE_PAYLOAD_HEX` | 固定 streams 的 canonical 明文 payload bytes。 |

**用 Rust 消費**時，只要相依 `fcb` crate、走 `bundle`/`case`/`submission` 的公開 API，就自動與這些向量一致——不必自己對齊位元組。

**用非 Rust 重寫 codec**時，把上述 hex `hex::decode` 後，用你的實作解密/解碼並**逐位元比對**，即為相容性檢查（最關鍵的互通陷阱見 [`fcb-reference.md`](./fcb-reference.md)：ciborium 把 `Vec<u8>` 編成 CBOR array-of-uint，不是 byte string）。

---

## 5. 端到端流程

```text
teacher                         student                       platform
───────                         ───────                       ────────
pack_case(streams, task)
   → .case 檔  ───────────────▶  peekHeader / openCase
                                 （讀 task、看證物）
                                 ...作答...
                                 pack_submission(work)
                                    → .casework 檔  ─────────▶  openSubmission
                                                               verifyBinding(work, case)
                                                                 → Match / *Mismatch
```

1. **出題**：教師以 `pack_case` 把證物 streams 與**不含答案**的 task 封成 `.case`，記下其 `header.bundle_hash`（證物版本）。
2. **作答**：學生用密碼 `openCase` 讀題與證物，產出 `Submission`（帶 `case_id` 與題目的 `bundle_hash`），`pack_submission` 封成 `.casework`。
3. **收件**：平台 `openSubmission` 後，以 `verify_binding` 確認——用的是**解密後** `Submission` 裡的 `case_id`/`bundle_hash`，不是明文 header：
   - `Match`：同 case、同證物版本，收件。
   - `CaseMismatch`：不同題目（檔案來自別處），退件。
   - `EvidenceVersionMismatch`：同題目但證物被重發過，學生作答的是**舊版**證物——退件並給明確理由（別靜默當 Match）。

各步驟的欄位語意、答案安全不變量（task 不含 answer/rubric/solution）與 binding 三態，詳見 [`fcb-data-model.md`](./fcb-data-model.md)。
