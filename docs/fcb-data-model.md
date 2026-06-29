# FCB 資料模型（data model）

本文件描述 FCB 的**內層語意資料結構**——case builder 要塞進 `.case` 的東西、消費端（browser
workbench／教師審閱平台）寫進 `.casework` 的東西、兩者如何綁定，外加把這些結構放上 wire 所需的
**container 位元組佈局、CBOR 編碼慣例、crypto／compress 管線與 error 語意**。目標是讓本檔可以單獨
作為「實作 FCB 內層」的權威參考：每一個欄位、型別、不變量都能對到 `crates/fcb` 原始碼或 golden vector
的確切出處。

> **「case builder」是什麼？** 產生 `.case` 的工具一律稱 **case builder（建構器）**，它可能是 CLI、
> 服務、或 library 呼叫端；本檔不再稱它「encoder CLI」。消費端則泛稱**消費端**，或具體點名
> **browser workbench**（學生作答介面）／**教師審閱平台**。

> **權威來源優先序與 capability 清單見 [`README.md`](./README.md)**（單一權威：4 層優先序 + `openspec/specs/`
> 下 11 個 capability、其中 7 個 `fcb-*`）。本檔不重述整塊；任何衝突一律以 `crates/fcb` 原始碼與 golden
> vector 為準。

外層信封與密碼學的**主擁有者文件**是 [`fcb-wire-format.md`](./fcb-wire-format.md)；本檔為了自洽會重述
container 佈局與 crypto 重點（§8–§12），但兩者數值必須一致，皆以 `crates/fcb` 原始碼為準。

---

## 0. 資料放哪裡（明文 vs 加密）

| 資料 | 位置 | 不需 passphrase 可讀？ |
|------|------|:--:|
| `case_id`、`bundle_hash` | 明文 header | ✔ |
| KDF salt／cost params、AEAD nonce、`key_check` | 明文 header | ✔ |
| stream **manifest**（每條 stream 的 id／type／數量） | 明文 header `meta.streams` | ✔ |
| **task spec**（題目敘述、步驟提示） | 明文 header `meta.task` | ✔ |
| stream **records**（實際證物事件） | 加密 payload | ✘（要解密） |
| 學生 notes／report／activity | 加密 payload（`.casework`） | ✘ |

設計意圖：解鎖前就能顯示「這是哪個 case、裡面有哪些 stream、題目要做什麼」。證物本體與學生作品才需密碼。
明文 header 含 salt／nonce／cost 是刻意的：在還沒有 key 之前，reader 必須先讀到這些才能推導 key
（`container.rs` 的 `peek_header()` 即在無 key 下讀出這些欄位）。

> **安全注意：** 明文 header 雖可在解鎖前讀，但**已被 AEAD 認證**：封裝時整段明文 container 前綴（含完整
> header）綁進 AEAD 的 AAD（見 §11）。竄改 `case_id` / `bundle_hash` / `meta` 任一欄位都會讓 `open` 失敗為
> `Corrupt`。此外 `.case` 開封（`open_case`）還會用解密後的 canonical payload 重算 `bundle_hash` 並比對，
> 驗證內容定址（見 §7）。

---

## 1. `.case` 的 `header.meta`

`.case` 的 `meta` 是一個 CBOR map，含兩把 key，彼此獨立解讀、且容忍額外 key
（`evidence::manifest_from_meta` 只讀 `streams`、`task::task_from_meta` 只讀 `task`）：

```text
meta = {
  "streams": [ StreamManifest, … ],   // evidence.rs
  "task":    TaskSpec                  // task.rs（library 路徑可省略；省略則無題目）
}
```

`.casework` 的 `meta` 則固定是**空 map** `{}`（`Value::Map(vec![])`，見 `submission::pack_submission`），見 §7。

> golden vector 用 `CaseMeta { streams, task }`（`tests/vectors.rs` 的 `CaseMeta` struct）凍結這個結構；
> 解出來是 CBOR `a2`（map of 2）。
>
> 「`task` 可省略」指的是 **library** 的 `task_to_meta` 路徑——`TaskMeta.task` 標了
> `#[serde(default, skip_serializing_if = "Option::is_none")]`（`task.rs` 的 `TaskMeta`），`None` 時整把
> `task` key 省略。但 golden vector 的 `CaseMeta` 是普通 struct、`task` 為非 `Option` 欄位、**永遠**寫出
> `task`。直接照抄 `CaseMeta` 當範本的人要注意這個差異。讀取端兩條路都容忍：
> `manifest_from_meta` 只看 `streams`、`task_from_meta` 只看 `task`（`evidence::manifest_from_meta`、`task::task_from_meta`）。

### 1.1 `StreamManifest`（`evidence.rs` 的 `StreamManifest`）

```text
StreamManifest = {
  "id":      text,    // bundle 內唯一；與 payload 的 StreamData.id 對應
  "type":    text,    // namespaced + versioned，如 "fcb.syslog.v1"、"acme.edr.v1"
  "records": u64      // 該 stream 的記錄數
}
```

| Rust 欄位 | Rust 型別 | serde attr | CBOR key | sourceRef |
|-----------|-----------|------------|----------|-----------|
| `id` | `String` | （無） | `id` | `StreamManifest.id` |
| `stream_type` | `String` | `#[serde(rename = "type")]` | **`type`** | `StreamManifest.stream_type` |
| `records` | `u64` | （無） | `records` | `StreamManifest.records` |

> ⚠️ CBOR key 是 **`"type"`**（Rust 欄位名其實是 `stream_type`，靠 `#[serde(rename = "type")]` 改名）。
> 三個欄位都**沒有** serde default → 解碼時皆必填。`StreamManifest` derive `Eq`。

### 1.2 `TaskSpec`（`task.rs` 的 `TaskSpec`）

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

| `TaskSpec` 欄位 | Rust 型別 | serde default? | 必填? | sourceRef |
|-----------------|-----------|----------------|:--:|-----------|
| `report_mode` | `ReportMode`（enum） | 無 | **必填** | `TaskSpec.report_mode` |
| `instructions` | `String` | 無 | **必填** | `TaskSpec.instructions` |
| `steps` | `Vec<TaskStep>` | `#[serde(default)]` → `[]` | 否（缺即空陣列） | `TaskSpec.steps` |

`report_mode` 是 `ReportMode` enum，標 `#[serde(rename_all = "lowercase")]`（`task.rs` 的 `ReportMode`），
序列化為小寫 CBOR text `"steps"`／`"freeform"`。`TaskStep` 三個欄位（`id`／`prompt`／
`answer_type`）皆 `String`、皆無 default、皆必填（`task.rs` 的 `TaskStep`）。**`TaskStep` 沒有任何答案欄位**——
這是刻意的，見 §6 答案安全。`TaskSpec`／`TaskStep`／`ReportMode` 都 derive `Eq`。

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

`StreamData`（`evidence.rs` 的 `StreamData`）只有 `id: String` 與 `records: Vec<Value>` 兩欄，**不帶 `type`**：
型別只活在 manifest，payload 靠 `id` join。`StreamData` derive `PartialEq` 但**不** derive `Eq`
（因 `Vec<Value>` 不是 `Eq`）。

消費端用 `evidence::decode_streams(manifest, payload.streams)` 以 `id` 把 manifest
（型別）與 payload（記錄）對起來。逐筆 manifest entry：

- 在 payload 裡找 `s.id == m.id`；**找不到** → `FcbError::Malformed("payload missing stream {id}")`，
  且 `?` 短路整個 `collect()`，整批回 `Err`。
- 找到 → 產 `DecodedStream { id, stream_type, records, is_builtin }`。

不變量：

- 輸出**保留 manifest 順序**（迭代 manifest，非 payload），長度 == manifest 長度。
- 方向性：`decode_streams`（reader 端）對「manifest 有列、payload 缺」回 error；反向「payload 有、manifest
  沒列」**不檢查**，多出來的 payload stream 被**靜默忽略**（迭代由 `decode_streams` 內部以 manifest 驅動；
  此 reader-side 行為無專屬測試覆蓋，屬未證實是否刻意，見 §14.3）。

