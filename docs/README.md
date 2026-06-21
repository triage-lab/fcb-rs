# FCB 協定開發文件

本目錄是 **FCB（Forensic Case Bundle）** 協定的開發者文件入口。讀者主要是要實作 FCB
**生產端／消費端**的人，尤其是 **case builder**（建構器，負責出題、把證物與題目打包成 `.case`）。
本目錄說明如何產生／讀取與 `crates/fcb` 參考實作 **byte-compatible** 的 `.case` 與 `.casework`。

> **術語**：產生 `.case` 的工具一律稱 **case builder**（建構器）。它**不一定**是 CLI——可以是函式庫、
> 服務、批次腳本或內嵌在某個編輯流程裡的程式，所以不要叫它「encoder CLI」或「CLI」。消費端則泛稱
> **消費端**，必要時直接點名 **browser workbench**（學生用的瀏覽器調查台）或 **教師審閱平台**。

---

## 這份文件給誰看 / 怎麼讀（雙受眾）

FCB 文件刻意拆成「給人讀的導讀」與「給機器解析的精確規格」兩種。挑對的進入點，可以省下大量來回：

| 你的處境 | 先讀哪份 | 為什麼 |
|----------|----------|--------|
| 我想先**理解** FCB 是什麼、為什麼這樣設計 | `fcb-wire-format.md` + `fcb-data-model.md`（人類友善導讀） | 有白話說明、圖、實例與「為什麼」，幫你建立心智模型。 |
| 我要**手刻** codec（非 Rust）、需要無歧義的欄位表／CBOR 佈局／不變量 | `fcb-reference.md`（機器可解析的權威規格） | 把所有欄位、型別、CBOR marker、邊界條件整理成可逐項對照、可機器解析的表，避免從散文反推格式。 |
| 我用 **Rust** 寫 case builder | 直接相依 `fcb` crate（見下方「起手建議」） | 直接呼叫參考實作，零 CBOR 漂移風險；文件只在你要驗證或除錯時才需要。 |
| 我在做**消費端**（workbench／審閱平台），只要會讀 | `fcb-data-model.md`（資料模型 + binding） | 著重 manifest／stream type 派發／答案安全／binding，是讀取端真正在意的。 |

> 經驗法則：**先用導讀建立理解，再用 `fcb-reference.md` 對齊每一個 byte。** 散文導讀為了好懂可能略過
> 邊角細節；任何精確宣稱（欄位順序、CBOR marker、邊界行為）都以 `fcb-reference.md` 與其引用的原始碼／
> golden vector 為準。

---

## 文件清單

| 檔案 | 受眾 | 內容 |
|------|------|------|
| [`fcb-wire-format.md`](./fcb-wire-format.md) | 人類導讀 | **外層信封**：container 位元組佈局（magic / KIND / `container_version` / `hdr_len` / header / payload）、passphrase 密碼學（Argon2id + XChaCha20-Poly1305 + KCV）、compress-then-encrypt 管線、CBOR 編碼規則與互通陷阱、端到端打包流程。 |
| [`fcb-data-model.md`](./fcb-data-model.md) | 人類導讀 | **內層資料結構**：header `meta`（stream manifest + task spec）、`.case` payload 信封、各 stream type 的記錄 schema（含 `fcb.syslog.v1`、ECS 對照、演進規則）、`.casework`（Submission）、binding 與答案安全不變量。 |
| [`fcb-reference.md`](./fcb-reference.md) | 機器可解析的權威規格 | 把上述兩份散文導讀的內容整併成**單一、無歧義、可逐項機器解析**的參考：完整欄位表、Rust 型別↔CBOR 對映、ciborium 慣例、常數值、error 語意、不變量與 golden-vector 出處。手刻 codec 或寫驗證工具時以此為準。 |

