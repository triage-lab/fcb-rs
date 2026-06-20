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
> 下 7 個 capability、其中 5 個 `fcb-*`）。本檔不重述整塊；任何衝突一律以 `crates/fcb` 原始碼與 golden
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

設計意圖：解鎖前就能顯示「這是哪個 case、裡面有哪些 stream、題目要做什麼」；證物本體與學生作品才需密碼。
明文 header 含 salt／nonce／cost 是刻意的——在還沒有 key 之前，reader 必須先讀到這些才能推導 key
（`crates/fcb/src/container.rs:12-13`）。

> **安全注意：** 明文 header **未被 AEAD 認證**（AEAD 只認證 payload，無 AAD，見 §11）。`case_id` /
> `bundle_hash` / `meta` 可被竄改而不觸發解密失敗；要防範須由生產端透過 `bundle_hash` 涵蓋範圍另外保證
> （見 §6、§13）。

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

`.casework` 的 `meta` 則固定是**空 map** `{}`（`Value::Map(vec![])`，`submission.rs:49`），見 §7。

> golden vector 用 `CaseMeta { streams, task }`（`tests/vectors.rs:31-35`）凍結這個結構；解出來是
> CBOR `a2`（map of 2，`vectors.rs:85`）。
>
> 「`task` 可省略」指的是 **library** 的 `task_to_meta` 路徑——`TaskMeta.task` 標了
> `#[serde(default, skip_serializing_if = "Option::is_none")]`（`task.rs:49-50`），`None` 時整把
> `task` key 省略。但 golden vector 的 `CaseMeta` 是普通 struct、`task` 為非 `Option` 欄位、**永遠**寫出
> `task`（`vectors.rs:31-35, 85`）。直接照抄 `CaseMeta` 當範本的人要注意這個差異。讀取端兩條路都容忍：
> `manifest_from_meta` 只看 `streams`、`task_from_meta` 只看 `task`（`evidence.rs:69-72`、`task.rs:60-63`）。

### 1.1 `StreamManifest`（`evidence.rs:27-33`）

```text
StreamManifest = {
  "id":      text,    // bundle 內唯一；與 payload 的 StreamData.id 對應
  "type":    text,    // namespaced + versioned，如 "fcb.syslog.v1"、"acme.edr.v1"
  "records": u64      // 該 stream 的記錄數
}
```

| Rust 欄位 | Rust 型別 | serde attr | CBOR key | sourceRef |
|-----------|-----------|------------|----------|-----------|
| `id` | `String` | （無） | `id` | evidence.rs:28 |
| `stream_type` | `String` | `#[serde(rename = "type")]` | **`type`** | evidence.rs:30-31 |
| `records` | `u64` | （無） | `records` | evidence.rs:32 |

> ⚠️ CBOR key 是 **`"type"`**（Rust 欄位名其實是 `stream_type`，靠 `#[serde(rename = "type")]` 改名）。
> 三個欄位都**沒有** serde default → 解碼時皆必填。derive `Eq`（`evidence.rs:26`）。

### 1.2 `TaskSpec`（`task.rs:39-45`）

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
| `report_mode` | `ReportMode`（enum） | 無 | **必填** | task.rs:41 |
| `instructions` | `String` | 無 | **必填** | task.rs:42 |
| `steps` | `Vec<TaskStep>` | `#[serde(default)]` → `[]` | 否（缺即空陣列） | task.rs:43-44 |

`report_mode` 是 `ReportMode` enum，標 `#[serde(rename_all = "lowercase")]`（`task.rs:22`），序列化為
小寫 CBOR text `"steps"`／`"freeform"`（`task.rs:24, 27`）。`TaskStep` 三個欄位（`id`／`prompt`／
`answer_type`）皆 `String`、皆無 default、皆必填（`task.rs:33-35`）。**`TaskStep` 沒有任何答案欄位**——
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

`StreamData`（`evidence.rs:37-39`）只有 `id: String` 與 `records: Vec<Value>` 兩欄；**不帶 `type`**——
型別只活在 manifest，payload 靠 `id` join。`StreamData` derive `PartialEq` 但**不** derive `Eq`
（因 `Vec<Value>` 不是 `Eq`，`evidence.rs:36`）。

消費端用 `evidence::decode_streams(manifest, payload.streams)`（`evidence.rs:77-93`）以 `id` 把 manifest
（型別）與 payload（記錄）對起來。逐筆 manifest entry：

- 在 payload 裡找 `s.id == m.id`；**找不到** → `FcbError::Malformed("payload missing stream {id}")`
  （`evidence.rs:84`），且 `?` 短路整個 `collect()`，整批回 `Err`。
- 找到 → 產 `DecodedStream { id, stream_type, records, is_builtin }`（`evidence.rs:85-90`）。

不變量：

- 輸出**保留 manifest 順序**（迭代 manifest，非 payload），長度 == manifest 長度。
- 方向性：「manifest 有列、payload 缺」是 error；反向「payload 有、manifest 沒列」**不檢查**，多出來的
  payload stream 被**靜默忽略**（迭代由 manifest 驅動，`evidence.rs:78-92`；此行為無專屬測試覆蓋，屬未證實
  是否刻意）。

> ⚠️ **已知缺口（見 §14）：** `fcb` crate **沒有**公開 helper 產生這個 `{ streams: [...] }` 信封；
> golden vector 與 stream_types 測試各自在 test 內自定 `CasePayload { streams }`（`vectors.rs:36-39`、
> `stream_types.rs:18-21`）。crate 只提供**讀**側（`StreamData` 型別 + `decode_streams`），沒有**寫**側
> 的 `pack_case`。case builder 要嘛照樣自組信封並餵 `bundle::pack_bytes`，要嘛回頭在 `evidence.rs`
> 補 `CasePayload { streams }` + `pack_case(...)`（建議）。

---

## 3. Stream type 的記錄 schema