> ✅ **`fcb` crate 已有公開的 `.case` payload 信封型別與一步打包器。** 產生這個 `{ streams: [...] }` 信封
> **不必**自幹：公開型別 `fcb::case::CasePayload { streams }` 即此信封，`CasePayload::to_canonical_bytes()`
> 產出明文 payload bytes；而**推薦的一步生產者**是 `fcb::case::pack_case(&CaseInput, passphrase)`——它組信封、
> 算 canonical `bundle_hash`、組 meta 並封裝成 `.case`（§13）。golden vector 與 `stream_types.rs` round-trip
> 測試都直接 `use fcb::case::CasePayload`（共用同一份 public 型別、**非** test-local 副本）。crate 同時提供
> 讀側（`StreamData` 型別 + `decode_streams`）與寫側（`CasePayload` + `pack_case`）。

---

## 3. Stream type 的記錄 schema

每筆記錄的形狀**不**由 container 層規定（container 只看到 `StreamData.records: Vec<Value>`）；由 stream
的 `type` 決定，消費端依 type 對應 parser/view。

### 3.0 內建 type 與派發

```text
BUILTIN_STREAM_TYPES = ["fcb.syslog.v1", "fcb.netflow.v1", "fcb.json.v1"]   // evidence.rs BUILTIN_STREAM_TYPES（3 個）
is_builtin_type(t) = BUILTIN_STREAM_TYPES.contains(&t)                       // evidence.rs is_builtin_type()
```

這份清單**只表示「有內建 handler」，不是封閉清單**。任何 namespaced type 都是一等公民，未知 type
**不致命**。`is_builtin_type` 是**精確字串比對**（大小寫敏感、不做 namespace prefix 或 version-agnostic
比對）：`fcb.syslog.v2` 會回 `false`。

`decode_streams` 為每條 stream 標 `is_builtin`（`DecodedStream.is_builtin`）。
`is_builtin = false` 的 stream 仍照常解出，只是消費端落到**通用 table/timeline fallback**（或某個註冊進
plugin registry 的 parser；plugin registry 是消費端概念，本 crate **未**實作）。經測試證實：未知 type
`vendor.unknown.v3` 夾在兩個已知 type 之間，三條 stream 全部解出、中間 `is_builtin = false`、外兩
`true`，不中斷其他 stream（`evidence.rs` 的 `decode_streams` unknown-type 測試）。

> **兩層測試、刻意分工（讀 §3.1 前先看）：** byte-stability 的 golden vector（`vectors.rs`）刻意用便宜的
> placeholder 記錄（syslog 記錄是 `Value::Text("evt1")`／`("evt2")`；EDR 記錄是
> `Value::Integer(7)`，見 `tests/vectors.rs`），好讓 frozen hex 保持精簡。這些**不是**真實 schema 記錄。真正
> 凍結 `fcb.syslog.v1` 欄位集的是另一支 round-trip 測試 `crates/fcb/tests/stream_types.rs`（以 byte-faithful
> 打包→開封鎖住欄位集／key 名／value 型別），對應的 spec 在 `openspec/specs/fcb-stream-types/spec.md`。
> 因此 §3.1 的 worked example 來源是 `stream_types.rs`、與上面的 placeholder 是兩套不同記錄、彼此不衝突。

### 3.1 `fcb.syslog.v1`

每筆記錄 = 一個 CBOR map。

**核心原則（最重要）：** `raw` 是**無損真相**。當 `raw` 存在時，它逐字保留原始整行、為該記錄的權威來源
（`spec.md` "Raw line is the authoritative source"）。其他解析欄位皆為**盡力而為（best-effort）**的衍生值，
**不得**作為事件的唯一表示。消費端必須能從 `raw` 重新解析回任一解析欄位，且只要保留 `raw`，原始行上的
資訊就不會遺失。

> **「CBOR 型別」欄是 wire 型別、「值域約束」欄是 spec-level validation。** 兩者刻意分開：`severity`／
> `facility` 在 wire 上就是普通 `uint`，**0–7／0–23 的值域是 spec 約束、codec 不強制**（ciborium 解碼接受
> 任意 uint，crate 層不檢查值域，見下方說明）。機器抽取規則時，「型別」與「validation」請分兩類讀。

| 欄位 | CBOR 型別 | 值域約束 | 必填 | 說明 | sourceRef |
|------|-----------|----------|:--:|------|-----------|
| `ts` | text（RFC 3339） | — | ✔ | 事件發生時間。正規化為 UTC、結尾 `Z`、毫秒精度，如 `2026-03-14T08:21:33.512Z`。代表 originator 回報的事件時間；時間排序與 lite 查詢 `time` range 以此為準。 | fcb-stream-types spec |
| `host` | text | — | ✔ | 來源主機（hostname、FQDN 或 IP），照擷取到的樣子保留。 | fcb-stream-types spec |
| `msg` | text | — | ✔ | 解析後、人類可讀的訊息本文（RFC 5424 MSG）。 | fcb-stream-types spec |
| `raw` | text | — | | 擷取到的原始整行、**逐字保留**、未解析（見上方核心原則）。 | fcb-stream-types spec |
| `app` | text | — | | 來源應用程式／程式名（RFC 5424 APP-NAME、RFC 3164 TAG 或等價物），如 `sshd`。 | fcb-stream-types spec |
| `pid` | uint | — | | 來源行程 ID（PROCID）。 | fcb-stream-types spec |
| `severity` | uint | 0–7（spec 約束、codec 不強制） | | syslog severity 數字碼（0 = Emergency、7 = Debug）。 | fcb-stream-types spec |
| `facility` | uint | 0–23（spec 約束、codec 不強制） | | syslog facility 數字碼。 | fcb-stream-types spec |
| `msgid` | text | — | | 訊息型別識別碼（RFC 5424 MSGID）。 | fcb-stream-types spec |
| `sd` | map<SD-ID, map<param, value>> | — | | 結構化資料（RFC 5424 STRUCTURED-DATA），**依 SD-ID 分組**的巢狀 map：外層 key 是 SD-ID，內層是該元素的 param 名稱對字串值。 | fcb-stream-types spec |
| `format` | text | `rfc3164`／`rfc5424`／`other` 三選一（spec 約束、codec 不強制） | | 來源 wire format。 | fcb-stream-types spec |

> 下面兩個 producer 不變量（severity/facility 只存數字碼、ts 正規化為 UTC）在 `fcb-stream-types` spec 中
> **同寫於一條 producer requirement**；兩者是不同主張、但共用該出處。

**`severity`／`facility` 以數字為準**：只存數字碼，**不存名稱**（`fcb-stream-types` spec 的 producer requirement）；人類可讀的名稱由消費端從
數字碼衍生（對照：`0 Emergency`、`1 Alert`、`2 Critical`、`3 Error`、`4 Warning`、`5 Notice`、
`6 Informational`、`7 Debug`）。

**`ts` 一律正規化為 UTC**：當來源格式缺年份或時區（典型是 RFC 3164）時，由 case builder 推斷年份與時區
以產出 UTC `ts`，而原始行仍逐字保存在 `raw`（`fcb-stream-types` spec 的 producer requirement，與上方 severity/facility 共用此出處）。

`crates/fcb/tests/stream_types.rs` 把以下三筆 worked example 打包→開封做 byte-faithful round-trip，
鎖住欄位集／key 名／value 型別（`syslog_v1_records_round_trip_byte_faithfully`／
`syslog_v1_minimal_record_round_trips`）：

**範例一（RFC 5424 來源，`stream_types.rs` 的 `syslog_v1_records_round_trip_byte_faithfully`）：**

```json
{
  "ts": "2026-03-14T08:21:33.512Z",
  "host": "mymachine.example.com",
  "app": "su",
  "msgid": "ID47",
  "severity": 2,
  "facility": 4,
  "sd": { "ex@32473": { "iut": "3" } },
  "format": "rfc5424",
  "msg": "'su root' failed",
  "raw": "<34>1 2026-03-14T08:21:33.512Z mymachine.example.com su - ID47 [ex@32473 iut=\"3\"] 'su root' failed"
}
```

`severity`／`facility` 在 CBOR 是 integer（`Value::Integer(2)`／`(4)`）；
`sd` 是巢狀 map `{"ex@32473":{"iut":"3"}}`。

