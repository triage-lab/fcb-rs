# FCB 協定開發文件

這裡是 **FCB（Forensic Case Bundle）** 協定的開發者文件入口。會看這份文件的人，多半正要動手實作 FCB
的**生產端或消費端**，其中最核心的角色是 **case builder**（建構器）：負責出題，把證物與題目打包成一份 `.case`。
這份文件要回答的問題只有一個——怎麼產生、怎麼讀取，才能跟 `crates/fcb` 參考實作做到 **byte-compatible** 的 `.case` 與 `.casework`。

> **術語**：產生 `.case` 的工具，這份文件一律叫它 **case builder**（建構器）。要留意的是它**不一定**是 CLI。
> 它可能是一個函式庫、一支服務、一段批次腳本，甚至是內嵌在某個編輯流程裡的程式碼，所以請別把它寫成
> 「encoder CLI」或「CLI」。消費端則統稱 **消費端**，需要講得更具體時再直接點名 **browser workbench**
> （學生用的瀏覽器調查台）或 **教師審閱平台**。

---

## 這份文件給誰看 / 怎麼讀（雙受眾）

FCB 文件刻意拆成兩種：一種寫給人讀，叫導讀；另一種寫給機器解析，是精確規格。一開始挑對進入點，後面能省掉大量來回：

| 你的處境 | 先讀哪份 | 為什麼 |
|----------|----------|--------|
| 我想先**理解** FCB 是什麼、為什麼這樣設計 | `fcb-wire-format.md` + `fcb-data-model.md`（人類友善導讀） | 有白話說明、圖、實例與「為什麼」，幫你建立心智模型。 |
| 我要**手刻** codec（非 Rust）、需要無歧義的欄位表／CBOR 佈局／不變量 | `fcb-reference.md`（機器可解析的權威規格） | 把所有欄位、型別、CBOR marker、邊界條件整理成可逐項對照、可機器解析的表，避免從散文反推格式。 |
| 我用 **Rust** 寫 case builder | 直接相依 `fcb` crate（見下方「起手建議」） | 直接呼叫參考實作，零 CBOR 漂移風險；文件只在你要驗證或除錯時才需要。 |
| 我在做**消費端**（workbench／審閱平台），只要會讀 | `fcb-data-model.md`（資料模型 + binding） | 著重 manifest／stream type 派發／答案安全／binding，是讀取端真正在意的。 |

> 經驗法則：**先用導讀把整體弄懂，再翻 `fcb-reference.md` 去對齊每一個 byte。** 導讀是散文，為了好讀，
> 邊角細節有時會略過。所以只要牽涉到精確的宣稱，像是欄位順序、CBOR marker、邊界行為，一律以
> `fcb-reference.md` 以及它引用的原始碼、golden vector 為準。

---

## 文件清單

| 檔案 | 受眾 | 內容 |
|------|------|------|
| [`fcb-wire-format.md`](./fcb-wire-format.md) | 人類導讀 | **外層信封**：container 位元組佈局（magic / KIND / `container_version` / `hdr_len` / header / payload）、passphrase 密碼學（Argon2id + XChaCha20-Poly1305 + KCV）、compress-then-encrypt 管線、CBOR 編碼規則與互通陷阱、端到端打包流程。 |
| [`fcb-data-model.md`](./fcb-data-model.md) | 人類導讀 | **內層資料結構**：header `meta`（stream manifest + task spec）、`.case` payload 信封、各 stream type 的記錄 schema（含 `fcb.syslog.v1`、ECS 對照、演進規則）、`.casework`（Submission）、binding 與答案安全不變量。 |
| [`fcb-reference.md`](./fcb-reference.md) | 機器可解析的權威規格 | 把上述兩份散文導讀的內容整併成**單一、無歧義、可逐項機器解析**的參考：完整欄位表、Rust 型別↔CBOR 對映、ciborium 慣例、常數值、error 語意、不變量與 golden-vector 出處。手刻 codec 或寫驗證工具時以此為準。 |
| [`fcb-integration-guide.md`](./fcb-integration-guide.md) | 消費端 getting-started | 從**消費端**把 `fcb-rs` 接起來：Rust（Cargo git dep）與 WASM/JS 兩條路的可跑範例、error kind 處理、golden-vector 契約、teacher→student→platform 端到端流程。建在本三份權威 docs 之上、交叉連結。 |
| [`fcb-cookbook.md`](./fcb-cookbook.md) | 任務導向 recipes | 常見任務速查（收件驗 binding、偵測證物版本不符、解碼 stream type、用 golden vector 驗相容、分辨密碼錯 vs 竄改、不需密碼 peek）。每則給目標＋呼叫，交叉連結整合指南與 reference。 |