每筆記錄的形狀**不**由 container 層規定（container 只看到 `Vec<Value>`，`evidence.rs:39`）；由 stream
的 `type` 決定，消費端依 type 對應 parser/view。

### 3.0 內建 type 與派發

```text
BUILTIN_STREAM_TYPES = ["fcb.syslog.v1", "fcb.netflow.v1", "fcb.json.v1"]   // evidence.rs:18（3 個）
is_builtin_type(t) = BUILTIN_STREAM_TYPES.contains(&t)                       // evidence.rs:21-23
```

這份清單**只表示「有內建 handler」，不是封閉清單**——任何 namespaced type 都是一等公民，未知 type
**不致命**。`is_builtin_type` 是**精確字串比對**（大小寫敏感、不做 namespace prefix 或 version-agnostic
比對）：`fcb.syslog.v2` 會回 `false`。

`decode_streams` 為每條 stream 標 `is_builtin`（`DecodedStream.is_builtin`，`evidence.rs:49-51`）。
`is_builtin = false` 的 stream 仍照常解出，只是消費端落到**通用 table/timeline fallback**（或某個註冊進
plugin registry 的 parser；註：plugin registry 是消費端概念，本 crate **未**實作）。經測試證實：未知 type
`vendor.unknown.v3` 夾在兩個已知 type 之間，三條 stream 全部解出、中間 `is_builtin = false`、外兩
`true`，不中斷其他 stream（`evidence.rs:136-152`，`#[test]` attribute 在 :135）。

> **兩層測試、刻意分工（讀 §3.1 前先看）：** byte-stability 的 golden vector（`vectors.rs`）刻意用便宜的
> placeholder 記錄（syslog 記錄是 `Value::Text("evt1")`／`("evt2")`，`vectors.rs:88`；EDR 記錄是
> `Value::Integer(7)`，`vectors.rs:89`），好讓 frozen hex 保持精簡——這些**不是**真實 schema 記錄。真正
> 凍結 `fcb.syslog.v1` 欄位集的是另一支 round-trip 測試 `crates/fcb/tests/stream_types.rs`（以 byte-faithful
> 打包→開封鎖住欄位集／key 名／value 型別），對應的 spec 在 `openspec/specs/fcb-stream-types/spec.md`。
> 因此 §3.1 的 worked example 來源是 `stream_types.rs`、與上面的 placeholder 是兩套不同記錄、彼此不衝突。

### 3.1 `fcb.syslog.v1`

每筆記錄 = 一個 CBOR map。

**核心原則（最重要）：** `raw` 是**無損真相**——當 `raw` 存在時，它逐字保留原始整行、為該記錄的權威來源
（`spec.md` "Raw line is the authoritative source"）。其他解析欄位皆為**盡力而為（best-effort）**的衍生值，
**不得**作為事件的唯一表示；消費端必須能從 `raw` 重新解析回任一解析欄位，且只要保留 `raw`，原始行上的
資訊就不會遺失。

> **「CBOR 型別」欄是 wire 型別、「值域約束」欄是 spec-level validation。** 兩者刻意分開：`severity`／
> `facility` 在 wire 上就是普通 `uint`，**0–7／0–23 的值域是 spec 約束、codec 不強制**（ciborium 解碼接受
> 任意 uint，crate 層不檢查值域，見下方說明）。機器抽取規則時，「型別」與「validation」請分兩類讀。

| 欄位 | CBOR 型別 | 值域約束 | 必填 | 說明 | sourceRef |
|------|-----------|----------|:--:|------|-----------|
| `ts` | text（RFC 3339） | — | ✔ | 事件發生時間。正規化為 UTC、結尾 `Z`、毫秒精度，如 `2026-03-14T08:21:33.512Z`。代表 originator 回報的事件時間；時間排序與 lite 查詢 `time` range 以此為準。 | spec.md:13 |
| `host` | text | — | ✔ | 來源主機（hostname、FQDN 或 IP），照擷取到的樣子保留。 | spec.md:14 |
| `msg` | text | — | ✔ | 解析後、人類可讀的訊息本文（RFC 5424 MSG）。 | spec.md:15 |
| `raw` | text | — | | 擷取到的原始整行、**逐字保留**、未解析（見上方核心原則）。 | spec.md:16 |
| `app` | text | — | | 來源應用程式／程式名（RFC 5424 APP-NAME、RFC 3164 TAG 或等價物），如 `sshd`。 | spec.md:17 |
| `pid` | uint | — | | 來源行程 ID（PROCID）。 | spec.md:18 |
| `severity` | uint | 0–7（spec 約束、codec 不強制） | | syslog severity 數字碼（0 = Emergency、7 = Debug）。 | spec.md:19 |
| `facility` | uint | 0–23（spec 約束、codec 不強制） | | syslog facility 數字碼。 | spec.md:20 |
| `msgid` | text | — | | 訊息型別識別碼（RFC 5424 MSGID）。 | spec.md:21 |
| `sd` | map<SD-ID, map<param, value>> | — | | 結構化資料（RFC 5424 STRUCTURED-DATA），**依 SD-ID 分組**的巢狀 map：外層 key 是 SD-ID，內層是該元素的 param 名稱對字串值。 | spec.md:22 |
| `format` | text | `rfc3164`／`rfc5424`／`other` 三選一（spec 約束、codec 不強制） | | 來源 wire format。 | spec.md:23 |

> 下面兩個 producer 不變量（severity/facility 只存數字碼、ts 正規化為 UTC）在 spec 中**同寫於
> `spec.md:25` 一行**；兩者是不同主張、但共用該行出處。

**`severity`／`facility` 以數字為準**：只存數字碼，**不存名稱**（`spec.md:25`）；人類可讀的名稱由消費端從
數字碼衍生（對照：`0 Emergency`、`1 Alert`、`2 Critical`、`3 Error`、`4 Warning`、`5 Notice`、
`6 Informational`、`7 Debug`）。