> **三份 docs 已互相對齊。** `fcb-reference.md` 為機器可解析的精確規格鏡像層（欄位表／CBOR marker／
> 常數／error 目錄／不變量總表皆已齊備）；`fcb-wire-format.md` 與 `fcb-data-model.md` 為人類導讀。
> 三份內容若有出入，先以 `fcb-reference.md` 與其引用的原始碼／golden vector 為準，再回到下方「權威來源
> 優先序」逐項核對。

---

## 權威來源優先序（衝突時以前者為準）

> 本節（權威來源優先序）與下節（7 個 capability）是整套 docs 的**單一權威來源**。三份 narrative／reference
> 檔不再重述整塊，只會一行指回這裡；要核對優先序或 capability 範圍時一律回本節為準。

任何 byte／數值／行為宣稱，最終都要能對應到某個具體出處。發生衝突時，**永遠以排序在前者為準**：

1. **`crates/fcb/src/*.rs` 參考實作** — 真相的最終來源。任何文件與原始碼牴觸，以原始碼為準。
2. **`crates/fcb/tests/vectors.rs` 與 `crates/fcb/tests/stream_types.rs`** — byte-exact golden vectors
   與 round-trip 契約。`FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` 鎖死 on-disk 位元組；`stream_types.rs`
   鎖死 `fcb.syslog.v1` 記錄欄位集。任何 FCB 實作都必須能解這些 vector，重建時必須產生**相同位元組**。
3. **`openspec/specs/fcb-*` 與其餘 capability spec**（共 **7 個 capability**，見下節）— 正規行為契約。
4. **本目錄 `docs/`** — 精修對象，**不是真相來源**。補上 specs 未涵蓋的 byte-level／crypto／schema 細節，
   並把分散的事實彙整成可照做的導讀與規格。文件若與上面三層不一致，是文件要修。

### 7 個 capability spec（`openspec/specs/`）

`openspec/specs/` 下共有 **7 個 capability**，其中 **5 個是 `fcb-*`**（FCB codec 直接相關），另 **2 個是
消費端協定**（不在 codec 範圍，但同屬 FCB 生態）：

| Capability | 類別 | 範圍 |
|------------|------|------|
| `fcb-container-format` | fcb-* | 外層 container：magic／KIND／多層版本與優雅拒絕／明文 header／compress-then-encrypt／passphrase 密碼學。 |
| `fcb-evidence-model` | fcb-* | 自描述 typed streams、manifest、namespaced stream type、未知 type 的優雅退場（generic fallback）。 |
| `fcb-task-spec` | fcb-* | 內嵌 task 定義（`report_mode` steps／freeform、`steps`）與**答案安全不變量**（學生 build 零答案）。 |
| `fcb-submission` | fcb-* | `.casework`（KIND=work）格式、student identity、以 `case_id` + `bundle_hash` 做 case binding。 |
| `fcb-stream-types` | fcb-* | `fcb.syslog.v1` 記錄 schema、演進／相容規則、worked examples（RFC 5424／3164／minimal）。 |
| `plugin-protocol` | 非 fcb（消費端） | 消費端 plugin 介面（parser／view／query-engine／tool／ai-provider 等 kind 與 manifest）。**不影響 `.case` 位元組格式**。 |
| `query-model` | 非 fcb（消費端） | 消費端查詢模型（pipeline AST 作為查詢契約）。**不影響 `.case` 位元組格式**。 |

> case builder 作者主要相關的是 **5 個 `fcb-*`**；`plugin-protocol` 與 `query-model` 屬消費端如何
> **呈現／查詢**證物，與「怎麼把 `.case` 打包成正確位元組」無關。
>
> 注意：目前各 spec 的 `## Purpose` 段仍是 archive 自動產生的 `TBD` 佔位字串（尚未填寫），但其
> `## Requirements` 段已是正式契約。

---

## 給 case builder 作者的起手建議

### 用 Rust 寫：直接相依 `fcb` crate

