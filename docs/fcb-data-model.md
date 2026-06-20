# FCB 資料模型（data model）

本文件描述 FCB 的**語意層資料結構**——encoder 要塞進 `.case` 的東西、學生端寫進 `.casework`
的東西，以及兩者如何綁定。byte-level 信封與密碼學見 [`fcb-wire-format.md`](./fcb-wire-format.md)。

權威來源：`crates/fcb/src/{evidence,task,submission,binding}.rs` 與
`crates/fcb/tests/vectors.rs`（golden vectors）。

---

## 0. 資料放哪裡

| 資料 | 位置 | 不需 passphrase 可讀？ |
|------|------|:--:|
| `case_id`、`bundle_hash` | 明文 header | ✔ |
| stream **manifest**（每條 stream 的 id/type/數量） | 明文 header `meta.streams` | ✔ |
| **task spec**（題目敘述、步驟提示） | 明文 header `meta.task` | ✔ |
| stream **records**（實際證物事件） | 加密 payload | ✘（要解密） |
| 學生 notes / report / activity | 加密 payload（`.casework`） | ✘ |

設計意圖：解鎖前就能顯示「這是哪個 case、裡面有哪些 stream、題目要做什麼」；證物本體與學生作品才需密碼。

---

## 1. `.case` 的 `header.meta`

`.case` 的 `meta` 是一個 CBOR map，含兩把 key，彼此獨立解讀、且容忍額外 key
（`evidence::manifest_from_meta` 只讀 `streams`、`task::task_from_meta` 只讀 `task`）：

```text
meta = {
  "streams": [ StreamManifest, … ],   // evidence.rs
  "task":    TaskSpec                  // task.rs（可省略；省略則無題目）
}
```

> golden vector 用 `CaseMeta { streams, task }`（`tests/vectors.rs`）凍結這個結構。
>
> 「`task` 可省略」指的是 library 的 `task_to_meta` 路徑——它把 `task` 標了
> `skip_serializing_if = "Option::is_none"`，`None` 時整把 key 省略。但 golden vector 的
> `CaseMeta` 是普通 struct、**永遠**寫出 `task`。直接照抄 `CaseMeta` 當範本的人要注意這個差異。

### 1.1 `StreamManifest`（`evidence.rs`）

```text
StreamManifest = {
  "id":      text,    // bundle 內唯一；與 payload 的 StreamData.id 對應
  "type":    text,    // namespaced + versioned，如 "fcb.syslog.v1"、"acme.edr.v1"
  "records": u64      // 該 stream 的記錄數
}
```

> CBOR key 是 `"type"`（Rust 欄位名是 `stream_type`，標了 `#[serde(rename = "type")]`）。

### 1.2 `TaskSpec`（`task.rs`）

```text
TaskSpec = {
  "report_mode":  "steps" | "freeform",
  "instructions": text,            // 整體題目敘述
  "steps":        [ TaskStep, … ]  // 僅 steps 模式有意義；freeform 給空陣列
}

TaskStep = {
  "id":          text,
  "prompt":      text,
  "answer_type": text   // 如 "ip"、"text"、"hostname" —— 只描述「答案型別」，不含答案
}
```

`report_mode` 是小寫 enum text。`report_mode` 與 `instructions` **沒有** serde default，屬**必填**；
只有 `steps` 標 `#[serde(default)]`，缺省為空陣列。**`TaskStep` 沒有任何答案欄位**——見 §6 答案安全。

---

## 2. `.case` 的 payload 信封

明文 payload（壓縮／加密**之前**）是一個 CBOR map，只有一把 key `streams`：

```text
payload = {
  "streams": [ StreamData, … ]
}

StreamData = {
  "id":      text,        // 必須對應某個 StreamManifest.id
  "records": [ <記錄>, … ] // 每筆記錄是一個 CBOR value，形狀由該 stream 的 type 決定（見 §3）
}
```

消費端用 `evidence::decode_streams(manifest, payload.streams)` 以 `id` 把 manifest（型別）與
payload（記錄）對起來。manifest 有列、payload 卻找不到對應 `id` → `Malformed`。