**`ts` 一律正規化為 UTC**：當來源格式缺年份或時區（典型是 RFC 3164）時，由 case builder 推斷年份與時區
以產出 UTC `ts`，而原始行仍逐字保存在 `raw`（`spec.md:25`，與上方 severity/facility 共用此行）。

`crates/fcb/tests/stream_types.rs` 把以下三筆 worked example 打包→開封做 byte-faithful round-trip，
鎖住欄位集／key 名／value 型別（`stream_types.rs:122-152`）：

**範例一（RFC 5424 來源，`stream_types.rs:34-58`）：**

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

`severity`／`facility` 在 CBOR 是 integer（`Value::Integer(2)`／`(4)`，`stream_types.rs:45-46`）；
`sd` 是巢狀 map `{"ex@32473":{"iut":"3"}}`（`stream_types.rs:36-39`）。

**範例二（RFC 3164 來源，缺年份／時區，由 case builder 推斷為 2026 年 UTC，`stream_types.rs:61-80`）：**

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

**範例三（minimal record，僅必填欄位，`stream_types.rs:83-89`）：**

```json
{ "ts": "2026-01-01T00:00:00Z", "host": "h1", "msg": "hello" }
```

只含 `ts`／`host`／`msg` 即為合法 `fcb.syslog.v1` 記錄（`spec.md` minimal record scenario）。

> **🔎 消費端視角（case builder 可略過）：查詢／facet 對齊。** 前端 lite 查詢的 `field=value` 過濾與
> facet 針對**記錄的 top-level 欄位**：`severity`／`facility` 為整數可做 `>`／`<` 數值比較；`sd` 是巢狀
> map、非 top-level，若要讓 `sd.*` 的 param 可被過濾，需由前端 parser 攤平或 case builder 直接放
> top-level。這屬 parser／query 實作細節（詳見 `query-model` spec），**case builder 只要穩定產出上述
> schema 即可**，不必為查詢改變 schema。

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

### 3.2 `fcb.netflow.v1` / `fcb.json.v1`（內建但尚未定義）

兩者列在 `BUILTIN_STREAM_TYPES`（`evidence.rs:18`）但**還沒有記錄 schema**——spec 與 crate 皆未定義其
record schema，亦無對應的 frozen round-trip 測試（`stream_types.rs` 只凍 syslog）。

- `fcb.json.v1`：預期是「通用 JSON 物件（任意 CBOR map）」，作為沒有專屬 schema 時的萬用容器。
- `fcb.netflow.v1`：預期含五元組（src/dst IP+port + protocol）＋ bytes/packets ＋ 時間區間，但**尚未凍結**。

在 schema 定案前，這兩個 type 雖在 `BUILTIN_STREAM_TYPES` 內、`is_builtin` 會回 `true`，但消費端對其
記錄形狀**不應有任何假設**；待真正要用時再比照 §3.1 syslog 流程定義（spec requirement + `stream_types.rs`
round-trip 凍結）。此為已知缺口，見 §14。

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
| 0 | `magic` | 4 B | `[0x89, b'F', b'C', b'B']` = `89 46 43 42` | container.rs:22, 140 |
| 4 | `KIND` | 1 B | `1` = `.case`、`2` = `.casework` | container.rs:41-46, 141 |
| 5 | `container_version` | 2 B LE | `1`（`CONTAINER_VERSION`） | container.rs:25, 142 |
| 7 | `hdr_len` | 4 B LE | 明文 header 的 byte 長度 | container.rs:143 |
| 11 | `header` | `hdr_len` B | 明文 CBOR header（§5） | container.rs:144 |
| 11+`hdr_len` | `payload` | 其餘 | `AEAD(zstd(明文 payload))`，verbatim 寫入 | container.rs:145 |

固定前綴是 **11 bytes**（4+1+2+4），其後接 header 與 payload。`0x89` 仿 PNG，偵測 7/8-bit 傳輸損壞並
避免文字碰撞；`container_version` 是**獨立欄位、不**烤進 magic（`container.rs:19-21`）。

**golden vector 佐證（byte-exact，`vectors.rs:28-29`）：**

```text
.case：     89 46 43 42 | 01 | 01 00 | dc 01 00 00 | a8 …    （KIND=1, ver=1, hdr_len=0x1dc=476, header=map(8)）
.casework： 89 46 43 42 | 02 | 01 00 | 25 01 00 00 | a8 …    （KIND=2, ver=1, hdr_len=0x125=293, header=map(8)）
```

`.case` 總長 578 bytes（11 prefix + 476 header + 91 payload）；`.casework` 總長 423 bytes
（11 + 293 + 119）。兩者 header 都以 `a8`（CBOR map of 8）起頭，因為 `Header` 永遠是 8 個欄位（§5）。

**讀取／驗證行為（`peek_header`／`read_container`，`container.rs:151-214`）：**

- 前 4 bytes 非 magic → `FcbError::BadMagic`（**不**嘗試解密）。
- 缺 KIND → `Malformed("missing KIND")`；未知 KIND byte → `Malformed("unknown KIND byte {other}")`
  （placeholder 名 `{other}`，逐字對齊 `container.rs:52`）。
- `header.min_reader > READER_VERSION(=1)` → `UnsupportedVersion { min_reader, supported }`（優雅拒絕，
  不吐部分資料）。
- `hdr_len` 越界 → `Malformed("header length out of bounds")`；header CBOR 壞 →
  `Malformed("bad header CBOR: ...")`。
- `container_version` 目前**讀但不分派**（`peek_header` 直接 `let _container_version` 丟棄；
  `read_container` 保留但不驗證）：明文註解標「reserved for future parse-path dispatch; v1 is the only
  known layout today」（`container.rs:205-206`），即使 `container_version != 1` 也不報錯、照 v1 解析。
- `peek_header` **不複製 payload**，可在無 passphrase 下讀得 `case_id`／`kdf.salt`／`aead.nonce`
  （測試 `container.rs:282-291`）。

---

## 5. 明文 header 欄位 + CBOR 編碼規則