**範例二（RFC 3164 來源，缺年份／時區，由 case builder 推斷為 2026 年 UTC，同一 round-trip 測試）：**

```json
{
  "ts": "2026-10-11T22:14:15Z",
  "host": "mymachine",
  "app": "su",
  "severity": 2,
  "facility": 4,
  "format": "rfc3164",
  "msg": "'su root' failed for lonvick on /dev/pts/8",
  "raw": "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8"
}
```

（此例**省略** `msgid`／`sd`／`pid`——選填欄位無值即省略。）

**範例三（minimal record，僅必填欄位，`stream_types.rs` 的 `syslog_v1_minimal_record_round_trips`）：**

```json
{ "ts": "2026-01-01T00:00:00Z", "host": "h1", "msg": "hello" }
```

只含 `ts`／`host`／`msg` 即為合法 `fcb.syslog.v1` 記錄（`spec.md` minimal record scenario）。

> **🔎 消費端視角（case builder 可略過）：查詢／facet 對齊。** 前端 lite 查詢的 `field=value` 過濾與
> facet 針對**記錄的 top-level 欄位**：`severity`／`facility` 為整數可做 `>`／`<` 數值比較；`sd` 是巢狀
> map、非 top-level，若要讓 `sd.*` 的 param 可被過濾，需由前端 parser 攤平或 case builder 直接放
> top-level。這屬前端 parser／query 實作細節（消費端範疇，本 crate 不實作查詢層），**case builder 只要
> 穩定產出上述 schema 即可**，不必為查詢改變 schema。

#### 3.1.1 演進／相容規則

`fcb.syslog.v1`（與所有 typed stream record schema）依下列規則演進（`spec.md` 三條演進 requirement）：

1. **同版本內只能加選填欄位**：同一 type 版本內，schema 僅以新增 OPTIONAL 欄位的方式演進；既有欄位的
   型別／語意不在版本內變動。
2. **消費端忽略未知欄位、不因選填缺漏失敗**：消費端遇到不認得的欄位應略過、照常處理其餘欄位；遇到自己
   認得但被省略的選填欄位，應視為「不存在」而**不得**失敗。生產端缺值的選填欄位則直接省略。
3. **破壞性變更升型別版本**：凡會改動既有欄位型別／語意、或移除必填欄位的變更，一律發為新的 stream type
   版本（如 `fcb.syslog.v2`），**不**在既有版本內就地修改。
4. **未知型別／版本落 generic fallback、不致命**：沒有對應 handler 的 reader 遇到未知 type 或版本時，
   落到**通用 table／timeline fallback**，**不**因此中斷或報錯（與 fcb-evidence-model 的 unknown-type
   行為一致，見 §3.0）。

#### 3.1.2 ECS 對照（crosswalk）

本 schema 採扁平短名（flat short names）；下表對應到 Elastic Common Schema（ECS）欄位，方便對接既有
SIEM／log pipeline。

> **出處注意：** 此 ECS 對照表**只在 docs**，`fcb-stream-types/spec.md` 本身**無** ECS crosswalk
> （spec 全文無 "ECS" 字樣）。此表屬 docs 提供的補充對接指引，非規範性 requirement。

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

### 3.2 `fcb.netflow.v1`

每筆記錄 = 一個描述單一網路 flow 的 CBOR map。schema 由 `fcb-stream-types/spec.md` 的
「fcb.netflow.v1 record schema」requirement 凍結，並以 `crates/fcb/tests/stream_types.rs` 的
`netflow_v1_records_round_trip_byte_faithfully` 做 byte-faithful round-trip。

> 與 syslog 一致：「CBOR 型別」是 wire 型別、「值域約束」是 spec-level validation——port 0–65535、`proto`
> 的 IANA 語意皆為 spec 約束，**codec 不強制**（ciborium 接受任意 uint）。

| 欄位 | CBOR 型別 | 值域約束 | 必填 | 說明 |
|------|-----------|----------|:--:|------|
| `ts_start` | text（RFC 3339） | UTC、結尾 `Z` | ✔ | flow 第一個封包的時間。 |
| `ts_end` | text（RFC 3339） | UTC、結尾 `Z`、`>= ts_start` | ✔ | flow 最後一個封包的時間。 |
| `src_ip` | text | IPv4 或 IPv6 | ✔ | 來源位址，照擷取樣子保留。 |
| `dst_ip` | text | IPv4 或 IPv6 | ✔ | 目的位址。 |
| `src_port` | uint | 0–65535（無 port 協定用 `0`） | ✔ | 來源埠。 |
| `dst_port` | uint | 0–65535（無 port 協定用 `0`） | ✔ | 目的埠。 |
| `proto` | uint | IANA 協定號（6=TCP、17=UDP、1=ICMP） | ✔ | 傳輸層協定。 |
| `bytes` | uint | — | ✔ | flow 總位元組數。 |
| `packets` | uint | — | ✔ | flow 總封包數。 |
| `tcp_flags` | uint | — | | 整段 flow 觀察到的 TCP 旗標累積 OR（如 `0x02`=SYN）；僅 TCP flow。 |
| `app` | text | — | | 選填的 L7／應用標籤（如 `tls`、`dns`）。 |

**範例一（HTTPS TCP flow，含選填，`stream_types.rs` `netflow_tcp_record`）：**

```json
{
  "ts_start": "2026-03-14T08:20:00.000Z",
  "ts_end": "2026-03-14T08:20:03.500Z",
  "src_ip": "10.0.0.5", "dst_ip": "203.0.113.10",
  "src_port": 49512, "dst_port": 443, "proto": 6,
  "bytes": 18452, "packets": 24,
  "tcp_flags": 26, "app": "tls"
}
```

**範例二（DNS UDP flow，僅必填，`netflow_udp_record`）：**

```json
{
  "ts_start": "2026-03-14T08:19:58.000Z", "ts_end": "2026-03-14T08:19:58.040Z",
  "src_ip": "10.0.0.5", "dst_ip": "10.0.0.1",
  "src_port": 53124, "dst_port": 53, "proto": 17,
  "bytes": 168, "packets": 2
}
```

演進規則與 §3.1.1 相同（同版本只加選填欄位、破壞性變更升 `fcb.netflow.v2`）。

### 3.3 `fcb.json.v1`

每筆記錄 = 一個**任意 CBOR map**——作為沒有專屬 stream type 時的通用物件容器。schema 由
`fcb-stream-types/spec.md` 的「fcb.json.v1 record schema」requirement 凍結，並以
`json_v1_records_round_trip_byte_faithfully` 做 byte-faithful round-trip。

- **無必填 key**；map key 為 text，value 為任意 CBOR（text／int／float／bool／null／array／巢狀 map）。
- 消費端**逐位元保留**每個 key 與 value，**不得**丟棄、重排或強制轉型未知內容。

**範例（巢狀 alert 物件，`json_nested_record`）：**

```json
{ "kind": "alert", "score": 0.91, "tags": ["beacon", "c2"], "meta": { "asn": 64512 } }
```

巢狀 `meta` map、`tags` array、float `score`、int `asn` round-trip 後皆原樣保留。最小合法記錄如
`{ "k": "v" }`。

---

## 4. 把資料放上 wire：container 位元組佈局

以上 §1–§3 的 CBOR 結構最終被包進一個固定的 container frame。本節是**內層結構如何落到實體位元組**的權威
摘要；外層信封的完整論述見 [`fcb-wire-format.md`](./fcb-wire-format.md)。**所有整數為 little-endian、
字串為 UTF-8。**

```text
magic(4) | KIND(u8=1B) | container_version(u16 LE=2B) | hdr_len(u32 LE=4B)
         | header (hdr_len bytes, 明文 CBOR)
         | payload (其餘全部；= AEAD(zstd(serialized payload)))
```