> ⚠️ **已知缺口（見 §7）：** `fcb` crate **沒有**公開 helper 產生這個 `{ streams: [...] }` 信封；
> golden vector 是在 test 裡自定 `CasePayload { streams }`。encoder 要嘛照樣自組、要嘛在 crate
> 補 `evidence::pack_case`（建議）。

---

## 3. Stream type 的記錄 schema

每筆記錄的形狀**不**由 container 層規定（container 只看到 `Vec<Value>`）；由 stream 的 `type`
決定，消費端依 type 對應 parser/view。`BUILTIN_STREAM_TYPES = ["fcb.syslog.v1",
"fcb.netflow.v1", "fcb.json.v1"]`（`evidence.rs`）只表示「有內建 handler」，**不是封閉清單**：
任何 namespaced type 都是一等公民，未知 type 不致命。

> 目前 golden vector 的 syslog 記錄仍是 placeholder（`Value::Text("evt1")`）；正式 schema 已在
> `openspec/specs/fcb-stream-types/` 凍結，以下 §3.1 即依該 spec 撰寫。

### 3.1 `fcb.syslog.v1`

每筆記錄 = 一個 CBOR map。

**核心原則（最重要）：** `raw` 是**無損真相**——當 `raw` 存在時，它逐字保留原始整行、為該記錄的權威來源。
其他解析欄位皆為**盡力而為（best-effort）**的衍生值，**不得**作為事件的唯一表示；消費端必須能從 `raw`
重新解析回任一解析欄位，且只要保留 `raw`，原始行上的資訊就不會遺失。

| 欄位 | CBOR 型別 | 必填 | 說明 |
|------|-----------|:--:|------|
| `ts` | text（RFC 3339） | ✔ | 事件發生時間。正規化為 UTC、結尾 `Z`、毫秒精度，如 `2026-03-14T08:21:33.512Z`。代表 originator 回報的事件時間；時間排序與 lite 查詢 `time` range 以此為準。 |
| `host` | text | ✔ | 來源主機（hostname、FQDN 或 IP），照擷取到的樣子保留。 |
| `msg` | text | ✔ | 解析後、人類可讀的訊息本文（RFC 5424 MSG）。 |
| `raw` | text | | 擷取到的原始整行、**逐字保留**、未解析（見上方核心原則）。 |
| `app` | text | | 來源應用程式／程式名（RFC 5424 APP-NAME、RFC 3164 TAG 或等價物），如 `sshd`。 |
| `pid` | uint | | 來源行程 ID（PROCID）。 |
| `severity` | uint 0–7 | | syslog severity 數字碼（0 = Emergency、7 = Debug）。 |
| `facility` | uint 0–23 | | syslog facility 數字碼。 |
| `msgid` | text | | 訊息型別識別碼（RFC 5424 MSGID）。 |
| `sd` | map<SD-ID, map<param, value>> | | 結構化資料（RFC 5424 STRUCTURED-DATA），**依 SD-ID 分組**的巢狀 map：外層 key 是 SD-ID，內層是該元素的 param 名稱對字串值。 |
| `format` | text | | 來源 wire format，三選一：`rfc3164`／`rfc5424`／`other`。 |

**`severity`／`facility` 以數字為準**：只存數字碼，**不存名稱**；人類可讀的名稱由消費端從數字碼衍生
（對照：`0 Emergency`、`1 Alert`、`2 Critical`、`3 Error`、`4 Warning`、`5 Notice`、`6 Informational`、
`7 Debug`）。

**`ts` 一律正規化為 UTC**：當來源格式缺年份或時區（典型是 RFC 3164）時，由 encoder 推斷年份與時區以產出
UTC `ts`，而原始行仍逐字保存在 `raw`。

範例記錄（以 JSON 表示 CBOR）——RFC 5424 來源：

```json
{
  "ts": "2026-03-14T08:21:33.512Z",
  "host": "mymachine.example.com",
  "app": "su",
  "msgid": "ID47",
  "severity": 2,
  "facility": 4,
  "msg": "'su root' failed",
  "sd": { "ex@32473": { "iut": "3" } },
  "format": "rfc5424",
  "raw": "<34>1 2026-03-14T08:21:33.512Z mymachine.example.com su - ID47 [ex@32473 iut=\"3\"] 'su root' failed"
}
```