`Header` 是 `#[derive(Serialize, Deserialize)]` struct（`container.rs:83-101`），無 `#[serde(rename)]`／
`rename_all`，故 ciborium 把它編成 **以欄位名為 text key 的 CBOR map、順序 = 宣告順序**。共 8 欄 → `a8`。

| # | 欄位 | Rust 型別 | CBOR key（text） | 寫入值（生產端） | sourceRef |
|---|------|-----------|------------------|------------------|-----------|
| 1 | `header_schema_ver` | `u16` | `header_schema_ver` | `1` | container.rs:85-86；bundle.rs:71 |
| 2 | `min_reader` | `u16` | `min_reader` | `1` | container.rs:87-88；bundle.rs:72 |
| 3 | `case_id` | `String` | `case_id` | 由呼叫端帶入 | container.rs:89-90 |
| 4 | `bundle_hash` | `String` | `bundle_hash` | 由呼叫端帶入（§6） | container.rs:91-92 |
| 5 | `kdf` | `KdfParams` | `kdf` | 見下 | container.rs:93 |
| 6 | `aead` | `AeadParams` | `aead` | 見下 | container.rs:93-94 |
| 7 | `key_check` | `Vec<u8>` | `key_check` | KCV（32B，§10） | container.rs:95-98 |
| 8 | `meta` | `ciborium::value::Value` | `meta` | §1（case）／`{}`（work） | container.rs:99-100 |

**`KdfParams`（巢狀 map，5 欄 → `a5`，`container.rs:60-69`）：**

| # | 欄位 | 型別 | CBOR key | 寫入值 |
|---|------|------|----------|--------|
| 1 | `algo` | `String` | `algo` | `"argon2id"`（bundle.rs:59；`derive_key` 唯一驗的 algo） |
| 2 | `salt` | `Vec<u8>` | `salt` | 16 隨機 bytes |
| 3 | `m_cost` | `u32` | `m_cost` | Argon2 記憶體 KiB；預設 19456 |
| 4 | `t_cost` | `u32` | `t_cost` | Argon2 迭代；預設 2 |
| 5 | `p_cost` | `u32` | `p_cost` | Argon2 平行度；預設 1 |

**`AeadParams`（巢狀 map，2 欄 → `a2`，`container.rs:75-78`）：**

| # | 欄位 | 型別 | CBOR key | 寫入值 |
|---|------|------|----------|--------|
| 1 | `algo` | `String` | `algo` | `"xchacha20poly1305"`（bundle.rs:77；**描述性，open 不驗證**，見 §11） |
| 2 | `nonce` | `Vec<u8>` | `nonce` | 24 隨機 bytes |

### 5.1 ciborium 編碼慣例與陷阱（互通關鍵）

非 Rust 的 case builder／reader 重寫 codec 時必須對齊以下 ciborium 0.2 行為，否則無法 round-trip：

1. **`Vec<u8>` → CBOR array of uint（不是 byte string）。** `salt`／`nonce`／`key_check` 都是 `Vec<u8>`，
   ciborium 走 `serialize_seq` → 產出 **CBOR array（major type 4），每個 byte 編成一個 unsigned integer**，
   **不是** byte string（major type 2）。golden hex 佐證（`vectors.rs:28`）：
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
   | 0–23 | 單 byte `0x00 + n`（inline） | `1`（`header_schema_ver`／`min_reader`）→ `01`；`7`（severity Debug）→ `07` |
   | 24–255 | `0x18` + 1 byte | `32`（golden vector 測試 cost `m_cost`）→ `18 20`（`vectors.rs:52`）；`0x2f`（key_check 內某 byte）→ `18 2f` |
   | 256–65535 | `0x19` + 2 byte（**big-endian**） | **`19456`（production 預設 `m_cost`，`bundle.rs:16`）→ `19 4C 00`**；`65535` → `19 ff ff` |
   | 65536–2³²−1 | `0x1a` + 4 byte（big-endian） | `100000` → `1a 00 01 86 a0` |
   | 2³²–2⁶⁴−1 | `0x1b` + 8 byte（big-endian） | 罕見，僅超大 `records`／`pid` 才會用到 |

   > ⚠️ 機讀注意：CBOR 整數**不是** little-endian（container frame 的 `container_version`／`hdr_len`
   > 是 LE，但那是 frame 欄位、不走 CBOR）。CBOR 多 byte 計數一律 **big-endian**。golden vector 內唯一的
   > 整數 worked example 是測試 cost `m_cost=32`→`18 20`（`vectors.rs:52`）；production 預設 `m_cost=19456`
   > 落在 256–65535 區段，編成 `19 4C 00`（`0x4C00 = 19456`），照抄測試向量無法推得，特別標出。

   **CBOR text string（major type 3）的長度前綴編碼**（與上面 array／uint 計數規則並列，重建 byte-exact
   header 必備）：每個欄位名、enum variant 小寫字串、`case_id`／`algo`／`format` 等 text value 一律照此編碼，
   長度以 **UTF-8 byte 數**計（非字元數）：

   | text 長度（UTF-8 bytes） | 長度前綴 | 範例（golden vector，`vectors.rs:28`） |
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
   宣告序」只對 **struct** 成立。`header.meta`（型別是 `ciborium::value::Value`，`container.rs:99-100`）與
   syslog 記錄裡的 `sd`（型別是 `Value::Map`）**不是 struct**，是任意 `Value::Map`——ciborium 依其內部
   `Vec<(key, value)>` 的**插入順序**逐筆寫出，既不排序、也不套用宣告序規則。這直接決定 byte-exactness：
   - **`header.meta`**：golden vector 走 `cbor::to_value(&CaseMeta { streams, task })`（`vectors.rs:85`），
     `streams` 在前、`task` 在後的順序來自 `CaseMeta` struct 宣告序，解出來是 `a2` → `{streams, task}`。
     但若改用 `evidence::manifest_to_meta` + `task::task_to_meta` **合併**兩把 key（README 起手建議的路徑），
     合併後 `Value::Map` 的順序取決於**合併程式的插入序**、不再是 struct 宣告序——須**確保 `streams` 在
     `task` 之前**才能對齊 golden vector。
   - **syslog `sd`**：`Value::Map(vec![("ex@32473", Map(vec![("iut", "3")]))])`（`stream_types.rs:36-39`）
     的外層 SD-ID 與內層 param 順序，皆由建構 `vec` 的**插入序**決定。重建時須照原插入序排列。