> **Rust 寫的 case builder 直接相依 `fcb` crate**（它已是 `crate-type = ["cdylib", "rlib"]`），
> 走參考實作的公開路徑組 bundle，零 CBOR 漂移風險：
>
> - 打包：`bundle::pack_bytes(&BundleParams, payload, passphrase)`（隨機 salt/nonce、compress-then-encrypt）。
> - 組 header `meta`：`evidence::manifest_to_meta(&[StreamManifest])` 出 `{ streams: [...] }`，
>   `task::task_to_meta(&TaskSpec)` 出 `{ task: ... }`，兩者合併成 `.case` 的 `meta`。
>   **⚠️ 合併時 `streams` 必須排在 `task` 之前**，否則 CBOR map 的 key 順序與 golden vector
>   不 byte-exact（golden 用的 `CaseMeta { streams, task }` 宣告序就是 streams→task，見
>   `crates/fcb/tests/vectors.rs:31-35,85`）；詳見 [`fcb-wire-format.md`](./fcb-wire-format.md) §2 規則 2b。
>   兩個 helper 各只產一把 key（`evidence::manifest_to_meta` → `evidence.rs:61-65`、
>   `task::task_to_meta` → `task.rs:54-56`），合併序由你的呼叫端決定，crate 不幫你排。
> - 答案安全防呆：`task::contains_answer_fields(&value)` 可在打包前 assert 沒夾帶答案 key。
>
> 注意 `lib.rs` **沒有**頂層 `pack`/`open` re-export，只 re-export `error::{FcbError, Result}`；
> 高階函式都在子模組（`bundle::`、`evidence::`、`task::`、`submission::`、`binding::`），走模組路徑呼叫。
> 所以你的 `use` 大致長這樣：
>
> ```rust
> use fcb::bundle::{self, BundleParams};
> use fcb::container::BundleKind;
> use fcb::evidence::{manifest_to_meta, StreamData, StreamManifest};
> use fcb::task::{task_to_meta, ReportMode, TaskSpec, TaskStep};
> use fcb::cbor;
> ```

### 可直接照抄的最小 `.case` 打包範例（Rust）

> `fcb::case::pack_case` 是 `.case` 的權威產出 helper：給它 manifest、選用 task 與
> `CasePayload { streams }`，它會以 **canonical 序列化**算出 `bundle_hash`、組
> `{ streams, task? }` header meta、產生隨機 salt/nonce 並用預設 Argon2id cost 封裝。生產端與
> 消費端（含 WASM bridge）共用同一個公開 `CasePayload` 信封，從根本杜絕格式漂移。
>
> ```rust
> use ciborium::value::Value;
> use fcb::case::{pack_case, CaseInput, CasePayload};
> use fcb::evidence::{StreamData, StreamManifest};
>
> fn pack_minimal_case() -> Vec<u8> {
>     // 1) manifest：宣告每個 stream 的 id / type / 記錄筆數。
>     let manifest = vec![StreamManifest {
>         id: "s0".into(),
>         stream_type: "fcb.syslog.v1".into(),
>         records: 1,
>     }];
>     // 2) payload：{ streams: [StreamData{ id, records }] } 公開信封。
>     let payload = CasePayload {
>         streams: vec![StreamData {
>             id: "s0".into(),
>             records: vec![Value::Map(vec![
>                 (Value::Text("ts".into()), Value::Text("2026-01-01T00:00:00Z".into())),
>                 (Value::Text("host".into()), Value::Text("h1".into())),
>                 (Value::Text("msg".into()), Value::Text("hello".into())),
>             ])],
>         }],
>     };
>     // 3) pack_case：bundle_hash 由 canonical payload 自動算出；task=None 時 meta 不含 task。
>     let input = CaseInput { case_id: "case-demo".into(), manifest, task: None, payload };
>     pack_case(&input, "passphrase").unwrap()
> }
> ```
>
> 需要逐欄掌握底層 `bundle::pack_bytes` + `*_to_meta` 手組信封的版本，見
> `crates/fcb/tests/stream_types.rs:88-115`（含 RFC 5424／3164／minimal 三筆 worked example）。