| 偏移 | 欄位 | 大小 | 內容 | sourceRef |
|------|------|------|------|-----------|
| 0 | `magic` | 4 B | `[0x89, b'F', b'C', b'B']` = `89 46 43 42` | `MAGIC`；`encode_prefix()` |
| 4 | `KIND` | 1 B | `1` = `.case`、`2` = `.casework` | `BundleKind`；`encode_prefix()` |
| 5 | `container_version` | 2 B LE | `1`（`CONTAINER_VERSION`） | `CONTAINER_VERSION`；`encode_prefix()` |
| 7 | `hdr_len` | 4 B LE | 明文 header 的 byte 長度 | `encode_prefix()` |
| 11 | `header` | `hdr_len` B | 明文 CBOR header（§5） | `encode_prefix()` |
| 11+`hdr_len` | `payload` | 其餘 | `AEAD(zstd(明文 payload))`，verbatim 寫入 | `write_container()` |

固定前綴是 **11 bytes**（4+1+2+4），其後接 header 與 payload。`0x89` 仿 PNG，偵測 7/8-bit 傳輸損壞並
避免文字碰撞；`container_version` 是**獨立欄位、不**烤進 magic（`MAGIC` 與 `CONTAINER_VERSION` 為 `container.rs`
內各自獨立的常數）。

**golden vector 佐證（byte-exact，`tests/vectors.rs` 的 `FROZEN_CASE_HEX`／`FROZEN_WORK_HEX`）：**

```text
.case：     89 46 43 42 | 01 | 01 00 | dc 01 00 00 | a8 …    （KIND=1, ver=1, hdr_len=0x1dc=476, header=map(8)）
.casework： 89 46 43 42 | 02 | 01 00 | 25 01 00 00 | a8 …    （KIND=2, ver=1, hdr_len=0x125=293, header=map(8)）
```

`.case` 總長 578 bytes（11 prefix + 476 header + 91 payload）；`.casework` 總長 423 bytes
（11 + 293 + 119）。兩者 header 都以 `a8`（CBOR map of 8）起頭，因為 `Header` 永遠是 8 個欄位（§5）。

**讀取／驗證行為（`container.rs` 的 `peek_header()`／`read_container()`）：**

- 前 4 bytes 非 magic → `FcbError::BadMagic`（**不**嘗試解密）。
- 缺 KIND → `Malformed("missing KIND")`；未知 KIND byte → `Malformed("unknown KIND byte {other}")`
  （placeholder 名 `{other}`，逐字對齊 `BundleKind` 的 KIND 解析）。
- `header.min_reader > READER_VERSION(=2)` → `UnsupportedVersion { min_reader, supported }`（優雅拒絕，
  不吐部分資料）。`READER_VERSION` 從 `1` 升到 `2`，因明文 header 改綁進 AEAD AAD（§11）：pre-AAD 的 v1
  reader 開不了新 bundle，故新 bundle 寫 `min_reader = 2` 讓舊 reader 優雅拒絕。
- `hdr_len` 越界 → `Malformed("header length out of bounds")`；header CBOR 壞 →
  `Malformed("bad header CBOR: ...")`。
- `container_version` 目前**讀但不分派**（`peek_header` 直接 `let _container_version` 丟棄；
  `read_container` 保留但不驗證）：明文註解標「reserved for future parse-path dispatch; v1 is the only
  known layout today」（見 `read_container()` 內註解），即使 `container_version != 1` 也不報錯、照 v1 解析。
- `peek_header` **不複製 payload**，可在無 passphrase 下讀得 `case_id`／`kdf.salt`／`aead.nonce`
  （`container.rs` 的 `peek_header` 測試）。

---

## 5. 明文 header 欄位 + CBOR 編碼規則

`Header` 是 `#[derive(Serialize, Deserialize)]` struct（`container.rs` 的 `Header`），無 `#[serde(rename)]`／
`rename_all`，故 ciborium 把它編成 **以欄位名為 text key 的 CBOR map、順序 = 宣告順序**。共 8 欄 → `a8`。

| # | 欄位 | Rust 型別 | CBOR key（text） | 寫入值（生產端） | sourceRef |
|---|------|-----------|------------------|------------------|-----------|
| 1 | `header_schema_ver` | `u16` | `header_schema_ver` | `1` | `Header.header_schema_ver`；`pack_bytes()` |
| 2 | `min_reader` | `u16` | `min_reader` | `2`（AAD 格式） | `Header.min_reader`；`pack_bytes()` |
| 3 | `case_id` | `String` | `case_id` | 由呼叫端帶入 | `Header.case_id` |
| 4 | `bundle_hash` | `String` | `bundle_hash` | 由呼叫端帶入（§6） | `Header.bundle_hash` |
| 5 | `kdf` | `KdfParams` | `kdf` | 見下 | `Header.kdf` |
| 6 | `aead` | `AeadParams` | `aead` | 見下 | `Header.aead` |
| 7 | `key_check` | `Vec<u8>` | `key_check` | KCV（32B，§10） | `Header.key_check` |
| 8 | `meta` | `ciborium::value::Value` | `meta` | §1（case）／`{}`（work） | `Header.meta` |

**`KdfParams`（巢狀 map，5 欄 → `a5`，`container.rs` 的 `KdfParams`）：**

| # | 欄位 | 型別 | CBOR key | 寫入值 |
|---|------|------|----------|--------|
| 1 | `algo` | `String` | `algo` | `"argon2id"`（`pack_bytes()` 寫入；`derive_key` 唯一驗的 algo） |
| 2 | `salt` | `Vec<u8>` | `salt` | 16 隨機 bytes |
| 3 | `m_cost` | `u32` | `m_cost` | Argon2 記憶體 KiB；預設 19456 |
| 4 | `t_cost` | `u32` | `t_cost` | Argon2 迭代；預設 2 |
| 5 | `p_cost` | `u32` | `p_cost` | Argon2 平行度；預設 1 |

**`AeadParams`（巢狀 map，2 欄 → `a2`，`container.rs` 的 `AeadParams`）：**

| # | 欄位 | 型別 | CBOR key | 寫入值 |
|---|------|------|----------|--------|
| 1 | `algo` | `String` | `algo` | `"xchacha20poly1305"`（`pack_bytes()` 寫入；**描述性，open 不驗證**，見 §11） |
| 2 | `nonce` | `Vec<u8>` | `nonce` | 24 隨機 bytes |

### 5.1 ciborium 編碼慣例與陷阱（互通關鍵）

非 Rust 的 case builder／reader 重寫 codec 時必須對齊以下 ciborium 0.2 行為，否則無法 round-trip：