3. **`#[serde(rename = "…")]`** 改單一欄位的 wire key：本 codec 只有 `StreamManifest.stream_type` →
   `"type"`（`evidence.rs:30-31`）。

4. **`#[serde(rename_all = "lowercase")]` enum → 小寫 text。** `ReportMode::Steps`／`Freeform` →
   `"steps"`／`"freeform"`（`task.rs:22-27`）。

5. **`Option` + `skip_serializing_if = "Option::is_none"` → `None` 省略整把 key。** 只有 library 的
   `TaskMeta.task` 用此（`task.rs:49-50`）；`None` 時 `task` key 完全不出現。`#[serde(default)]`（如
   `TaskSpec.steps`、`StreamsMeta.streams`）則讓「缺 key」解成預設值而非報錯。

> **CBOR map 內 entry 的順序由「結構決定」、非排序**：ciborium 依宣告順序寫出，重現 byte-exact golden
> vector 的關鍵就是欄位宣告順序不能變。golden vector 測試 `case_vector_is_byte_stable` 一旦順序漂移即
> 失敗（"case format drifted"，`vectors.rs:106-109`）。

---

## 6. `.casework` 的 payload（Submission）

`.casework`（KIND=2/work）由**消費端**產生（`submission::pack_submission`，`submission.rs:43-52`）、
**教師審閱平台**讀取（`open_submission`，`submission.rs:55-61`，非 work KIND 會被拒）。其 `header.meta`
為**空 map** `{}`（`Value::Map(vec![])`，`submission.rs:49`）；payload 是 `Submission` 的 CBOR：

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
| `case_id` | `String` | `case_id` | 否 | submission.rs:28 |
| `bundle_hash` | `String` | `bundle_hash` | 否 | submission.rs:30 |
| `student` | `Student { id, name }` | `student` | 否（typed） | submission.rs:31, 17-21 |
| `notes` | `Vec<Value>` | `notes` | **是**（每元素為不透明 CBOR） | submission.rs:33 |
| `report` | `Value` | `report` | **是**（單一不透明 CBOR；steps 為陣列、freeform 為字串） | submission.rs:35 |
| `activity` | `Vec<Value>` | `activity` | **是**（每元素為不透明 CBOR） | submission.rs:37 |
| `exported_at` | `String` | `exported_at` | 否 | submission.rs:39 |

`notes` / `report` / `activity` 在 container 層是**不透明 CBOR**——schema 由 browser workbench 擁有，
case builder **不需要**關心（case builder 只產 `.case`）。所有 7 欄皆**無** serde default → 解碼時全必填。
`Submission` derive `PartialEq` 但**不** derive `Eq`（因含 `Value`，`submission.rs:25`）；`Student` 則
derive `Eq`。

**KIND-gating：** `open_submission` 對 `kind != BundleKind::Work` 回
`FcbError::Malformed("not a .casework (KIND != work)")`（`submission.rs:57-59`，確切字串）；測試
`opening_a_case_as_submission_is_rejected` 證實把 `.case` 當 submission 開會被拒。

> `case_id` / `bundle_hash` **同時存在於明文 header 與 payload `Submission` 內**（`pack_submission` 把
> 兩者複製進 header params，`submission.rs:47-48`）：教師平台可不解密、先讀 header 取得綁定資訊，解密後
> 再用 payload 內的值交叉驗證。

> ⚠️ **`Submission` 的 on-disk byte 順序／編碼目前沒有 golden vector 凍結。** 唯一 byte-stable 的 WORK
> 向量 `FROZEN_WORK_HEX` 其實是用 **test-local 的 3 欄 `WorkPayload { case_id, bundle_hash, report }`**
> （`vectors.rs:40-45, 96-104`）建出來的，**不是** library 的 7 欄 `Submission`——它凍結的是 container
> frame（KIND=2／空 meta `a0`／crypto 管線）與這 3 欄 stand-in，**不**釘住 `Submission` 的欄位順序或
> 編碼。真正的 7 欄 `Submission` 只有 random-salt 的往返測試 `submission_random_round_trip`
> （`vectors.rs:175-189`）覆蓋——只證 round-trip、不證 byte-stability。非 Rust 重實作者若要驗 `Submission`
> 的 byte-exactness，須自行補向量；目前不能拿 `FROZEN_WORK_HEX` 當 `Submission` 的位元組依據。

---

## 7. Binding（綁定）

來源：`binding.rs`。

```text
verify_binding(work_case_id, work_bundle_hash, case_id, case_bundle_hash) -> BindingCheck   // binding.rs:37-50
  = Match                     // 同 case、同證物版本
  | CaseMismatch              // 根本是別的 case
  | EvidenceVersionMismatch   // 同 case，但證物被重新發版

work_key(case_id) = "fcb:work:{case_id}"                    // binding.rs:54-56；本機（IndexedDB）作品分庫鍵
compute_bundle_hash(bytes) = "sha256:" + lower_hex(SHA256(bytes))   // binding.rs:14-22
```

**`verify_binding` 判斷順序（case 身分優先於證物版本）：**

1. `work_case_id != case_id` → `CaseMismatch`（`binding.rs:43-44`）。
2. 否則 `work_bundle_hash != case_bundle_hash` → `EvidenceVersionMismatch`（`binding.rs:45-46`）。
3. 否則 → `Match`（`binding.rs:47-48`）。

`BindingCheck` derive `Debug, Clone, Copy, PartialEq, Eq`（`binding.rs:25`），三個 variant。case_id
**先於** bundle_hash 檢查——不同 case 永遠不會回報版本不符。