範例記錄（以 JSON 表示 CBOR）——RFC 3164 來源（缺年份／時區，由 encoder 推斷為 2026 年 UTC）：

```json
{
  "ts": "2026-10-11T22:14:15Z",
  "host": "mymachine",
  "app": "su",
  "severity": 2,
  "facility": 4,
  "msg": "'su root' failed for lonvick on /dev/pts/8",
  "format": "rfc3164",
  "raw": "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8"
}
```

**查詢／facet 對齊（重要）：** 前端 lite 查詢的 `field=value` 過濾與 facet 是針對**記錄的 top-level
欄位**（`EvidenceRecord = Record<string, unknown>`）。因此：
- `severity` / `facility` 為整數，可用 lite 的 `>` / `<` 數值比較（如 `severity<4` 找 Error 以上）。
- `sd` 是依 SD-ID 分組的巢狀 map，並非 top-level 欄位。若希望 `sd` 裡的 param（如 `iut`）可被
  `field=value` 過濾／當 facet，**建議前端 syslog parser 把 `sd.*` 攤平成 top-level 欄位**（或 encoder
  直接把常被查的欄位放 top-level）。此細節屬 parser 實作，encoder 只要穩定產出上述 schema 即可。

#### 3.1.1 演進／相容規則

`fcb.syslog.v1`（與所有 typed stream record schema）依下列規則演進：

1. **同版本內只能加選填欄位**：同一 type 版本內，schema 僅以新增 OPTIONAL 欄位的方式演進；既有欄位的
   型別／語意不在版本內變動。
2. **消費端忽略未知欄位、不因選填缺漏失敗**：消費端遇到不認得的欄位應略過、照常處理其餘欄位；遇到自己
   認得但被省略的選填欄位，應視為「不存在」而**不得**失敗。生產端缺值的選填欄位則直接省略。
3. **破壞性變更升型別版本**：凡會改動既有欄位型別／語意、或移除必填欄位的變更，一律發為新的 stream type
   版本（如 `fcb.syslog.v2`），**不**在既有版本內就地修改。
4. **未知型別／版本落 generic fallback、不致命**：沒有對應 handler 的 reader 遇到未知 type 或版本時，
   落到**通用 table／timeline fallback**，**不**因此中斷或報錯（與 fcb-evidence-model 的 unknown-type
   行為一致，見 §3.3）。

#### 3.1.2 ECS 對照（crosswalk）

本 schema 採扁平短名（flat short names）；下表對應到 Elastic Common Schema（ECS）欄位，方便對接既有
SIEM／log pipeline：

| 本 schema | ECS 欄位 |
|-----------|----------|
| `ts` | `@timestamp` |
| `host` | `host.name`／`log.syslog.hostname` |
| `app` | `log.syslog.appname`／`process.name` |
| `pid` | `process.pid`／`log.syslog.procid` |
| `severity` | `log.syslog.severity.code` |
| `facility` | `log.syslog.facility.code` |
| `msgid` | `log.syslog.msgid` |
| `sd` | `log.syslog.structured_data` |
| `msg` | `message` |
| `raw` | `event.original` |

### 3.2 `fcb.netflow.v1` / `fcb.json.v1`（內建但尚未定義）

兩者列在 `BUILTIN_STREAM_TYPES` 但**還沒有記錄 schema**。`fcb.json.v1` 預期是「通用 JSON 物件
（任意 CBOR map）」；`fcb.netflow.v1` 預期含五元組 + bytes/packets + 時間。兩者待真正要用時再比照
syslog 流程定義。

### 3.3 未知 type 的退場機制

`decode_streams` 會為每條 stream 標 `is_builtin`（type 是否在 `BUILTIN_STREAM_TYPES`）。
`is_builtin = false` 的 stream 仍是一等公民、照常解出，只是消費端落到**通用 table/timeline
fallback**（或某個註冊進 plugin registry 的 parser）。未知 type 夾在兩個已知 type 之間也不會中斷
其他 stream。