1. **`Vec<u8>` → CBOR array of uint（不是 byte string）。** `salt`／`nonce`／`key_check` 都是 `Vec<u8>`，
   ciborium 走 `serialize_seq` → 產出 **CBOR array（major type 4），每個 byte 編成一個 unsigned integer**，
   **不是** byte string（major type 2）。golden hex 佐證（`tests/vectors.rs` 的 `FROZEN_CASE_HEX`）：
   - `salt`（16B）→ 起頭 `0x90`（array(16)），後接 16 個 uint。
   - `nonce`（24B）→ 起頭 `0x98 0x18`（array, 1-byte count = 24），後接 24 個 uint。
   - `key_check`（32B）→ 起頭 `0x98 0x20`（array, count = 32），後接 32 個 uint。
   - array 計數規則：0–23 個 → 單 byte `0x80+n`；24–255 個 → `0x98` + 1-byte count。

   > 互通陷阱：若把這些寫成 CBOR byte string（起頭 `0x50`/`0x58`），ciborium 反序列化成 `Vec<u8>` 時
   > 預期 array，將**不相容**。

   **CBOR unsigned integer（major type 0）的計數編碼**（重建 byte-exact header／payload 必備）：所有
   `u16`／`u32`／`u64` 整數欄位——`header_schema_ver`／`min_reader`／`m_cost`／`t_cost`／`p_cost`／
   `records`、syslog 的 `pid`／`severity`／`facility`——以及 `salt`／`nonce`／`key_check` array **內每個
   ≥24 的 byte 元素**，全部照此門檻表編成 major type 0：

   | 整數值 | 編碼 | 範例 |
   |---|---|---|
   | 0–23 | 單 byte `0x00 + n`（inline） | `1`（`header_schema_ver`）→ `01`；`2`（`min_reader`，AAD 格式）→ `02`；`7`（severity Debug）→ `07` |
   | 24–255 | `0x18` + 1 byte | `32`（golden vector 測試 cost `m_cost`）→ `18 20`（`FROZEN_CASE_HEX`）；`0x2f`（key_check 內某 byte）→ `18 2f` |
   | 256–65535 | `0x19` + 2 byte（**big-endian**） | **`19456`（production 預設 `m_cost`，`DEFAULT_M_COST`）→ `19 4C 00`**；`65535` → `19 ff ff` |
   | 65536–2³²−1 | `0x1a` + 4 byte（big-endian） | `100000` → `1a 00 01 86 a0` |
   | 2³²–2⁶⁴−1 | `0x1b` + 8 byte（big-endian） | 罕見，僅超大 `records`／`pid` 才會用到 |

   > ⚠️ 機讀注意：CBOR 整數**不是** little-endian（container frame 的 `container_version`／`hdr_len`
   > 是 LE，但那是 frame 欄位、不走 CBOR）。CBOR 多 byte 計數一律 **big-endian**。golden vector 內唯一的
   > 整數 worked example 是測試 cost `m_cost=32`→`18 20`（`FROZEN_CASE_HEX`）；production 預設 `m_cost=19456`
   > 落在 256–65535 區段，編成 `19 4C 00`（`0x4C00 = 19456`），照抄測試向量無法推得，特別標出。

   **CBOR text string（major type 3）的長度前綴編碼**（與上面 array／uint 計數規則並列，重建 byte-exact
   header 必備）：每個欄位名、enum variant 小寫字串、`case_id`／`algo`／`format` 等 text value 一律照此編碼，
   長度以 **UTF-8 byte 數**計（非字元數）：

   | text 長度（UTF-8 bytes） | 長度前綴 | 範例（golden vector，`FROZEN_CASE_HEX`） |
   |---|---|---|
   | 0–23 | 單 byte `0x60 + len` | `"kdf"`（3）→ `63`；`"case_id"`（7）→ `67`；`"argon2id"`（8）→ `68`；`"min_reader"`（10）→ `6a`；`"bundle_hash"`（11）→ `6b`；`"header_schema_ver"`（17）→ `71`；`"xchacha20poly1305"`（17）→ `71` |
   | 24–255 | `0x78` + 1-byte len | golden vector 內無 ≥24 byte 的字串；舉例：32 byte 字串 → `78 20` 後接 32 個 UTF-8 byte |
   | 256–65535 | `0x79` + 2-byte len（big-endian） | — |

   > 注意：unsigned integer／text／byte string／array／map 的 count byte 共用同一套「short/1-byte/2-byte/
   > 4-byte/8-byte」階梯與 big-endian 多 byte 計數，只是 major type bits 不同（uint=`0x00`、byte
   > string=`0x40`、text=`0x60`、array=`0x80`、map=`0xa0`）。

2. **struct → text-key map，順序 = 宣告序。** 所有 struct（`Header`／`KdfParams`／`AeadParams`／
   `StreamManifest`／`StreamData`／`TaskSpec`／`TaskStep`／`Submission`／`Student`）都編成「以欄位名為
   text key 的 CBOR map」，key 順序即 Rust 欄位宣告順序。

   **⚠️ 例外：`ciborium::value::Value::Map` 依「插入順序」原樣輸出、不做 canonical 排序。** 上面「順序 =
   宣告序」只對 **struct** 成立。`header.meta`（型別是 `ciborium::value::Value`，見 `Header.meta`）與
   syslog 記錄裡的 `sd`（型別是 `Value::Map`）**不是 struct**，是任意 `Value::Map`——ciborium 依其內部
   `Vec<(key, value)>` 的**插入順序**逐筆寫出，既不排序、也不套用宣告序規則。這直接決定 byte-exactness：
   - **`header.meta`**：golden vector 走 `cbor::to_value(&CaseMeta { streams, task })`（`tests/vectors.rs`），
     `streams` 在前、`task` 在後的順序來自 `CaseMeta` struct 宣告序，解出來是 `a2` → `{streams, task}`。
     但若改用 `evidence::manifest_to_meta` + `task::task_to_meta` **合併**兩把 key（README 起手建議的路徑），
     合併後 `Value::Map` 的順序取決於**合併程式的插入序**、不再是 struct 宣告序——須**確保 `streams` 在
     `task` 之前**才能對齊 golden vector。
   - **syslog `sd`**：`Value::Map(vec![("ex@32473", Map(vec![("iut", "3")]))])`（`stream_types.rs` 範例一）
     的外層 SD-ID 與內層 param 順序，皆由建構 `vec` 的**插入序**決定。重建時須照原插入序排列。

3. **`#[serde(rename = "…")]`** 改單一欄位的 wire key：本 codec 只有 `StreamManifest.stream_type` →
   `"type"`（`evidence.rs` 的 `StreamManifest.stream_type`）。

4. **`#[serde(rename_all = "lowercase")]` enum → 小寫 text。** `ReportMode::Steps`／`Freeform` →
   `"steps"`／`"freeform"`（`task.rs` 的 `ReportMode`）。

5. **`Option` + `skip_serializing_if = "Option::is_none"` → `None` 省略整把 key。** 只有 library 的
   `TaskMeta.task` 用此（`task.rs` 的 `TaskMeta`）；`None` 時 `task` key 完全不出現。`#[serde(default)]`（如
   `TaskSpec.steps`、`StreamsMeta.streams`）則讓「缺 key」解成預設值而非報錯。

> **CBOR map 內 entry 的順序由「結構決定」、非排序**：ciborium 依宣告順序寫出，重現 byte-exact golden
> vector 的關鍵就是欄位宣告順序不能變。golden vector 測試 `case_vector_is_byte_stable` 一旦順序漂移即
> 失敗（"case format drifted"）。

---

## 6. `.casework` 的 payload（Submission）

`.casework`（KIND=2/work）由**消費端**產生（`submission::pack_submission`）、
**教師審閱平台**讀取（`submission::open_submission`，非 work KIND 會被拒）。其 `header.meta`
為**空 map** `{}`（`Value::Map(vec![])`，由 `pack_submission` 寫入）；payload 是 `Submission` 的 CBOR：

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

| 欄位 | Rust 型別 | CBOR key | 對 container 不透明? | sourceRef |
|------|-----------|----------|:--:|-----------|
| `case_id` | `String` | `case_id` | 否 | `Submission.case_id` |
| `bundle_hash` | `String` | `bundle_hash` | 否 | `Submission.bundle_hash` |
| `student` | `Student { id, name }` | `student` | 否（typed） | `Submission.student`；`Student` |
| `notes` | `Vec<Value>` | `notes` | **是**（每元素為不透明 CBOR） | `Submission.notes` |
| `report` | `Value` | `report` | **是**（單一不透明 CBOR；steps 為陣列、freeform 為字串） | `Submission.report` |
| `activity` | `Vec<Value>` | `activity` | **是**（每元素為不透明 CBOR） | `Submission.activity` |
| `exported_at` | `String` | `exported_at` | 否 | `Submission.exported_at` |

`notes` / `report` / `activity` 在 container 層是**不透明 CBOR**：schema 由 browser workbench 擁有，
case builder **不需要**關心（case builder 只產 `.case`）。所有 7 欄皆**無** serde default → 解碼時全必填。
`Submission` derive `PartialEq` 但**不** derive `Eq`（因含 `Value`）；`Student` 則
derive `Eq`。

**KIND-gating：** `open_submission` 對 `kind != BundleKind::Work` 回
`FcbError::Malformed("not a .casework (KIND != work)")`（確切字串）；測試
`opening_a_case_as_submission_is_rejected` 證實把 `.case` 當 submission 開會被拒。

> `case_id` / `bundle_hash` **同時存在於明文 header 與 payload `Submission` 內**（`pack_submission` 把
> 兩者複製進 header params）：教師平台可不解密、先讀 header 取得綁定資訊，解密後
> 再用 payload 內的值交叉驗證。