**`compute_bundle_hash` 格式：** 字面前綴 `"sha256:"`（7 chars）＋ 32-byte SHA-256 digest 的**小寫**
zero-padded hex（`{b:02x}`），輸出固定 **71 chars**（7 + 64，測試 `binding.rs:70` 斷言）。內容可定址：同
輸入同 hash、異輸入異 hash。

**`work_key`：** 純 ASCII text `fcb:work:` ＋ case_id 原樣（無 hashing／escaping）；不同 case_id 產不同
key（作品隔離，`binding.rs:90-94`）。實體 IndexedDB store 屬消費端範疇、不在 `binding.rs`。

**建議 `bundle_hash` 定義**（codec **不**強制）：
`bundle_hash = compute_bundle_hash(.case 的明文 payload bytes)`，即 §2 信封壓縮／加密**前**的序列化位元組。
如此同一份證物的 hash 穩定，學生作品才能可靠綁回特定證物版本；證物一改版，舊作品開啟時即可用
`EvidenceVersionMismatch` 提示。

> ⚠️ 此「`bundle_hash` 應等於明文 payload bytes 的 hash」是**慣例、未在程式碼強制**：`compute_bundle_hash`
> 接受任意 bytes，`binding.rs`／`submission.rs` 不會重算或驗證它對得上 payload。golden vector 用假值
> `"sha256:deadbeef"`（`vectors.rs:63, 99`）。正規定義尚未凍結，見 §14。

---

## 8. 答案安全不變量

來源：`task.rs`。學生端會**解密整個 `.case`**，所以 `.case` 內**任何**東西學生都看得到。因此：

- **`.case` 裡零答案／零評分標準／零步驟解答**；答案只留在教師母版與審閱平台。
- 型別上 `TaskStep` 根本沒有答案欄位（`task.rs:30-36`），解碼經過 typed model 會把任何夾帶的答案欄位
  **丟掉**——因為沒有欄位能裝它。
- 防呆：`FORBIDDEN_ANSWER_KEYS = ["answer", "answer_key", "rubric", "solution", "expected"]`（5 個，
  `task.rs:17-18`）；`contains_answer_fields(value)`（`task.rs:67-79`）遞迴檢查解出的 task 是否仍含這些
  key（消費端可 assert）。遞迴只走 `Value::Map`／`Value::Array`；scalar leaf 回 `false`；forbidden-key
  比對只認 `Value::Text` key。
- 測試 `answer_fields_are_stripped_on_decode`（`task.rs:103-130`）證實：含夾帶 `answer` 的髒 map 被偵測
  （`true`），解成 `TaskSpec` 後丟掉，再編碼即回 `false`。

case builder 設計守則：把答案／rubric 放在**不會進 `.case`** 的教師母版資料結構；打包 `.case` 時只輸出
`TaskSpec`（prompt + answer_type）。

---

## 9. Crypto／compress 管線（內層結構如何被封裝）

§2／§6 的明文 payload 經 **compress-then-encrypt** 變成 container 的 payload 區（`compress.rs`、
`crypto.rs`、`bundle.rs`）。本節是讓本檔自洽的摘要；完整論述見 [`fcb-wire-format.md`](./fcb-wire-format.md)。

```text
pack：   plaintext --(zstd Fastest)--> zstd frame --(XChaCha20-Poly1305, no AAD)--> ciphertext   // compress.rs:42-45
open：   ciphertext --(AEAD open，先 KCV 檢查)--> zstd frame --(zstd decompress)--> plaintext     // compress.rs:48-56
```

**順序不變量：先壓縮、後加密**（`compress.rs:42-56`；測試 `order_is_compress_then_encrypt` 證實解密後
得到的 inner 前 4 bytes == `ZSTD_MAGIC`、outer 前 4 bytes != `ZSTD_MAGIC`）。**禁止** encrypt-then-compress。

`pack_bytes`（`bundle.rs:57-84`）逐步：① 16-byte 隨機 salt → ② 24-byte 隨機 nonce → ③ 組 `KdfParams`
（algo `"argon2id"` + salt + cost）→ ④ `derive_key` 推 32-byte key → ⑤ 算 `key_check`（KCV，加密前算）
→ ⑥ `pack_payload`（compress-then-encrypt）→ ⑦ 組 `Header` → ⑧ `write_container`。salt/nonce 每次
`pack_bytes` 都用 `getrandom` 重新產生。

`open_bytes`（`bundle.rs:88-98`）：`read_container` → `derive_key(passphrase, header.kdf)` →
`unpack_payload(key, header.key_check, header.aead.nonce, payload)`。

---

## 10. KDF / KCV / AEAD 常數與行為

| 項目 | 值／行為 | sourceRef |
|------|----------|-----------|
| KDF 演算法 | Argon2id（`Algorithm::Argon2id`） | crypto.rs:34 |
| Argon2 version | `Version::V0x13`（= Argon2 v1.3 / 0x13） | crypto.rs:34 |
| 輸出長度 | 32 bytes（`KEY_LEN`，雙重綁定：`Some(32)` + 32-byte buffer） | crypto.rs:21, 32, 35 |
| 預設 cost | `m_cost=19456`（KiB）／`t_cost=2`（迭代）／`p_cost=1`（平行度） | bundle.rs:16-18 |
| salt 長度 | 16 bytes（隨機） | bundle.rs:19 |
| algo 驗證 | `derive_key` **只**驗 `kdf.algo == "argon2id"`，否則 `Malformed("unsupported KDF: {algo}")` | crypto.rs:29-31 |
| KCV 公式 | `key_check = SHA256(KCV_DOMAIN ‖ key)` = `SHA256(b"FCB-key-check-v1" ‖ key)`，32 bytes（domain 在前、key 在後） | crypto.rs:25, 43-48 |
| KCV 用途 | 開封時先 `ct_eq(KCV(key), header.key_check)` 區分錯密碼 vs 竄改 | crypto.rs:83-93 |
| AEAD | XChaCha20-Poly1305，nonce **24 bytes**、**無 AAD** | crypto.rs:23, 65-79 |
| AEAD algo 驗證 | `aead.algo`（`"xchacha20poly1305"`）寫入 header 但開封時**從不驗證**，`seal`/`open` 完全忽略 | bundle.rs:77；crypto.rs（無讀取） |
| 壓縮 | ruzstd 0.8（純 Rust，native == wasm），編碼僅 `CompressionLevel::Fastest`；標準 zstd frame magic `28 B5 2F FD` | compress.rs:6-10, 21, 29-31 |