> **這三份 docs 已經彼此對齊。** `fcb-reference.md` 是機器可解析的精確規格鏡像層，欄位表、CBOR marker、
> 常數、error 目錄、不變量總表都已備齊；`fcb-wire-format.md` 與 `fcb-data-model.md` 則是寫給人讀的導讀。
> 萬一三份內容對不上，先以 `fcb-reference.md` 以及它引用的原始碼、golden vector 為準，再回到下方
> 「權威來源優先序」逐項核對。

---

## 權威來源優先序（衝突時以前者為準）

> 本節（權威來源優先序）和下一節（7 個 capability）合起來，是整套 docs 的**單一權威來源**。三份 narrative
> 與 reference 檔不再把這整塊重抄一遍，只用一行指回這裡；之後要核對優先序或 capability 範圍，一律回本節為準。

任何關於 byte、數值或行為的宣稱，最後都得能對應到某個具體出處。一旦發生衝突，**排序在前的永遠贏**：

1. **`crates/fcb/src/*.rs` 參考實作** — 真相的最終來源。文件跟原始碼一旦牴觸，以原始碼為準，沒有例外。
2. **`crates/fcb/tests/vectors.rs` 與 `crates/fcb/tests/stream_types.rs`** — byte-exact golden vectors
   與 round-trip 契約。`FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` 把 on-disk 位元組鎖死，`stream_types.rs`
   則鎖死 `fcb.syslog.v1` 的記錄欄位集。任何 FCB 實作都得能解開這些 vector，重建出來的位元組必須**一模一樣**。
3. **`openspec/specs/fcb-*` 與其餘 capability spec**（共 **7 個 capability**，見下節）— 正規的行為契約。
4. **本目錄 `docs/`** — 是精修對象，**不是真相來源**。它的工作是補上 specs 沒涵蓋到的 byte-level、crypto、
   schema 細節，再把散落各處的事實彙整成可以照著做的導讀與規格。文件跟上面三層對不上時，要修的是文件。

### 7 個 capability spec（`openspec/specs/`）

`openspec/specs/` 底下總共 **7 個 capability**。其中 **5 個是 `fcb-*`**，跟 FCB codec 直接相關；剩下 **2 個是
消費端協定**，雖然不在 codec 範圍內，但同樣屬於 FCB 生態：

| Capability | 類別 | 範圍 |
|------------|------|------|
| `fcb-container-format` | fcb-* | 外層 container：magic／KIND／多層版本與優雅拒絕／明文 header／compress-then-encrypt／passphrase 密碼學。 |
| `fcb-evidence-model` | fcb-* | 自描述 typed streams、manifest、namespaced stream type、未知 type 的優雅退場（generic fallback）。 |
| `fcb-task-spec` | fcb-* | 內嵌 task 定義（`report_mode` steps／freeform、`steps`）與**答案安全不變量**（學生 build 零答案）。 |
| `fcb-submission` | fcb-* | `.casework`（KIND=work）格式、student identity、以 `case_id` + `bundle_hash` 做 case binding。 |
| `fcb-stream-types` | fcb-* | `fcb.syslog.v1` 記錄 schema、演進／相容規則、worked examples（RFC 5424／3164／minimal）。 |
| `plugin-protocol` | 非 fcb（消費端） | 消費端 plugin 介面（parser／view／query-engine／tool／ai-provider 等 kind 與 manifest）。**不影響 `.case` 位元組格式**。 |
| `query-model` | 非 fcb（消費端） | 消費端查詢模型（pipeline AST 作為查詢契約）。**不影響 `.case` 位元組格式**。 |