> ℹ️ **`Submission` 的 on-disk 位元組已由 golden vector 凍結。** `FROZEN_SUBMISSION_HEX`
> （`vectors.rs`，固定 salt/nonce）逐位元釘住真實 7 欄 `Submission` 的封裝，並由
> `submission_vector_is_byte_stable` 守住格式回歸、`frozen_submission_vector_decodes_to_expected_structure`
> 驗 7 欄還原。`FROZEN_WORK_HEX` 則是另一條向量，凍結 **test-local 的 3 欄 `WorkPayload`**（歷史保留，
> 證 container frame／空 meta `a0`／crypto 管線）。非 Rust 重實作者要驗 `Submission` byte-exactness，
> 直接比對 `FROZEN_SUBMISSION_HEX` 即可，不需自補向量。

---

## 7. Binding（綁定）

來源：`binding.rs`。

```text
verify_binding(work_case_id, work_bundle_hash, case_id, case_bundle_hash) -> BindingCheck   // binding::verify_binding
  = Match                     // 同 case、同證物版本
  | CaseMismatch              // 根本是別的 case
  | EvidenceVersionMismatch   // 同 case，但證物被重新發版

work_key(case_id) = "fcb:work:{case_id}"                    // binding::work_key；本機（IndexedDB）作品分庫鍵
compute_bundle_hash(bytes) = "sha256:" + lower_hex(SHA256(bytes))   // binding::compute_bundle_hash
```

**`verify_binding` 判斷順序（case 身分優先於證物版本）：**

1. `work_case_id != case_id` → `CaseMismatch`。
2. 否則 `work_bundle_hash != case_bundle_hash` → `EvidenceVersionMismatch`。
3. 否則 → `Match`。

`BindingCheck` derive `Debug, Clone, Copy, PartialEq, Eq`，三個 variant。case_id
**先於** bundle_hash 檢查——不同 case 永遠不會回報版本不符。

**`compute_bundle_hash` 格式：** 字面前綴 `"sha256:"`（7 chars）＋ 32-byte SHA-256 digest 的**小寫**
zero-padded hex（`{b:02x}`），輸出固定 **71 chars**（7 + 64，`binding.rs` 的 `compute_bundle_hash` 測試斷言）。內容可定址：同
輸入同 hash、異輸入異 hash。

**`work_key`：** 純 ASCII text `fcb:work:` ＋ case_id 原樣（無 hashing／escaping）；不同 case_id 產不同
key（作品隔離，`binding.rs` 的 `work_key` 測試）。實體 IndexedDB store 屬消費端範疇、不在 `binding.rs`。

**建議 `bundle_hash` 定義**（codec **不**強制）：
`bundle_hash = compute_bundle_hash(.case 的明文 payload bytes)`，即 §2 信封壓縮／加密**前**的序列化位元組。
如此同一份證物的 hash 穩定，學生作品才能可靠綁回特定證物版本；證物一改版，舊作品開啟時即可用
`EvidenceVersionMismatch` 提示。

> ℹ️ canonical 定義「`bundle_hash` = 明文 payload bytes 的 hash」已由 `fcb::case::case_bundle_hash` 落實、
> 由 `pack_case` 自動帶入 header（§13）。低階 `compute_bundle_hash` 仍接受任意 bytes、不自行驗證涵蓋範圍；
> golden vector 的 header 仍用假值 `"sha256:deadbeef"`（`vectors.rs`）。canonical hash 的凍結見 §14 的
> 「已關閉」註與 `case_canonical_bundle_hash_is_frozen`。

> ✅ **`.case` 開封會驗證內容定址（`open_case`，fcb-wasm）。** `open_case` 對解密後的 canonical payload
> **重算 `bundle_hash` 並和 header 值比對**，不符即 `Corrupt`（`fcb-wasm/src/lib.rs`）。header 現已被 AEAD
> AAD 認證、保證 `bundle_hash` 欄位未被竄改，但 AAD 不保證它**等於** payload 的 hash，故 `.case` 路徑額外
> 重算。
>
> ⚠️ **`.case` 與 `.casework` 在這點不同。** 上述重算**只**發生在 `.case`——因為 `.case` 的 `bundle_hash`
> 本就是其 canonical payload 的雜湊。**`.casework`（submission）不重算**：submission 的 header `bundle_hash`
> 是**綁回其 case 的參照**（記錄作答時所對的證物版本），不是 submission payload 的雜湊，所以
> `open_submission` 不對它做內容定址驗證。

> ⚠️ **binding 對 re-pack 敏感（by-design）。** 因 `bundle_hash` 是內容定址，case payload **任何**重新封裝
> （即使只改一個 byte）都會得到新雜湊，使既有 submission 的 binding 變成 `EvidenceVersionMismatch`。要避免
> 學生作品被誤判為舊版，請在**發題後凍結** case payload、不要重 pack。

---

## 8. 答案安全不變量

來源：`task.rs`。學生端會**解密整個 `.case`**，所以 `.case` 內**任何**東西學生都看得到。因此：

- **`.case` 裡零答案／零評分標準／零步驟解答**；答案只留在教師母版與審閱平台。
- 型別上 `TaskStep` 根本沒有答案欄位（`task.rs` 的 `TaskStep`），解碼經過 typed model 會把任何夾帶的答案欄位
  **丟掉**，因為沒有欄位能裝它。
- 防呆：`FORBIDDEN_ANSWER_KEYS = ["answer", "answer_key", "rubric", "solution", "expected"]`（5 個，
  `task.rs` 的 `FORBIDDEN_ANSWER_KEYS`）；`contains_answer_fields(value)`（`task::contains_answer_fields`）
  遞迴檢查解出的 task 是否仍含這些 key（消費端可 assert）。遞迴只走 `Value::Map`／`Value::Array`；
  scalar leaf 回 `false`；forbidden-key 比對只認 `Value::Text` key。
- 測試 `answer_fields_are_stripped_on_decode`（`task.rs`）證實：含夾帶 `answer` 的髒 map 被偵測
  （`true`），解成 `TaskSpec` 後丟掉，再編碼即回 `false`。

case builder 設計守則：把答案／rubric 放在**不會進 `.case`** 的教師母版資料結構；打包 `.case` 時只輸出
`TaskSpec`（prompt + answer_type）。

---

## 9. Crypto／compress 管線（內層結構如何被封裝）

§2／§6 的明文 payload 經 **compress-then-encrypt** 變成 container 的 payload 區（`compress.rs`、
`crypto.rs`、`bundle.rs`）。本節是讓本檔自洽的摘要；完整論述見 [`fcb-wire-format.md`](./fcb-wire-format.md)。

```text
pack：   plaintext --(zstd Fastest)--> zstd frame --(XChaCha20-Poly1305, AAD=明文前綴)--> ciphertext   // compress::pack_payload
open：   ciphertext --(AEAD open，先 KCV 檢查、AAD 須相符)--> zstd frame --(zstd decompress)--> plaintext  // compress::unpack_payload
```

AAD = 明文 container 前綴（magic、KIND、container_version、hdr_len、完整 header CBOR）；pack 對
`encode_prefix(...)` 封裝、open 取 payload 之前的位元組為 AAD（§11）。AAD 不符即視為竄改、失敗為 `Corrupt`。

**順序不變量：先壓縮、後加密**（`compress::pack_payload`；測試 `order_is_compress_then_encrypt` 證實解密後
得到的 inner 前 4 bytes == `ZSTD_MAGIC`、outer 前 4 bytes != `ZSTD_MAGIC`）。**禁止** encrypt-then-compress。

`pack_bytes`（`bundle::pack_bytes`）逐步：① 16-byte 隨機 salt → ② 24-byte 隨機 nonce → ③ 組 `KdfParams`
（algo `"argon2id"` + salt + cost）→ ④ `derive_key` 推 32-byte key → ⑤ 算 `key_check`（KCV，加密前算）
→ ⑥ 組 `Header`（`min_reader = 2`）並以 `encode_prefix(KIND, header)` 序列化出明文前綴 → ⑦ `pack_payload`
（compress-then-encrypt，**以該前綴為 AAD**）→ ⑧ 輸出 `前綴 ‖ ciphertext`。step ⑥ 必須先於 ⑦，因為前綴是
seal 的 AAD。salt/nonce 每次 `pack_bytes` 都用 `getrandom` 重新產生。