**錯密碼 vs 竄改的分流（`open_payload`，`crypto.rs:83-93`）：**

```text
若 !ct_eq(KCV(key), expected_kcv)        → Err(WrongPassphrase)   // KCV 不符 = 錯密碼
否則 AEAD decrypt（tag 不符／被竄改）    → Err(Corrupt)
```

- KCV 比對用 hand-rolled constant-time `ct_eq`（長度不同立刻回 `false`、否則 XOR 累加無提前退出，
  `crypto.rs:96-105`）。
- nonce 長度 ≠ 24 → `Malformed("nonce must be 24 bytes, got N")`（`crypto.rs:54-62`）。
- 壞 zstd frame 解壓失敗 → `Corrupt`（`compress.rs:34-39`）。

> ⚠️ **`aead.algo` 不被驗證**（相對地 `kdf.algo` **會**驗）：即使 header 把 `aead.algo` 寫成別的字串，
> `open` 仍照 XChaCha20-Poly1305 解。非 Rust reimplementation 不能靠這個欄位選 AEAD。

---

## 11. 安全特性（明確界線）

- **AEAD 只認證 payload、無 AAD**：`seal`／`open` 只傳 `(nonce, payload)`（`crypto.rs:68, 77`）。因此
  **明文 header（含 `case_id`／`bundle_hash`／`meta`／manifest／task）未被 AEAD 認證**——可被竄改而
  不觸發解密失敗。
- **明文 header 是刻意的**：salt／cost／nonce 必須在還沒 key 之前就讀到（`container.rs:12-13`）。
- **`bundle_hash` 涵蓋範圍由生產端負責、codec 不驗證**：`compute_bundle_hash` 對任意 bytes 算 hash，
  `binding.rs`／`submission.rs` 不重算、不比對它與 payload 的關係（§7）。若要讓 header 的 `bundle_hash`
  能反映證物內容，須由 case builder 自律地用「明文 payload bytes」算（§7 建議），並理解這層保證在 codec
  外、非 AEAD 強制。
- **錯密碼與竄改可區分且皆非靜默**：`WrongPassphrase`（KCV 不符）vs `Corrupt`（KCV 符但 AEAD/zstd 失敗），
  兩者都**不**吐部分／損壞資料（§10、§12）。
- **答案安全**靠 typed model 結構性保證（§8），非靠加密——學生本來就能解密整包。

---

## 12. Error 語意

`FcbError`（`#[derive(Debug, Error, PartialEq, Eq)]`，`error.rs:8-9`）五個 variant：

| Variant | `#[error]` 文字 | 語意 | 觸發點（摘） | sourceRef |
|---------|-----------------|------|--------------|-----------|
| `BadMagic` | `"not an FCB container (bad magic)"` | 前 4 bytes 非 FCB magic | `peek/read_container` 入口 | error.rs:11-12；container.rs:153, 182 |
| `UnsupportedVersion { min_reader, supported }` | `"unsupported FCB version: …"` | bundle 要求的 reader 比本 reader 新 | `min_reader > READER_VERSION(=1)` | error.rs:14-16；container.rs:169-173 |
| `Malformed(String)` | `"malformed FCB container: {0}"` | 結構性無效 | 壞長度前綴、未知/缺 KIND、header 越界、壞 header CBOR、未知 KDF algo、nonce 長度錯、`payload missing stream {id}` 等 | error.rs:19-20；container/crypto/evidence 多處 |
| `WrongPassphrase` | `"wrong passphrase"` | KCV 不符（錯密碼） | `crypto.rs:89-91` | error.rs:23-24 |
| `Corrupt` | `"corrupt or tampered bundle"` | KCV 符但 AEAD/zstd 失敗（竄改）；或 payload 層 `cbor::decode` 失敗 | crypto.rs:74-78；compress.rs:35-37；cbor.rs:38-40 | error.rs:27-28 |

> 注意分流：`cbor::decode`（**解密/解壓後**的 payload CBOR 解碼）失敗映射成 **`Corrupt`**（`cbor.rs:39`），
> 而 header／meta 層的 CBOR 失敗（`to_value`／`from_value`／`encode`）映射成 **`Malformed`**
> （`cbor.rs:17, 18, 25, 26, 33`——皆為 `map_err` 那幾行）。`WrongPassphrase` 與
> `Corrupt` 刻意分開——對學生／operator 意義不同（密碼錯 vs 檔案被竄改，`error.rs:5-7`）。

---

## 13. 端到端打包流程（end-to-end，可照做）

以 `.case` 為例，case builder 從零組一份 bundle 的步驟（目前須手組信封，因無 `pack_case` helper，§14）：

1. **整理 manifest**：對每條 stream 建 `StreamManifest { id, type, records }`（記得 CBOR key 是 `"type"`）。
2. **整理 task**（可選）：建 `TaskSpec { report_mode, instructions, steps }`，**不含任何答案欄位**（§8）。
3. **組 meta**：`meta = { "streams": [...manifest], "task": <TaskSpec> }`；可用
   `evidence::manifest_to_meta` 產出 `{streams}`，再合併 `task::task_to_meta` 的 `{task}`，或直接照
   `CaseMeta { streams, task }` 形狀組（注意 library `task_to_meta` 在 `None` 時省略 `task`，§1）。