> 對 case builder 作者來說，真正相關的是那 **5 個 `fcb-*`**。`plugin-protocol` 和 `query-model` 講的是
> 消費端怎麼**呈現或查詢**證物，跟「怎麼把 `.case` 打包成正確位元組」是兩回事。
>
> 一個小提醒：目前各 spec 的 `## Purpose` 段還是 archive 自動產生的 `TBD` 佔位字串，尚未填寫；但它們的
> `## Requirements` 段已經是正式契約了。

---

## 給 case builder 作者的起手建議

### 用 Rust 寫：直接相依 `fcb` crate

> **如果用 Rust 寫 case builder，最省事的做法就是直接相依 `fcb` crate**（它已經是
> `crate-type = ["cdylib", "rlib"]`）。走參考實作的公開路徑來組 bundle，CBOR 不會漂移：
>
> - 打包：`bundle::pack_bytes(&BundleParams, payload, passphrase)`（隨機 salt/nonce、compress-then-encrypt）。
> - 組 header `meta`：`evidence::manifest_to_meta(&[StreamManifest])` 出 `{ streams: [...] }`，
>   `task::task_to_meta(&TaskSpec)` 出 `{ task: ... }`，兩者合併成 `.case` 的 `meta`。
>   **⚠️ 合併時 `streams` 一定要排在 `task` 前面**，不然 CBOR map 的 key 順序就跟 golden vector 對不上、
>   做不到 byte-exact（golden 用的 `CaseMeta { streams, task }`，宣告序本來就是 streams→task，見
>   `crates/fcb/tests/vectors.rs:31-35,85`）；細節見 [`fcb-wire-format.md`](./fcb-wire-format.md) §2 規則 2b。
>   這兩個 helper 各自只產出一把 key（`evidence::manifest_to_meta` → `evidence.rs:61-65`、
>   `task::task_to_meta` → `task.rs:54-56`），怎麼合併由你的呼叫端決定，crate 不會替你排。
> - 答案安全防呆：`task::contains_answer_fields(&value)` 可在打包前 assert 沒夾帶答案 key。
>
> 有一點要先知道：`lib.rs` **並沒有**把 `pack`/`open` 之類的頂層函式 re-export 出來，它只 re-export 了
> `error::{FcbError, Result}`。高階函式全都待在子模組裡（`bundle::`、`evidence::`、`task::`、`submission::`、
> `binding::`），得走模組路徑來呼叫。所以你的 `use` 區塊大概會長這樣：
>
> ```rust
> use fcb::bundle::{self, BundleParams};
> use fcb::container::BundleKind;
> use fcb::evidence::{manifest_to_meta, StreamData, StreamManifest};
> use fcb::task::{task_to_meta, ReportMode, TaskSpec, TaskStep};
> use fcb::cbor;
> ```

### 可直接照抄的最小 `.case` 打包範例（Rust）

> 要產出 `.case`，權威的 helper 就是 `fcb::case::pack_case`。你餵給它 manifest、選擇性的 task，再加上
> `CasePayload { streams }`，剩下的它全包了：用 **canonical 序列化**算出 `bundle_hash`、組好
> `{ streams, task? }` 的 header meta、產生隨機 salt/nonce，最後以預設 Argon2id cost 封裝。生產端和
> 消費端（連同 WASM bridge）共用同一個公開的 `CasePayload` 信封，格式從源頭就不會漂移。
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
> 如果你想逐欄看清楚底層怎麼用 `bundle::pack_bytes` 加上 `*_to_meta` 手動組信封，範例在
> `crates/fcb/tests/stream_types.rs:88-115`，裡面有 RFC 5424、3164、minimal 三筆 worked example。

### 用非 Rust 語言寫：照 `fcb-reference.md` 逐位元對齊