`open_bytes`（`bundle::open_bytes`）：`read_container` → 取 payload 之前的位元組為 `aad` →
`derive_key(passphrase, header.kdf)` →
`unpack_payload(key, header.key_check, header.aead.nonce, payload, aad)`。AAD 不符即 `Corrupt`。

---

## 10. KDF / KCV / AEAD 常數與行為

| 項目 | 值／行為 | sourceRef |
|------|----------|-----------|
| KDF 演算法 | Argon2id（`Algorithm::Argon2id`） | `crypto::derive_key` |
| Argon2 version | `Version::V0x13`（= Argon2 v1.3 / 0x13） | `crypto::derive_key` |
| 輸出長度 | 32 bytes（`KEY_LEN`，雙重綁定：`Some(32)` + 32-byte buffer） | `crypto::KEY_LEN` |
| 預設 cost | `m_cost=19456`（KiB）／`t_cost=2`（迭代）／`p_cost=1`（平行度） | `DEFAULT_M_COST`／`DEFAULT_T_COST`／`DEFAULT_P_COST` |
| salt 長度 | 16 bytes（隨機） | `bundle.rs` 的 `SALT_LEN` |
| algo 驗證 | `derive_key` **只**驗 `kdf.algo == "argon2id"`，否則 `Malformed("unsupported KDF: {algo}")` | `crypto::derive_key` |
| KCV 公式 | `key_check = SHA256(KCV_DOMAIN ‖ key)` = `SHA256(b"FCB-key-check-v1" ‖ key)`，32 bytes（domain 在前、key 在後） | `crypto::key_check_value`；`KCV_DOMAIN` |
| KCV 用途 | 開封時先 `ct_eq(KCV(key), header.key_check)` 區分錯密碼 vs 竄改 | `crypto::open_payload` |
| AEAD | XChaCha20-Poly1305，nonce **24 bytes**、**AAD = 明文 container 前綴**（magic/KIND/version/hdr_len/header CBOR） | `crypto::seal`／`crypto::open`；`NONCE_LEN` |
| AEAD algo 驗證 | `aead.algo`（`"xchacha20poly1305"`）寫入 header 但開封時**從不驗證**，`seal`/`open` 完全忽略 | `pack_bytes()` 寫入；`crypto`（無讀取） |
| 壓縮 | ruzstd 0.8（純 Rust，native == wasm），編碼僅 `CompressionLevel::Fastest`；標準 zstd frame magic `28 B5 2F FD` | `compress::compress`；`ZSTD_MAGIC` |

**錯密碼 vs 竄改的分流（`crypto::open_payload`）：**

```text
若 !ct_eq(KCV(key), expected_kcv)        → Err(WrongPassphrase)   // KCV 不符 = 錯密碼
否則 AEAD decrypt（tag 不符／被竄改）    → Err(Corrupt)
```

- KCV 比對用 hand-rolled constant-time `ct_eq`（長度不同立刻回 `false`、否則 XOR 累加無提前退出，
  `crypto.rs` 的 `ct_eq`）。
- nonce 長度 ≠ 24 → `Malformed("nonce must be 24 bytes, got N")`（`crypto.rs` 的 nonce 長度檢查）。
- 壞 zstd frame 解壓失敗 → `Corrupt`（`compress::decompress`）。

> ⚠️ **`aead.algo` 不被驗證**（相對地 `kdf.algo` **會**驗）：即使 header 把 `aead.algo` 寫成別的字串，
> `open` 仍照 XChaCha20-Poly1305 解。非 Rust reimplementation 不能靠這個欄位選 AEAD。

---

## 11. 安全特性（明確界線）

- **AEAD 同時認證 payload 與整段明文 header／前綴**：`seal`／`open` 都收 `aad: &[u8]`，封裝時把明文
  container 前綴（magic、KIND、container_version、hdr_len、完整 header CBOR）綁進 AEAD 的 AAD
  （`crypto::seal`／`crypto::open` 的 `aad` 參數、`bundle::pack_bytes`／`bundle::open_bytes`）。因此竄改 **header 任一欄位**（含
  `case_id`／`bundle_hash`／`meta`／manifest／task）——只要 passphrase 正確、header 仍能解析——`open`
  都會失敗為 **`Corrupt`**（測試 `header_tamper_is_corrupt`）。結構性檢查仍先行：magic 壞 → `BadMagic`、
  未知 KIND → `Malformed`。
- **明文 header 是刻意的（但仍被認證）**：salt／cost／nonce 必須在還沒 key 之前就讀到
  （`container.rs` 的 `peek_header()` 即在無 key 下讀出），所以 header 是明文、可 `peek`；不過整段前綴綁進 AAD，明文 ≠ 未認證。
- **`.case` 開封驗證內容定址、`.casework` 不驗**：`open_case`（fcb-wasm）對解密後的 canonical payload
  重算 `bundle_hash` 並比對，不符即 `Corrupt`（AAD 只保證欄位未被竄改、不保證等於 payload hash，故額外
  重算）。`.casework` 的 header `bundle_hash` 是綁回 case 的參照、非 submission payload 的雜湊，
  `open_submission` 不重算（§7）。
- **`bundle_hash` 是明文 payload 的確認 oracle（低熵情境，by-design）**：`bundle_hash` 是**明文 payload 的
  SHA-256**、存在不需 passphrase 即可讀的明文 header 裡。對**低熵／可猜的 payload**，能猜中 payload 的人
  可藉這個雜湊**確認**猜測；高熵／大型 payload 不受影響。這是內容定址綁定的固有取捨，非漏洞。
- **binding 對 re-pack 敏感（by-design）**：因 `bundle_hash` 內容定址，case payload 任何重新封裝都得到新
  雜湊，使既有 submission 的 binding 變成 `EvidenceVersionMismatch`——發題後請凍結 payload（§7）。
- **錯密碼與竄改可區分且皆非靜默**：`WrongPassphrase`（KCV 不符）vs `Corrupt`（KCV 符但 AEAD/AAD/zstd
  失敗），兩者都**不**吐部分／損壞資料（§10、§12）。
- **答案安全**靠 typed model 結構性保證（§8），非靠加密——學生本來就能解密整包。

---

## 12. Error 語意

`FcbError`（`#[derive(Debug, Error, PartialEq, Eq)]`，`error.rs` 的 `FcbError`）五個 variant：

| Variant | `#[error]` 文字 | 語意 | 觸發點（摘） | sourceRef |
|---------|-----------------|------|--------------|-----------|
| `BadMagic` | `"not an FCB container (bad magic)"` | 前 4 bytes 非 FCB magic | `peek_header`／`read_container` 入口 | `FcbError::BadMagic` |
| `UnsupportedVersion { min_reader, supported }` | `"unsupported FCB version: …"` | bundle 要求的 reader 比本 reader 新 | `min_reader > READER_VERSION(=2)` | `FcbError::UnsupportedVersion`；`read_container` |
| `Malformed(String)` | `"malformed FCB container: {0}"` | 結構性無效 | 壞長度前綴、未知/缺 KIND、header 越界、壞 header CBOR、未知 KDF algo、nonce 長度錯、`payload missing stream {id}` 等 | `FcbError::Malformed`；container/crypto/evidence 多處 |
| `WrongPassphrase` | `"wrong passphrase"` | KCV 不符（錯密碼） | `crypto::open_payload` 的 KCV 比對 | `FcbError::WrongPassphrase` |
| `Corrupt` | `"corrupt or tampered bundle"` | KCV 符但 AEAD/zstd 失敗（竄改）；或 payload 層 `cbor::decode` 失敗 | `crypto::open`／`compress::decompress`／`cbor::decode` | `FcbError::Corrupt` |

> 注意分流：`cbor::decode`（**解密/解壓後**的 payload CBOR 解碼）失敗映射成 **`Corrupt`**，
> 而 header／meta 層的 CBOR 失敗（`cbor::to_value`／`cbor::from_value`／`cbor::encode`）映射成 **`Malformed`**
> （皆為各自的 `map_err`）。`WrongPassphrase` 與
> `Corrupt` 刻意分開——對學生／operator 意義不同（密碼錯 vs 檔案被竄改，見 `error.rs` 的 `FcbError` 模組註解）。