4. **組 payload 信封**：`payload = { "streams": [ StreamData { id, records }, ... ] }`，`records` 依各
   stream type 的 schema（syslog 見 §3.1）。`cbor::encode` 成明文 payload bytes。**此處無 crate helper，
   須自組**（§14）。
5. **算 `bundle_hash`**（建議）：`compute_bundle_hash(明文 payload bytes)`（§7），得 `"sha256:…"`。
6. **打包**：`BundleParams::new(BundleKind::Case, case_id, bundle_hash, meta)`（用預設 Argon2 cost），
   呼叫 `bundle::pack_bytes(&params, &payload_bytes, passphrase)`。內部會壓縮→加密→組 container frame
   （§9）。
7. **產出** `.case` bytes（`89 46 43 42 01 …`）。

`.casework` 則直接用 `submission::pack_submission(&Submission, passphrase)`（meta 自動為 `{}`，
`submission.rs:43-52`），不需手組信封。

開封：`bundle::open_bytes(bytes, passphrase)` → `(kind, header, payload_bytes)`；再
`cbor::decode::<CasePayload>(payload_bytes)` 取 `streams`，配 `manifest_from_meta(&header.meta)` 餵
`decode_streams` join 出 `DecodedStream`。`.casework` 用 `open_submission`（KIND-gated）。

---

## 14. 已知缺口（實作 case builder 前先看）與 Non-Goals

**已知缺口（Known Gaps）：**

1. **沒有 `.case` payload 信封 helper（`pack_case`）。** crate 有 `StreamData` 與 `decode_streams`（讀），
   但沒有「組 `{ streams: [...] }` 並 `bundle::pack_bytes`」的 `pack_case`（寫）。`CasePayload` 只存在於兩個
   test 檔（`vectors.rs:36-39`、`stream_types.rs:18-21`），源碼樹無此型別。建議在 `evidence.rs` 補
   `CasePayload { streams }` + `pack_case(...)`，讓生產／消費共用同一份序列化。
2. **沒有 `bundle_hash` 的正規定義 helper。** `compute_bundle_hash` 接受任意 bytes；要固定成「明文 payload
   bytes」（§7 建議）並包成 helper，避免生產端各算各的。正規定義**尚未凍結**。
3. **`fcb.netflow.v1` / `fcb.json.v1` 記錄 schema 未定義。** 兩者在 `BUILTIN_STREAM_TYPES` 內、`is_builtin`
   回 `true`，但 spec／crate／測試皆未凍結其 record schema（§3.2）。
4. **WASM 綁定僅 `fcb_version`。** `crates/fcb/src/wasm.rs` 只導出 `fcb_version()`（回 `CARGO_PKG_VERSION`），
   尚無 `openBundle`／`packSubmission` 等 richer binding（`wasm.rs:6-13`）。
5. **plugin parser registry 未實作。** `DecodedStream` 的註解提到 `is_builtin = false` 可落到「a registered
   plugin」（`evidence.rs:50`），但本 crate **沒有**任何 registry 程式碼——plugin registry 純屬消費端概念
   （另見下方 Non-Goals）。
6. **payload 多餘 stream 的行為無測試斷言。** payload 含 manifest 未列的 stream 會被**靜默忽略**（§2 不變量，
   迭代由 manifest 驅動，`evidence.rs:78-92`），但此行為**沒有專屬測試**——是否刻意如此**未證實**。
7. **`Submission` 無 byte-stability 凍結。** golden WORK 向量凍結的是 test-local 3 欄 `WorkPayload`，**非**
   library 的 7 欄 `Submission`（詳見 §6）；目前無向量釘住 `Submission` 的 on-disk 位元組。

**Non-Goals（本檔／本層不負責）：**

- **不**定義 `notes`／`report`／`activity` 的 schema——那是 browser workbench 的範疇（§6 不透明）。
- **不**定義 plugin parser registry 的執行機制——`is_builtin = false` 的 fallback 是消費端概念，本 crate
  未實作。
- **不**規定 `bundle_hash` 必須涵蓋哪些 bytes（codec 不驗證，§7／§11）；亦**不**靠 AEAD 保護明文 header。
- 實體 IndexedDB／儲存層**不**在 binding 範疇（`work_key` 只給 key 字串）。

以上缺口建議「回頭在 `fcb` crate 補 helper + 定 schema」，而非只在 case builder 端自幹——這樣 browser
端、case builder、教師平台三方共用同一份真實程式碼，從根本杜絕格式漂移。

---

## 15. 相依套件版本表

來源：`crates/fcb/Cargo.toml`（版本字串為 Cargo semver caret 範圍，如 `"0.2"` = `^0.2`；實際解析 patch
版需看 `Cargo.lock`）。

| 套件 | 版本 | features | 用途 | sourceRef |
|------|------|----------|------|-----------|
| `serde` | `1` | `["derive"]` | 序列化 derive | Cargo.toml:12 |
| `ciborium` | `0.2` | — | CBOR 編解碼（§5.1 慣例由它決定） | Cargo.toml:13 |
| `argon2` | `0.5` | — | Argon2id KDF（§10） | Cargo.toml:14 |
| `chacha20poly1305` | `0.10` | — | XChaCha20-Poly1305 AEAD（§10–§11） | Cargo.toml:15 |
| `ruzstd` | `0.8` | — | 純 Rust zstd（native==wasm，僅 Fastest，§9） | Cargo.toml:16 |
| `sha2` | `0.10` | — | SHA-256（KCV、`bundle_hash`） | Cargo.toml:17 |
| `thiserror` | `2` | — | `FcbError` derive（§12） | Cargo.toml:18 |
| `getrandom` | `0.2` | wasm 加 `["js"]` | 隨機 salt/nonce | Cargo.toml:19（base dep）, 23（wasm target dep） |
| `hex`（dev） | `0.4` | — | golden vector hex 比對 | Cargo.toml:26 |
| `wasm-bindgen`（wasm） | `0.2` | — | WASM 入口 | Cargo.toml:22 |
