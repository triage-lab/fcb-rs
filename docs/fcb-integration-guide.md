# FCB 整合指南（Integration Guide）

這份指南站在**消費端**角度，帶你把 `fcb-rs` 接起來：開封 `.case`、產出 `.casework`、在瀏覽器接 WASM bridge。協定的逐位元細節這裡不重述，要查的時候請翻權威文件：

- 容器與封裝：[`fcb-wire-format.md`](./fcb-wire-format.md)
- 資料模型與 stream schema：[`fcb-data-model.md`](./fcb-data-model.md)
- 逐位元 reference / golden vectors / error 目錄：[`fcb-reference.md`](./fcb-reference.md)
- 總覽與最小範例：[`README.md`](./README.md)

這裡有兩種角色檔案。一是 **`.case`**（KIND=case），裝的是教師出的題目加上證物；二是 **`.casework`**（KIND=work），裝的是學生作答。`.casework` 靠 `case_id` 加 `bundle_hash` 綁回特定題目與證物版本。

> ### ⚠️ 安全須知（接 codec 前先讀）
>
> - **明文 header（含框架前綴）現已被 AEAD 認證。** container 的 header（含 `case_id`、`bundle_hash`、manifest、task）雖然在磁碟上仍是明文（salt/nonce/KDF 參數必須在推 key 前就讀得到），但整段「容器前綴」（magic、KIND、container_version、hdr_len、完整 header CBOR）會被綁進 XChaCha20-Poly1305 的 **additional authenticated data（AAD）**。任何對 header 任一 byte 的竄改都會破壞 AEAD tag，`open`／`openCase`／`openSubmission` 一律以 **Corrupt** 失敗——封檔後就無法在不知道密碼的情況下偷改 header 而不被發現。
> - **`.case` 開檔還會額外核對 `bundle_hash` 內容定址。** 對 `.case` 而言 `bundle_hash` 就是 canonical payload 的 SHA-256，所以 `openCase` 會對**解密後的 payload 重算** canonical `bundle_hash` 並與 header 比對，不符即 **Corrupt**——擋下「header 與 payload 不一致」的偽造檔。（`.casework` 的 header `bundle_hash` 是**綁定參照**、指向某個 `.case` 的證物版本，並非 submission payload 自身的 hash，故 `openSubmission` **不**重算它。）
> - **`.casework` 收件端的 binding 檢查仍是必要的，不是選用的。** AEAD 只保證 header「未被竄改」，不保證學生填進去的 `case_id`／`bundle_hash` 真的對得上你手上的那份 `.case`。收件時請以**解密後** `Submission` 裡的 `case_id` 與 `bundle_hash` 走 `verify_binding`（§1.6 / §5），而不是只看欄位長相。
> - **仍存在的設計性提醒（by design）：** `bundle_hash` 是對**明文 payload** 算的 content hash，對低熵 payload 會構成一個 **confirmation oracle**（攻擊者若能猜出 payload，便能用公開的 `bundle_hash` 驗證猜測）；且此 binding 對「**重新封裝**」敏感——只要 payload 的 canonical 位元組有任何差異，重算出的 `bundle_hash` 就會不同，無法以此判定「語意上等價」的兩份證物。
> - **每次封裝都產新的隨機 salt/nonce。** `pack_case` 與 `pack_submission` 內部會各自產出 fresh salt（16 B）與 nonce（24 B）。**千萬不要跨 bundle 快取或重用 key／nonce**，這份隨機性正是安全邊界所在。
> - **passphrase 是使用者輸入的字串。** 任意 UTF-8 都收，codec **不強制**長度或強度（Argon2id 會拖慢暴力破解，但弱密碼還是猜得出來）。請在 UI 引導使用者選一個熵夠高的密碼（互動解鎖建議 ≥128 bit 熵）。`.case` 與 `.casework` 的密碼**各自獨立**。
> - **密碼不會被自動清零。** API 收的是 `&str`，Rust **不會**在用完後幫你把密碼位元組從記憶體抹掉。長駐的生產工具（CLI／服務）建議拿 [`zeroize`](https://docs.rs/zeroize/) 之類的型別把輸入包起來，用完即清。

---

## 1. Rust 路徑

### 1.1 相依

`fcb-rs` 還沒上 crates.io，所以用 Cargo git dependency 拉：

```toml
# Cargo.toml
[dependencies]
fcb = { git = "https://github.com/triage-lab/fcb-rs", package = "fcb" }
ciborium = "0.2"   # 操作 CBOR Value 時需要
```

### 1.2 不需密碼先看 header（peek）

`peek_header` 只讀明文 header，**不需 passphrase**。很適合在解鎖前先告訴使用者這是哪個 case、哪個證物版本、有哪些 stream：

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

`case::pack_case` 是產 `.case` 的權威 helper，幾件事它都幫你包好了：用 canonical 序列化算出 `bundle_hash`、組好 `{streams, task?}` meta、產生隨機 salt/nonce，再以預設 Argon2id cost 封裝。

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

> stream 記錄得符合對應 type 的 schema（syslog / netflow / json 見 data-model §3）。`pack_case` 不會檢查記錄形狀，schema 是生產端自己的責任。

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

收件平台**必須**確認學生作答對應的就是同一個 case、同一份證物版本。因為 header 沒被認證，這一步是安全關鍵，前面「安全須知」也提過：

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

產出落在 `crates/fcb-wasm/pkg/`，裡面有 `.wasm`、JS glue 跟 `.d.ts`。

### 2.2 bridge 函式

`pkg/` 匯出（名稱對齊 `crates/fcb-wasm/src/lib.rs` 的 `#[wasm_bindgen(js_name = …)]`）：

| 函式 | 作用 |
| ---- | ---- |
| `peekHeader(bytes)` | 不需密碼讀 header（kind、versions、case_id、bundle_hash、manifest、task）。 |
| `openCase(bytes, passphrase)` | 解密 `.case`、回 `{ case_id, bundle_hash, task, streams }`。 |
| `openSubmission(bytes, passphrase)` | 解密 `.casework`、回 `Submission`。 |
| `packSubmission(submission, passphrase)` | 把 `Submission` 封成 `.casework` bytes。 |
| `packCase(caseObject, passphrase)` | 把一個 case 物件封成 sealed `.case` bundle bytes。 |
| `computeBundleHash(bytes)` | `sha256:<hex>` 內容雜湊。 |
| `verifyBinding(workCaseId, workBundleHash, caseId, caseBundleHash)` | 回 `"match"` / `"case-mismatch"` / `"evidence-version-mismatch"`。 |
| `workKey(caseId)` | 本地儲存分割鍵 `fcb:work:<caseId>`。 |

### 2.3 薄 adapter（對齊 bridge 風格）

bridge 失敗時會丟出帶 `kind` 的 JS error，這裡把它收斂成比較好處理的形狀：

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

export function packCase(caseObject, passphrase) {
  try {
    return { ok: true, value: fcb.packCase(caseObject, passphrase) };
  } catch (e) {
    // 空 manifest 之類的結構問題會回 kind "malformed"（見 §3）。
    return { ok: false, kind: e.kind ?? "unknown", message: String(e.message ?? e) };
  }
}
```

接著照 error kind 分流出能直接給使用者看的訊息。這裡要小心：`wrong-passphrase` 是請使用者重輸密碼，`corrupt` 是直接拒收，兩者**不能混為一談**：

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

### 2.4 用 `packCase` 在瀏覽器出題

`packCase` 把一個 JS case 物件封成 sealed `.case` bytes，底層走的就是 native `pack_case` 的同一條路（canonical `bundle_hash`、隨機 salt/nonce、預設 Argon2id cost），所以**只要物件形狀對，JS 出的 `.case` 其 canonical payload 與 `bundle_hash` 會和 native producer 完全一致**（sealed bytes 本身因每次隨機 salt/nonce 而不同，要驗證一致性請比對 `bundle_hash`，而不是逐位元比 sealed bytes）。物件形狀如下：

```js
const caseObject = {
  case_id: "acme-ir-2026-03",
  manifest: [
    { id: "s0", type: "fcb.syslog.v1", records: 1 },   // 鍵是 `type`，不是 `stream_type`
  ],
  task: {
    report_mode: "steps",                               // 小寫字串："steps" / "freeform"
    instructions: "Investigate the host.",
    steps: [{ id: "q1", prompt: "source IP?", answer_type: "ip" }],
  },
  payload: {
    streams: [{ id: "s0", records: [ /* 可被 CBOR 編碼的值 */ ] }],
  },
};

const caseBytes = fcb.packCase(caseObject, "passphrase");   // Uint8Array，可寫成 .case 檔
```

兩個 footgun：

- **Footgun 1：manifest 每筆用鍵 `type`，不是 `stream_type`。** wire 與 JS 物件裡這個欄位都叫 `type`（Rust 端才是 `StreamManifest.stream_type`）。寫成 `stream_type` 會被當成缺 `type`，反序列化就壞——這個會回 kind `malformed`。
- **Footgun 2：超過 JS safe-integer 範圍的整數 record 值必須用 `BigInt` 傳。** record 裡的整數若超過 `2^53-1`（`Number.MAX_SAFE_INTEGER`），用普通 JS number 傳會被編成 CBOR float，canonical 位元組因此與 native producer 不同。pack 邊界現在會**主動擋下**這種「整數值卻超出 safe range 的 plain number」，回 kind `malformed`（不再靜默退化成 float、也不會默默產出對不上的 `bundle_hash`）。超過範圍的整數**務必**以 `BigInt` 傳入（例如 `9007199254784000n`，serde-wasm-bindgen 會編成 CBOR 整數）；safe range 內的整數照常用 number 即可。

---

## 3. Error kind 處理

codec 的 `FcbError` 共有五種變體。bridge 會用穩定字串（`error_kind`）回報，JS 端再照這個字串分流：

| `FcbError` | bridge kind | 意義 / 處理 |
| ---------- | ----------- | ----------- |
| `BadMagic` | `bad-magic` | 根本不是 FCB 檔（magic 不符）。當成「選錯檔」處理。 |
| `UnsupportedVersion { … }` | `unsupported-version` | bundle 要求的 reader 版本比目前手上的高。提示升級。 |
| `Malformed(_)` | `malformed` | 結構壞了，或出現了不該有的內容（例如把 `.case` 當 `.casework` 開）。退件。 |
| `WrongPassphrase` | `wrong-passphrase` | 密碼錯（靠 key-check value 區分，**先於**解密就判得出來）。**請使用者重新輸入密碼**。 |
| `Corrupt` | `corrupt` | 密碼對，但 AEAD 驗證沒過，代表內容被竄改或毀損。**拒收這個檔**，別當成密碼錯。 |

重點是要記住 **`wrong-passphrase` 跟 `corrupt` 是兩回事**。前者請使用者重輸密碼就好，後者則代表這個檔案已經不可信。codec 靠 plaintext header 裡的 key-check value 把這兩者分開（細節見 [`fcb-wire-format.md`](./fcb-wire-format.md) §4）。

---

## 4. Golden-vector 契約

`crates/fcb/tests/vectors.rs` 裡的 **frozen 向量**是跨實作相容性的權威基準。它們用固定 salt/nonce 建出，逐位元釘死：

| 向量 | 釘住什麼 |
| ---- | -------- |
| `FROZEN_CASE_HEX` | 一個 `.case`（2 streams + task）的完整 sealed bytes。 |
| `FROZEN_WORK_HEX` | 一個 `.casework`（test-local 3 欄 `WorkPayload`，屬於歷史向量）。 |
| `FROZEN_SUBMISSION_HEX` | 真實 7 欄 `Submission` 的 `.casework` sealed bytes。 |
| `FROZEN_CASE_BUNDLE_HASH` | 固定那組 streams 的 canonical `bundle_hash`。 |
| `FROZEN_CASE_PAYLOAD_HEX` | 固定那組 streams 的 canonical 明文 payload bytes。 |

**用 Rust 消費**的話最省事：只要相依 `fcb` crate、走 `bundle`/`case`/`submission` 的公開 API，產出就自動和這些向量一致，不必自己去對齊位元組。

**用非 Rust 重寫 codec**時，先把上述 hex 拿去 `hex::decode`，再用你的實作解密、解碼，然後**逐位元比對**，這就是相容性檢查。最容易踩到的互通陷阱見 [`fcb-reference.md`](./fcb-reference.md)：ciborium 會把 `Vec<u8>` 編成 CBOR array-of-uint，而不是 byte string。

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

1. **出題**：教師用 `pack_case` 把證物 streams 跟一份**不含答案**的 task 封成 `.case`，並記下它的 `header.bundle_hash`（也就是證物版本）。
2. **作答**：學生輸入密碼，用 `openCase` 讀題、看證物，作答後產出 `Submission`（帶上 `case_id` 與題目的 `bundle_hash`），再用 `pack_submission` 封成 `.casework`。
3. **收件**：平台 `openSubmission` 之後，用 `verify_binding` 確認。比對的依據是**解密後** `Submission` 裡的 `case_id` 跟 `bundle_hash`，不是明文 header：
   - `Match`：同 case、同證物版本，收件。
   - `CaseMismatch`：根本不是同一題（檔案來自別處），退件。
   - `EvidenceVersionMismatch`：題目對得上，但證物被重發過，學生作答的是**舊版**證物。這時要退件並講清楚理由，別靜默地當成 Match。

每一步的欄位語意、答案安全不變量（task 不含 answer/rubric/solution）以及 binding 三態，都詳見 [`fcb-data-model.md`](./fcb-data-model.md)。