---

## 13. 端到端打包流程（end-to-end，可照做）

以 `.case` 為例。**Rust 呼叫端最簡路徑是 `fcb::case::pack_case(&CaseInput, passphrase)`**——一步組
`{streams}` 信封、以 canonical 序列化算出 `bundle_hash`、組 `{streams, task?}` meta 並封裝。下面拆解其
底層步驟，供理解格式與非 Rust 重實作參考：

1. **整理 manifest**：對每條 stream 建 `StreamManifest { id, type, records }`（記得 CBOR key 是 `"type"`）。
2. **整理 task**（可選）：建 `TaskSpec { report_mode, instructions, steps }`，**不含任何答案欄位**（§8）。
3. **組 meta**：`meta = { "streams": [...manifest], "task": <TaskSpec> }`；可用
   `evidence::manifest_to_meta` 產出 `{streams}`，再合併 `task::task_to_meta` 的 `{task}`，或直接照
   `CaseMeta { streams, task }` 形狀組（注意 library `task_to_meta` 在 `None` 時省略 `task`，§1）。
4. **組 payload 信封**：`payload = { "streams": [ StreamData { id, records }, ... ] }`，`records` 依各
   stream type 的 schema（syslog 見 §3.1）。公開型別 `fcb::case::CasePayload { streams }` 即此信封；
   `CasePayload::to_canonical_bytes()`（= `cbor::encode`）產出明文 payload bytes。
5. **算 canonical `bundle_hash`**：`case::case_bundle_hash(&CasePayload)` = `compute_bundle_hash(明文
   payload bytes)`（§7），得 `"sha256:…"`。
6. **打包**：`BundleParams::new(BundleKind::Case, case_id, bundle_hash, meta)`（用預設 Argon2 cost），
   呼叫 `bundle::pack_bytes(&params, &payload_bytes, passphrase)`。內部會壓縮→加密→組 container frame
   （§9）。
7. **產出** `.case` bytes（`89 46 43 42 01 …`）。

`.casework` 則直接用 `submission::pack_submission(&Submission, passphrase)`（meta 自動為 `{}`，
見 `submission::pack_submission`），不需手組信封。

開封：`bundle::open_bytes(bytes, passphrase)` → `(kind, header, payload_bytes)`；再
`cbor::decode::<CasePayload>(payload_bytes)` 取 `streams`，配 `manifest_from_meta(&header.meta)` 餵
`decode_streams` join 出 `DecodedStream`。`.casework` 用 `open_submission`（KIND-gated）。

---

## 14. 已知缺口（實作 case builder 前先看）與 Non-Goals

**已知缺口（Known Gaps）：**

> ✅ **已關閉（本批）：** (1) `.case` payload 信封 helper 與 canonical `bundle_hash` 凍結——公開型別
> `fcb::case::CasePayload { streams }` 與 `pack_case(&CaseInput, passphrase)` 統一生產／消費序列化、
> `case::case_bundle_hash` 凍結 `bundle_hash = sha256(明文 payload bytes)`（§13 步驟 4–5）；(2)
> **`fcb.netflow.v1` / `fcb.json.v1` 記錄 schema 凍結**（§3.2／§3.3，`stream_types.rs` round-trip 測試）；
> (3) **`Submission` byte-stable golden vector**——`FROZEN_SUBMISSION_HEX` + `submission_vector_is_byte_stable`
> 釘住真實 7 欄 `Submission` 的 on-disk 位元組（§6）。

1. **核心 `fcb` crate 自帶的 `src/wasm.rs` 是 stub（**非**整體 WASM 缺口）。** `crates/fcb/src/wasm.rs`
   只導出 `fcb_version()`（回 `CARGO_PKG_VERSION`）。但**消費面 WASM 其實完整**：完整 JS surface 由
   `fcb-wasm` bridge crate 提供（`peekHeader`／`openCase`／`openSubmission`／`packSubmission`／`packCase`／
   `computeBundleHash`／`verifyBinding`／`workKey`，見 `crates/fcb-wasm/src/lib.rs`）。因此「WASM 綁定僅
   `fcb_version`」**只**對核心 crate 的 stub 為真，不代表消費端缺 WASM。
2. **plugin parser registry 未實作。** `DecodedStream` 的註解提到 `is_builtin = false` 可落到「a registered
   plugin」，但本 crate **沒有**任何 registry 程式碼——plugin registry 純屬消費端概念
   （另見下方 Non-Goals）。
3. **（reader-only）payload 多餘 stream 的行為無測試斷言。** reader 端 `decode_streams` 對 payload 含 manifest
   未列的 stream 會**靜默忽略**（§2 不變量，迭代由 `decode_streams` 以 manifest 驅動），但此 reader-side 行為
   **缺乏專屬測試**——是否刻意如此**未證實**。（注意生產端不同：見下方第 4 點，生產者已強制 manifest≡payload。）
4. **（reader-only）`manifest.records` 對 reader 為 advisory；生產端已由 `pack_case` 強制。** 生產時推薦的
   `fcb::case::pack_case` **強制** manifest 與 payload 的 stream-id 集合**雙向相等**、且每條 stream 的 `records`
   計數須等於實際記錄數，任何 mismatch／多出／缺漏／重複都會 reject 為 `Malformed`（有對應 reject 測試）。
   但 reader 端 `open`／`decode_streams` **不**回頭拿 `records` 和實際記錄數核對；故對 reader 而言 peek 階段
   宣告的 `records` 仍是 advisory——消費端應以**開封後 payload** 解出的記錄為準推導筆數。

**Non-Goals（本檔／本層不負責）：**

- **不**定義 `notes`／`report`／`activity` 的 schema——那是 browser workbench 的範疇（§6 不透明）。
- **不**定義 plugin parser registry 的執行機制——`is_builtin = false` 的 fallback 是消費端概念，本 crate
  未實作。
- 低階 `compute_bundle_hash` 這個 primitive 本身**不**驗證涵蓋範圍（canonical 定義已由 `case::case_bundle_hash`
  落實、`.case` 開封由 `open_case` 重算驗證，§7／§11）。**明文 header／前綴現已被 AEAD AAD 認證**，竄改
  header 會失敗為 `Corrupt`（§11）——這已**不再**是 Non-Goal。
- 實體 IndexedDB／儲存層**不**在 binding 範疇（`work_key` 只給 key 字串）。

剩餘缺口建議「回頭在 `fcb` crate 定 schema」，而非只在消費端自幹。這樣 browser 端、case builder、
教師平台三方共用同一份真實程式碼，從根本杜絕格式漂移。`.case` 產出已可直接用 `fcb::case::pack_case`。

---

## 15. 相依套件版本表

來源：`crates/fcb/Cargo.toml`（版本字串為 Cargo semver caret 範圍，如 `"0.2"` = `^0.2`；實際解析 patch
版需看 `Cargo.lock`）。

| 套件 | 版本 | features | 用途 | sourceRef |
|------|------|----------|------|-----------|
| `serde` | `1` | `["derive"]` | 序列化 derive | `[dependencies]` |
| `ciborium` | `0.2` | — | CBOR 編解碼（§5.1 慣例由它決定） | `[dependencies]` |
| `argon2` | `0.5` | — | Argon2id KDF（§10） | `[dependencies]` |
| `chacha20poly1305` | `0.10` | — | XChaCha20-Poly1305 AEAD（§10–§11） | `[dependencies]` |
| `ruzstd` | `0.8` | — | 純 Rust zstd（native==wasm，僅 Fastest，§9） | `[dependencies]` |
| `sha2` | `0.10` | — | SHA-256（KCV、`bundle_hash`） | `[dependencies]` |
| `thiserror` | `2` | — | `FcbError` derive（§12） | `[dependencies]` |
| `getrandom` | `0.2` | wasm 加 `["js"]` | 隨機 salt/nonce | `[dependencies]` + wasm32 `[target.…dependencies]` |
| `hex`（dev） | `0.4` | — | golden vector hex 比對 | `[dev-dependencies]` |
| `wasm-bindgen`（wasm） | `0.2` | — | WASM 入口 | wasm32 `[target.…dependencies]` |