### 用非 Rust 語言寫：照 `fcb-reference.md` 逐位元對齊

> 只有在用**非 Rust 語言**重寫 codec 時，才需要照 [`fcb-reference.md`](./fcb-reference.md)
> 逐位元對齊（需要白話脈絡時搭配 [`fcb-wire-format.md`](./fcb-wire-format.md) +
> [`fcb-data-model.md`](./fcb-data-model.md)）。**最關鍵的互通陷阱**：ciborium 把 `Vec<u8>`（`salt`／`nonce`／`key_check`）編成
> **CBOR array of uint**（major type 4），**不是** byte string（major type 2）——寫錯這點，header 就與
> 參考實作不相容。驗收標準很單純：產出的位元組能通過 `cargo test -p fcb`（特別是
> `case_vector_is_byte_stable` / `frozen_case_vector_decodes_to_expected_structure`），即視為相容。

### `.case` 產出：用 crate 的 `fcb::case`（生產／消費共用）

> `fcb::case` 模組提供兩個權威 helper，生產端與消費端（含 WASM bridge）共用，從根本杜絕格式漂移：
>
> 1. **`pack_case(&CaseInput, passphrase)`** — 組 `{ streams: [StreamData] }` 信封並封裝成 `.case`。
>    `CaseInput { case_id, manifest, task, payload }`；`CasePayload { streams }` 是唯一的公開信封型別
>    （golden vector、`stream_types.rs`、WASM bridge 皆重用，不再各自宣告 test-local 版本）。
> 2. **canonical `bundle_hash`** — `case::case_bundle_hash(&CasePayload)` = `compute_bundle_hash(canonical
>    明文 payload bytes)`，已**凍結**為「sha256(明文 payload bytes)」，並由 `pack_case` 自動帶入 header，
>    使同一份證物無論 salt/nonce 為何皆得相同 hash。回歸測試：`crates/fcb/tests/vectors.rs` 的
>    `case_canonical_bundle_hash_is_frozen`、`pack_case_round_trips_and_binds_hash`。

---

## 已知缺口（Known Gaps）

誠實列出**尚未實作／尚未凍結**的部分，以免高估現況：

1. **WASM 綁定僅 `fcb_version`。** `crates/fcb/src/wasm.rs` 只導出 `fcb_version()`，尚無
   `openBundle`／`packSubmission` 等 richer bindings（註：`fcb-wasm` bridge crate 已有較完整的
   native core，見 `crates/fcb-wasm/src/lib.rs`）。
2. **plugin registry 是消費端概念、本 crate 未實作。** `DecodedStream` 的註解提到未知 type 會落
   generic fallback「或交給 a registered plugin」（`crates/fcb/src/evidence.rs:50`），但 crate 內**沒有
   任何 registry 程式碼**；plugin 介面屬 `plugin-protocol`（消費端 spec），與 `.case` 位元組格式無關。
3. **payload 多出 manifest 未列的 stream，行為未測。** `decode_streams` 以 manifest 驅動迭代、用 `id`
   反查 payload；manifest 缺對應 payload → `Malformed`，但**反向**（payload 有 manifest 未宣告的多餘
   stream）會被靜默忽略，且**無任何測試斷言**這是刻意行為（`crates/fcb/src/evidence.rs:77-93`，未證實）。
   重寫 codec／自訂 case builder 時別依賴這個未凍結的行為。

> ✅ 已關閉（本批）：**`pack_case` / `CasePayload` 公開 helper**、**canonical `bundle_hash` 凍結**、
> **`fcb.netflow.v1` / `fcb.json.v1` 記錄 schema 凍結**——見上方「`.case` 產出」段、`fcb::case` 模組與
> `fcb-data-model.md §3.2/§3.3`。
>
> `fcb-reference.md §9` 是最細的缺口清單（共 3 項）；本表已與其對齊。