---

## 4. `.casework` 的 payload（Submission）

`.casework`（KIND=work）由**學生端**產生（`submission::pack_submission`）、**教師審閱平台**讀取
（`open_submission`，非 work KIND 會被拒）。其 `header.meta` 為**空 map** `{}`；payload 是
`Submission` 的 CBOR：

```text
Submission = {
  "case_id":     text,          // 對應的題目
  "bundle_hash": text,          // 產出時對應的證物版本
  "student":     { "id": text, "name": text },
  "notes":       [ <value>, … ], // 標註與證物參照（schema 由 workbench 自定，container 視為不透明）
  "report":      <value>,        // steps 答案（陣列）或 freeform 本文（字串）
  "activity":    [ <value>, … ], // 調查動作記錄
  "exported_at": text            // ISO-8601，由呼叫端蓋章
}
```

`notes` / `report` / `activity` 在 container 層是不透明 CBOR——schema 由 browser workbench 擁有，
encoder **不需要**關心（encoder 只產 `.case`）。

> `case_id` / `bundle_hash` 同時存在於明文 header **與** payload `Submission` 內：教師平台可不解密、
> 先讀 header 取得綁定資訊，解密後再用 payload 內的值交叉驗證。

---

## 5. Binding（綁定）

來源：`binding.rs`。

```text
verify_binding(work_case_id, work_bundle_hash, case_id, case_bundle_hash) -> BindingCheck
  = Match                     // 同 case、同證物版本
  | CaseMismatch              // 根本是別的 case
  | EvidenceVersionMismatch   // 同 case，但證物被重新發版

work_key(case_id) = "fcb:work:{case_id}"   // 本機（IndexedDB）作品分庫鍵；不同 case 不混
```

**建議 `bundle_hash` 定義**（codec 不強制，見 wire-format §5）：
`bundle_hash = compute_bundle_hash(.case 的明文 payload bytes)`，即 §2 信封壓縮／加密前的序列化位元組。
如此同一份證物的 hash 穩定，學生作品才能可靠綁回特定證物版本；證物一改版，舊作品開啟時即可用
`EvidenceVersionMismatch` 提示。

---

## 6. 答案安全不變量

來源：`task.rs`。學生端會**解密整個 `.case`**，所以 `.case` 內**任何**東西學生都看得到。因此：

- **`.case` 裡零答案／零評分標準／零步驟解答**；答案只留在教師母版與審閱平台。
- 型別上 `TaskStep` 根本沒有答案欄位，解碼經過 typed model 會把任何夾帶的答案欄位**丟掉**。
- 防呆：`FORBIDDEN_ANSWER_KEYS = ["answer", "answer_key", "rubric", "solution", "expected"]`；
  `contains_answer_fields(value)` 可遞迴檢查解出的 task 是否仍含這些 key（消費端可 assert）。

encoder 設計守則：把答案/rubric 放在**不會進 `.case`** 的教師母版資料結構；打包 `.case` 時只輸出
`TaskSpec`（prompt + answer_type）。

---

## 7. 已知缺口（實作 encoder 前先看）

1. **沒有 `.case` payload 信封 helper。** crate 有 `StreamData` 與 `decode_streams`（讀），但沒有
   「組 `{ streams: [...] }` 並 `bundle::pack_bytes`」的 `pack_case`（寫）。建議在 `evidence.rs`
   補 `CasePayload { streams }` + `pack_case(...)`，讓生產／消費共用同一份序列化。
2. **沒有 `bundle_hash` 的正規定義 helper。** `compute_bundle_hash` 接受任意 bytes；要固定成「明文
   payload bytes」（§5 建議）並包成 helper，避免生產端各算各的。
3. **`fcb.syslog.v1` 記錄 schema 尚未正式化**（§3.1 為草案）。`netflow.v1` / `json.v1` 同樣待定義。

以上三點建議「回頭在 `fcb` crate 補 helper + 定 schema」，而非只在 encoder 端自幹——這樣 browser
端、encoder、教師平台三方共用同一份真實程式碼，從根本杜絕格式漂移。