> 只有當你要用**非 Rust 語言**重寫 codec，才需要照著 [`fcb-reference.md`](./fcb-reference.md) 逐位元對齊
> （想要白話脈絡，再搭配 [`fcb-wire-format.md`](./fcb-wire-format.md) 和
> [`fcb-data-model.md`](./fcb-data-model.md)）。這裡有一個**最容易踩到的互通陷阱**：ciborium 會把 `Vec<u8>`
> （也就是 `salt`、`nonce`、`key_check`）編成 **CBOR array of uint**（major type 4），而**不是** byte string
> （major type 2）。這點寫錯，header 就跟參考實作不相容了。驗收標準其實很單純：產出的位元組只要能通過
> `cargo test -p fcb`（尤其是 `case_vector_is_byte_stable` 和 `frozen_case_vector_decodes_to_expected_structure`），
> 就算相容。

### `.case` 產出：用 crate 的 `fcb::case`（生產／消費共用）

> `fcb::case` 模組提供兩個權威 helper，生產端和消費端（連同 WASM bridge）共用同一套，格式從源頭就不會漂移：
>
> 1. **`pack_case(&CaseInput, passphrase)`** — 把 `{ streams: [StreamData] }` 信封組好，封裝成 `.case`。
>    輸入是 `CaseInput { case_id, manifest, task, payload }`；其中 `CasePayload { streams }` 是唯一的公開
>    信封型別，golden vector、`stream_types.rs`、WASM bridge 全都重用它，不再各自宣告 test-local 版本。
> 2. **canonical `bundle_hash`** — `case::case_bundle_hash(&CasePayload)` 等於 `compute_bundle_hash(canonical
>    明文 payload bytes)`，已經**凍結**成「sha256(明文 payload bytes)」，而且由 `pack_case` 自動寫進 header。
>    這樣一來，同一份證物不管 salt/nonce 怎麼變，算出來的 hash 都一樣。回歸測試在 `crates/fcb/tests/vectors.rs`
>    的 `case_canonical_bundle_hash_is_frozen` 和 `pack_case_round_trips_and_binds_hash`。

---

## 已知缺口（Known Gaps）

這裡老實列出**還沒實作、還沒凍結**的部分，免得你把現況估得太樂觀：

1. **WASM 綁定目前只有 `fcb_version`。** `crates/fcb/src/wasm.rs` 只導出 `fcb_version()`，還沒有
   `openBundle`、`packSubmission` 這類更完整的 bindings（補充一句：`fcb-wasm` bridge crate 那邊的
   native core 已經比較完整，見 `crates/fcb-wasm/src/lib.rs`）。
2. **plugin registry 是消費端的概念，本 crate 並沒有實作。** `DecodedStream` 的註解提到未知 type 會落到
   generic fallback「或交給 a registered plugin」（`crates/fcb/src/evidence.rs:50`），可是 crate 裡**根本沒有
   任何 registry 程式碼**。plugin 介面是 `plugin-protocol`（消費端 spec）的事，跟 `.case` 位元組格式無關。
3. **payload 多出 manifest 沒列的 stream 時，行為沒測過。** `decode_streams` 是以 manifest 驅動迭代、再用 `id`
   去反查 payload。manifest 找不到對應的 payload 會回 `Malformed`；但**反過來**，payload 裡有 manifest 沒宣告的
   多餘 stream，目前會被靜默忽略，而且**沒有任何測試**斷言這是刻意設計（`crates/fcb/src/evidence.rs:77-93`，
   未證實）。重寫 codec 或自訂 case builder 時，別依賴這個還沒凍結的行為。

> ✅ 這一批已經關掉的缺口：**`pack_case` / `CasePayload` 公開 helper**、**canonical `bundle_hash` 凍結**、
> **`fcb.netflow.v1` / `fcb.json.v1` 記錄 schema 凍結**——分別見上方「`.case` 產出」段、`fcb::case` 模組，
> 以及 `fcb-data-model.md §3.2/§3.3`。
>
> 想看最細的缺口清單（共 3 項），請翻 `fcb-reference.md §9`；本表已經跟它對齊。
