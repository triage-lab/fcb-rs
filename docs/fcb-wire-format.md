# FCB 線上格式（wire format）

> **先講脈絡（為什麼有這個格式）。** FCB（Forensic Case Bundle）是一個數位鑑識**出題／調查平台**的
> 「證物＋題目」打包格式。流程是：**case builder**（建構器）把證物與題目打包成 `.case` 發給學生 →
> 學生在 **browser workbench**（瀏覽器調查台）解鎖、調查、作答 → 學生端產出 `.casework`（學生作品）→
> **教師審閱平台**讀回 `.casework` 批改。因為 `.case` 會散佈給多名學生、又要能在沒輸入 passphrase 前
> 先顯示 case 資訊，所以才需要本檔描述的這套「明文 header + 加密 payload」信封、以及 `case_id` / `task` /
> 答案安全等欄位。脈絡更完整見 [`README.md`](./README.md) 與 `crates/fcb/src/lib.rs` 的 crate-level module doc。

`.case`（教師指派的題目）與 `.casework`（學生作品）共用同一個 container 信封。本文件描述
**byte-level 外層佈局**、**明文 header 的 CBOR 編碼規則**，以及 **compress-then-encrypt 密碼學管線**。
語意層資料結構（stream 記錄 schema、TaskSpec、Submission 等）放在
[`fcb-data-model.md`](./fcb-data-model.md)，本檔只在「信封層」交代它們長什麼樣、塞在哪裡。

權威來源（衝突時以前者為準）：

1. 參考實作 `crates/fcb/src/{container,crypto,compress,bundle,binding,cbor,error}.rs`
2. byte-exact golden vectors `crates/fcb/tests/vectors.rs`、round-trip `crates/fcb/tests/stream_types.rs`
3. 行為契約 `openspec/specs/`：目前共 **11** 個 spec 目錄，其中 **7** 個是 `fcb-*` 能力 spec
   （`fcb-container-format`、`fcb-case-builder`、`fcb-evidence-model`、`fcb-stream-types`、`fcb-submission`、
   `fcb-task-spec`、`fcb-wasm-bridge`），另 4 個是治理／使用者文件 spec（`doc-language-standard`、
   `oss-project-docs`、`user-integration-guide`、`user-reference-and-changelog`）。本檔不重述各 spec 內容以免漂移。

> 本檔聚焦信封層 byte／crypto 細節；spec 與 docs／code 三者衝突時一律回上面三層原始來源
> （code → golden vector → spec）。

> **整數一律 little-endian（LE）。** 字串為 UTF-8。本檔每一條 byte / 數值 / 行為宣稱都能對應到
> `crates/fcb` 原始碼的具名符號（函式／型別／常數）或具名 golden vector。

---

## 0. 一張圖看懂整個信封

```text
┌───────────────────────────────────────────────────────────────────────┐
│ 固定 11-byte 前綴                                                       │
│  magic(4)  KIND(1)  container_version(2 LE)  hdr_len(4 LE)              │
├───────────────────────────────────────────────────────────────────────┤
│ header  ── hdr_len 個 byte，明文 CBOR（map of 8）                       │
│  header_schema_ver / min_reader / case_id / bundle_hash                │
│  kdf{algo,salt,m_cost,t_cost,p_cost} / aead{algo,nonce}                │
│  key_check / meta                                                      │
├───────────────────────────────────────────────────────────────────────┤
│ payload ── 其餘全部 = AEAD( zstd( 明文 payload ) )                     │
└───────────────────────────────────────────────────────────────────────┘
        ▲ header 明文，但整段前綴綁進 AAD     ▲ 這層被 AEAD 加密＋認證
          → 一併被 AEAD 認證                    （前綴是它的 AAD）
```

兩個設計直覺先講在前面，後面各節都圍著它打轉：

- **header 為什麼明文？** 因為解鎖前你就需要 KDF 的 `salt`/`params` 與 AEAD 的 `nonce` 才能推 key；
  這些本來就不是秘密。也因此消費端（browser workbench / 教師審閱平台）能在「還沒輸入 passphrase」
  時先 `peek_header` 顯示 case 資訊。header 雖是明文、可在解鎖前讀，但**整段明文前綴（含 header）會綁進
  AEAD 的 AAD**，所以仍被認證——竄改 header 任一欄位都會讓 `open` 失敗（見 §3 安全特性）。
- **payload 為什麼先壓再加密？** 密文近似亂數、壓不動；先 zstd 壓縮重複性高的 log 再 AEAD 封裝，
  才省得到空間（見 §3）。

---

## 1. Container 佈局（byte-level）

```text
magic(4) | KIND(u8) | container_version(u16 LE) | hdr_len(u32 LE)
         | header (hdr_len 個 byte，明文 CBOR)
         | payload (其餘全部；= AEAD(zstd(plaintext_payload)))
```

來源：`container.rs` 的 module doc 與 `write_container` / `read_container` / `peek_header`。固定前綴恰
`4 + 1 + 2 + 4 = 11` bytes（由 `encode_prefix` 的 `Vec::with_capacity(4 + 1 + 2 + 4 + hdr.len())` 落實）。

| 偏移 | 欄位 | 大小 | 值 | 來源 | 說明 |
|------|------|------|-----|------|------|
| 0 | `magic` | 4 B | `0x89 0x46 0x43 0x42`（`\x89FCB`） | `container.rs` `MAGIC` | 首 byte `0x89` 仿 PNG：偵測 7/8-bit 傳輸損壞、避免文字碰撞。**版本不烤進 magic**（見 `MAGIC` doc comment）。 |
| 4 | `KIND` | 1 B | `1`=`.case`、`2`=`.casework` | `container.rs` `BundleKind` | `BundleKind`（`to_u8`）。其他值 → `Malformed("unknown KIND byte {other}")`（`BundleKind::from_u8`，placeholder 名為 `{other}`）。 |
| 5 | `container_version` | 2 B LE | 目前 `1`（`CONTAINER_VERSION`） | `container.rs` `CONTAINER_VERSION` | 保留給未來 parse-path 分派；**目前不驗證**（見下方註）。 |
| 7 | `hdr_len` | 4 B LE | header CBOR 的位元組長度 | `container.rs` `encode_prefix` | 寫成 `hdr.len() as u32`，**未檢查是否超過 u32::MAX**（`as` 截斷；`encode_prefix`）。 |
| 11 | `header` | `hdr_len` B | 明文 CBOR（見 §2） | `container.rs` `encode_prefix` | KDF salt/params 與 AEAD nonce 必須在「還沒 key」時就讀得到，故刻意明文。 |
| 11+`hdr_len` | `payload` | 其餘 | `AEAD(zstd(plaintext))`（見 §3） | `container.rs` `write_container` / `read_container` | 在 container 層**逐位元組原樣寫入／讀出（verbatim）**，壓縮加密在 compress/crypto 層。 |

寫入順序固定為 magic → KIND → container_version → hdr_len → header → payload
（`encode_prefix` 後接 `write_container` 串上 payload）。

> **`container_version` 目前不分派也不驗證。** `read_container` 讀進 `container_version` 後保留在
> `Container` 結構，但原始碼明文註解「reserved for future parse-path dispatch; v1 is the only known
> layout today」（見 `read_container` 內的註解）。即使 `container_version != 1` 也不報錯、照 v1 解析。
> 控制相容性的是 **`header.min_reader`**（見 §2 / §6），不是 `container_version`。

### 兩個讀取進入點：`peek_header` vs `read_container`

| 函式 | 用途 | 是否複製 payload | 檢查項 | 來源 |
|------|------|------------------|--------|------|
| `peek_header(bytes)` | **解鎖前**只讀 header，顯示 case 資訊（不需要 passphrase） | 否 | magic / KIND / `min_reader` | `container.rs` `peek_header` |
| `read_container(bytes)` | 解析完整 frame，交給 `open_bytes` 解密 | 是（`bytes[pos..]`） | 同上，外加保留 `kind` / `container_version` | `container.rs` `read_container` |

兩者檢查順序一致：magic → KIND → container_version（`peek` 丟棄、`read` 保留）→ `hdr_len` →
header 範圍 → header CBOR → `min_reader`。`min_reader > READER_VERSION`（目前 `2`，`container.rs` `READER_VERSION`）
→ `UnsupportedVersion`（`peek_header` / `read_container` 各自的版本守門）。`READER_VERSION` 從 `1` 升到 `2`，是因為
明文 header 改為綁進 AEAD AAD（見 §3）：pre-AAD 的 v1 reader 沒有 AAD 步驟、開不了新 bundle，所以新
bundle 寫 `min_reader = 2` 讓舊 reader 優雅拒絕。

`read_u16` / `read_u32` 越界 → `Malformed("truncated u16"/"truncated u32")`
（`container.rs` `read_u16` / `read_u32`）；header 範圍越界 → `Malformed("header length out of bounds")`
（`container.rs` `header_slice`，以 `checked_add` 防 `usize` overflow）。

### Golden vector 前綴拆解

逐位元組對照 `crates/fcb/tests/vectors.rs` 凍結的 hex（解碼自字面字串，已 byte-for-byte 驗證）。

`FROZEN_CASE_HEX`（`.case`，全長 578 bytes，`vectors.rs` 的 `FROZEN_CASE_HEX` 常數）：

```text
89 46 43 42   magic
01            KIND = 1 (.case)
01 00         container_version = 1   (u16 LE)
dc 01 00 00   hdr_len = 0x000001dc = 476   (u32 LE)
a8 ...        header CBOR：map(8 entries)…（接 §2）
              （header 476 B 之後是 91 B 的 AEAD(zstd(payload)))
```

`FROZEN_WORK_HEX`（`.casework`，全長 423 bytes，`vectors.rs` 的 `FROZEN_WORK_HEX` 常數）：

```text
89 46 43 42   magic
02            KIND = 2 (.casework)
01 00         container_version = 1
25 01 00 00   hdr_len = 0x00000125 = 293   (u32 LE)
a8 ...        header CBOR：map(8 entries)…
              （header 293 B 之後是 119 B 的 AEAD(zstd(payload)))
```

兩者 header 都以 `a8` 起頭，因為 `Header` 永遠是 8 欄位的 CBOR map（`container.rs` 的 `Header` struct，含 `#[derive(…Serialize, Deserialize)]`）。
總長拆解：`.case` = 11 前綴 + 476 header + 91 payload = 578；`.casework` = 11 + 293 + 119 = 423。

> ⚠️ **凍結向量用的是「測試用快速 cost」，不是 production 預設。** 凍結 header 內的 Argon2 cost 是
> `m_cost=32`（CBOR `18 20`）/ `t_cost=1` / `p_cost=1`（凍結在 `FROZEN_CASE_HEX`，由 `build_case` 寫入），**不是**
> 函式庫預設的 `19456 / 2 / 1`（`bundle.rs` 的 `DEFAULT_M_COST` / `DEFAULT_T_COST` / `DEFAULT_P_COST`）。`build_case`
> 為了讓測試跑得快才寫死小 cost；正式打包用 §4 的預設。同理 `bundle_hash = "sha256:deadbeef"` 是占位假值
> （凍結在 `FROZEN_CASE_HEX`），不是真 SHA-256。

---

## 2. Header（明文 CBOR）

`header` 是 `ciborium::into_writer` 對 `Header` struct 的序列化結果（`container.rs` 的
`encode_header`）。ciborium 把 struct 編成 **以欄位名為 text key 的 CBOR map**，
**key 順序即 Rust 欄位宣告順序**。`Header` 有 8 個欄位 → map header byte `a8`。

```text
Header = {                          // CBOR map(8) → 起頭 a8
  "header_schema_ver": u16,         // 目前 1（Header.header_schema_ver；寫入 pack_bytes）
  "min_reader":        u16,         // 目前 2（AAD 格式；Header.min_reader；寫入 pack_bytes）
  "case_id":           text,        // 教師指派、穩定的題目識別碼（Header.case_id）
  "bundle_hash":       text,        // 綁定證物版本，格式 "sha256:<hex>"（Header.bundle_hash；見 §5）
  "kdf":  { … 5 欄位 → a5 },         // 見下（Header.kdf）
  "aead": { … 2 欄位 → a2 },         // 見下（Header.aead）
  "key_check": [u8…],               // 32 bytes，見 §4（Header.key_check）
  "meta": <任意 CBOR>               // .case={streams,task}；.casework={}（空 map）。見下（Header.meta）
}
```

> **沒有 `#[serde(rename …)]` 或 `rename_all`** 在 `Header`/`KdfParams`/`AeadParams` 上
> （`container.rs` 的這三個 struct），所以 CBOR key = Rust 欄位名原樣。`rename` 之類的陷阱出現在 `meta`
> 裡的子結構（見下方「ciborium 慣例與陷阱」）。

### 欄位表（含 CBOR key 與來源）

| # | 欄位 | Rust 型別 | CBOR key | 來源 |
|---|------|-----------|----------|------|
| 1 | `header_schema_ver` | `u16` | `"header_schema_ver"` | `container.rs` `Header.header_schema_ver` |
| 2 | `min_reader` | `u16` | `"min_reader"` | `container.rs` `Header.min_reader` |
| 3 | `case_id` | `String` | `"case_id"` | `container.rs` `Header.case_id` |
| 4 | `bundle_hash` | `String` | `"bundle_hash"` | `container.rs` `Header.bundle_hash` |
| 5 | `kdf` | `KdfParams` | `"kdf"` | `container.rs` `Header.kdf` |
| 6 | `aead` | `AeadParams` | `"aead"` | `container.rs` `Header.aead` |
| 7 | `key_check` | `Vec<u8>` | `"key_check"` | `container.rs` `Header.key_check` |
| 8 | `meta` | `ciborium::value::Value` | `"meta"` | `container.rs` `Header.meta` |

**`kdf` = `KdfParams`（巢狀 map(5) → `a5`，`container.rs` 的 `KdfParams` struct）：**

| 欄位 | 型別 | CBOR key | 凍結向量值 |
|------|------|----------|-----------|
| `algo` | `String` | `"algo"` | `"argon2id"`（`FROZEN_CASE_HEX`） |
| `salt` | `Vec<u8>` | `"salt"` | array(16)，起頭 `90`（`FROZEN_CASE_HEX`） |
| `m_cost` | `u32`（KiB） | `"m_cost"` | `32` = `18 20`（測試 cost，`FROZEN_CASE_HEX`） |
| `t_cost` | `u32`（迭代） | `"t_cost"` | `1` = `01`（`FROZEN_CASE_HEX`） |
| `p_cost` | `u32`（平行度） | `"p_cost"` | `1` = `01`（`FROZEN_CASE_HEX`） |

**`aead` = `AeadParams`（巢狀 map(2) → `a2`，`container.rs` 的 `AeadParams` struct）：**

| 欄位 | 型別 | CBOR key | 凍結向量值 |
|------|------|----------|-----------|
| `algo` | `String` | `"algo"` | `"xchacha20poly1305"`（`FROZEN_CASE_HEX`） |
| `nonce` | `Vec<u8>` | `"nonce"` | array(24)，起頭 `98 18`（`FROZEN_CASE_HEX`） |

正式打包時（`bundle.rs` 的 `pack_bytes`）：`kdf.algo` 寫死 `"argon2id"`、`kdf.salt` = 16 bytes 隨機、
cost 取 `BundleParams` 的 `m_cost`/`t_cost`/`p_cost`（預設 `19456`/`2`/`1`，即 `bundle.rs` 的
`DEFAULT_M_COST` / `DEFAULT_T_COST` / `DEFAULT_P_COST`，由 `BundleParams::new` 帶入）；
`aead.algo` 寫死 `"xchacha20poly1305"`、`aead.nonce` = 24 bytes 隨機。

### 一段連續的 header hex 拆解（可逐 byte diff）

上面的欄位表是分散的 marker 速查；下面把 `FROZEN_CASE_HEX` 的 header **從頭連續拆到
`kdf.salt`**，每行一個 byte 群＋註解，讓手刻 codec 的作者能直接和自己的輸出逐 byte 對齊（已 byte-for-byte
驗證）：

```text
89 46 43 42                          magic \x89FCB                （11-byte 前綴，§1）
01                                   KIND = 1 (.case)
01 00                                container_version = 1 (u16 LE)
dc 01 00 00                          hdr_len = 476 (u32 LE)
a8                                   header = CBOR map(8)
  71 6865616465725f736368656d615f766572  key "header_schema_ver"  (0x60+17 → 0x71，後接 17 個 UTF-8 byte)
  01                                   value 1
  6a 6d696e5f726561646572            key "min_reader"          (0x60+10 → 0x6a)
  02                                   value 2                   (AAD 格式：min_reader = 2)
  67 636173655f6964                  key "case_id"             (0x60+7 → 0x67)
  6f 61636d652d69722d323032362d3033  value "acme-ir-2026-03"   (0x60+15 → 0x6f)
  6b 62756e646c655f68617368          key "bundle_hash"         (0x60+11 → 0x6b)
  6f 7368613235363a6465616462656566  value "sha256:deadbeef"   (0x60+15 → 0x6f；占位假值)
  63 6b6466                          key "kdf"                 (0x60+3 → 0x63)
  a5                                   value = CBOR map(5) = KdfParams
    64 616c676f                      key "algo"                (0x60+4 → 0x64)
    68 686172676f6e326964            value "argon2id"          (0x60+8 → 0x68)
    64 73616c74                      key "salt"                (0x60+4 → 0x64)
    90 1853 1841 184c 1854 01 02 …   value salt = array(16)    (0x80+16 → 0x90；每 byte≥24 以 18 xx 表示)
    …                                （之後接 m_cost 18 20 / t_cost 01 / p_cost 01，再 "aead" map(2)、
                                       "key_check" array(32)、"meta"——形狀見下方各表）
```

> 縮排只是排版輔助，CBOR 本身沒有縮排或分隔符——map 的 entry 數由 `a8`/`a5` 的 count 決定，解碼時連續讀。
> 同一份 header 的完整 byte 仍以 `FROZEN_CASE_HEX` 字面字串為唯一真相。

### `meta` 欄位的形狀（信封層觀點）

`meta` 在 container 層是 opaque CBOR（`ciborium::value::Value`），上層才解讀。兩種 KIND 形狀不同：

| KIND | `meta` 形狀 | CBOR | 來源 |
|------|-----------|------|------|
| `.case` | `{ "streams": [StreamManifest…], "task": TaskSpec }` | map(2) → `a2` | `FROZEN_CASE_HEX`、data-model §1 |
| `.casework` | `{}`（空 map） | `a0` | `FROZEN_WORK_HEX`、data-model §0 |

`StreamManifest` 的 CBOR key 是 `id` / **`type`**（Rust 欄位 `stream_type` 標 `#[serde(rename = "type")]`，
見 `evidence.rs` 的 `StreamManifest.stream_type`）/ `records`。`StreamManifest`、`TaskSpec`、`Submission` 等深層 schema 與其
演進規則一律見 [`fcb-data-model.md`](./fcb-data-model.md)，本檔不重述以免漂移。

> **`task` 的互通陷阱（library vs 測試向量）：** 函式庫的 `TaskMeta.task` 是
> `Option<TaskSpec>` + `#[serde(default, skip_serializing_if = "Option::is_none")]`
> （`task.rs` 的 `TaskMeta`），`None` 時**整個 `task` key 會被省略**；但凍結向量用的測試結構
> `CaseMeta`（`vectors.rs`）把 `task` 寫成非 Option 欄位，**永遠寫出 `task`**（凍結在 `FROZEN_CASE_HEX`）。所以凍結
> `.case` 的 meta 是 `a2`（含 `task`）。若以 `CaseMeta` 為樣板複製，會和 library 端 `TaskMeta`（`task: None` 省略 `task` key）的輸出
> 不一致。讀取端兩種都容忍（`manifest_from_meta` 只讀 `streams`、`task_from_meta` 只讀 `task`）。

### ⚠️ ciborium 慣例與陷阱（互通關鍵）

非 Rust 的 case builder（建構器）想產出 byte-相容的 header，必須複刻 ciborium 0.2 的這些行為：

1. **`Vec<u8>` 編成 CBOR array of uint，不是 byte string。**
   `salt` / `nonce` / `key_check` 都是 `Vec<u8>`（`container.rs` 的 `KdfParams.salt` / `AeadParams.nonce` /
   `Header.key_check`）。ciborium 對 serde 的
   `Vec<u8>` 走 `serialize_seq`，產出 **CBOR array（major type 4），每個 byte 是一個 unsigned integer**，
   **不是** byte string（major type 2，起頭會是 `0x40`/`0x50`/`0x58`）。
   CBOR array（major type 4）的計數編碼規則（由 golden vector `FROZEN_CASE_HEX` 驗證）：

   | 元素數 | 起頭 byte | 例（本檔欄位） |
   |--------|-----------|----------------|
   | 0–23 | 單 byte `0x80 + n` | 16-byte salt → `0x90`（array(16)） |
   | 24–255 | `0x98` + 1-byte count | 24-byte nonce → `98 18`；32-byte key_check → `98 20` |

   且每個 byte 值 ≥ 24 時，array 元素本身也以 `18 xx` 兩 byte 表示（uint 的小整數編碼規則，完整門檻見下表）。
   **若把這些欄位寫成 byte string，ciborium 反序列化成 `Vec<u8>` 時預期 array → 不相容。**

   **CBOR unsigned integer（major type 0）的計數編碼規則。** 這套規則同時適用於兩處：（a）header 內的整數
   **value** 欄位——`m_cost` / `t_cost` / `p_cost`（`container.rs` 的 `KdfParams`）、以及 `meta` 內
   `StreamManifest.records`（`u64`，`evidence.rs` 的 `StreamManifest.records`）、`fcb.syslog.v1` 的 `pid` / `severity` / `facility`；
   （b）上面 `Vec<u8>` array 內**每個值 ≥ 24 的 byte 元素**（如 `salt` / `nonce` / `key_check`）：

   | 數值 n | 起頭 byte | 後接 | 例 |
   |--------|-----------|------|-----|
   | 0–23 | 單 byte `0x00 + n`（值即本身） | — | `t_cost = 1` → `01`；byte `0x17`=23 → `17` |
   | 24–255 | `0x18` + 1-byte | 1 個 byte（大端等同單 byte） | 測試 cost `m_cost = 32` → `18 20`（`FROZEN_CASE_HEX`）；byte `0x53`=83 → `18 53` |
   | 256–65535 | `0x19` + 2-byte（**big-endian**） | 2 個 byte | **production 預設 `m_cost = 19456`（`bundle.rs` `DEFAULT_M_COST`）→ `19 4c 00`** |
   | 65536–2³²−1 | `0x1a` + 4-byte（big-endian） | 4 個 byte | `records = 100000` → `1a 00 01 86 a0` |
   | 2³²–2⁶⁴−1 | `0x1b` + 8-byte（big-endian） | 8 個 byte | （`u64` `records` 上界） |

   > ⚠️ **注意整數本身的多 byte 編碼是 big-endian**，與 §1 container 前綴的 `container_version` / `hdr_len`
   > （**little-endian**）相反——前綴是裸 LE 整數、不是 CBOR；header 內的整數才走 CBOR major type 0 的
   > big-endian。凍結向量唯一的整數 worked example 是測試 cost `m_cost = 32 → 18 20`（`FROZEN_CASE_HEX`）；
   > production 預設 `19456` 落在 256–65535 區段、要編成 `19 4c 00`（`0x4c00 = 19456`，big-endian 2-byte），
   > 不要誤用單 byte 或 LE。

   **CBOR text string（major type 3）的計數編碼規則。** 所有 struct 欄位名（map 的 text key）、
   enum variant 的小寫字串、以及 `case_id` / `algo` / `format` 等 text **value**，都用同一套長度前綴
   （長度按 **UTF-8 byte 數**算、非字元數）：

   | byte 長度 | 起頭 byte | 後接 |
   |-----------|-----------|------|
   | 0–23 | 單 byte `0x60 + len` | UTF-8 bytes |
   | 24–255 | `0x78` + 1-byte len | UTF-8 bytes |
   | 256–65535 | `0x79` + 2-byte len（big-endian） | UTF-8 bytes |

   例（皆取自 `FROZEN_CASE_HEX`）：key `"header_schema_ver"`（17 bytes）→ `0x71`（=`0x60+17`）
   後接該 17 個 byte；key `"bundle_hash"`（11 bytes）→ `0x6b`；value `"argon2id"`（8 bytes）→ `0x68`；
   value `"acme-ir-2026-03"`（`case_id`，15 bytes）→ `0x6f`；value `"xchacha20poly1305"`（17 bytes）→ `0x71`。
   **注意 CBOR text string 與 CBOR array／byte string 是三種不同的 major type，count 前綴各自不同（text=`0x60+`、
   array=`0x80+`、byte string=`0x40+`），不可混用。**（text/array 計數編碼皆由 ciborium 0.2 決定，由
   golden vector `FROZEN_CASE_HEX` 逐 byte 佐證。）

2. **struct → text-key map，key 順序 = 宣告順序。** 見上方 `Header`/`KdfParams`/`AeadParams`。
   text key 本身的長度前綴照上表編碼。

2b. **`ciborium::value::Value::Map` 依內部 `Vec<(key,value)>` 的插入順序原樣輸出，不做 canonical 排序。**
   這條與規則 2 不同：規則 2 的「順序=宣告序」只對 **struct** 成立；`header.meta`（`ciborium::value::Value`，
   `container.rs` 的 `Header.meta`）與 `fcb.syslog.v1` 的 `sd`（`Value::Map`，`stream_types.rs` 的 `rfc5424_record`）在型別上是
   `Value::Map`，**不是** struct，ciborium 一律照建構時 `Vec` 的插入序輸出。這直接決定 byte-exactness：
   - `header.meta`（`.case`）必須是 `{ "streams", "task" }` **這個順序**（`streams` 在前），golden vector 才對得上
     （凍結在 `FROZEN_CASE_HEX`）。若改用 `manifest_to_meta`（`evidence.rs`）＋ `task_to_meta`（`task.rs`）合併，
     必須自己保證 `streams` 在 `task` 之前。
   - `sd` 的外層 SD-ID（如 `"ex@32473"`）與內層 param（如 `"iut"`）順序皆由建構 `vec` 的插入序決定
     （`stream_types.rs` 的 `rfc5424_record`），消費端與生產端須一致。

3. **`#[serde(rename = "...")]` 改 key 名。** 例：`StreamManifest.stream_type` 標
   `#[serde(rename = "type")]` → CBOR key 為 `"type"`（`evidence.rs` 的 `StreamManifest.stream_type`）。

4. **`#[serde(rename_all = "lowercase")]` 的 enum → 小寫 text。** 例：`TaskSpec.report_mode`
   的 `ReportMode` → `"steps"` / `"freeform"`（見 data-model §1.2）。

5. **`Option<T>` + `skip_serializing_if = "Option::is_none"` → `None` 時整個 key 省略。**
   例：`TaskMeta.task`（`task.rs` 的 `TaskMeta`，見上方 task 陷阱）。

> 上述行為由 ciborium 0.2 決定；原始碼層級只看得到 `Vec<u8>` / struct / `#[serde(...)]` 宣告，
> 確切 byte 由 golden vector（`FROZEN_CASE_HEX`）佐證。

---

## 3. Payload 管線：compress-then-encrypt

來源：`compress.rs`（`pack_payload`/`unpack_payload`/`compress`/`decompress`）。順序重要——密文近似
亂數、壓不動，所以**先 zstd 壓縮、再 AEAD 加密**：

```text
pack:    plaintext --zstd Fastest--> zstd_frame --XChaCha20-Poly1305--> payload
open:    payload --AEAD decrypt--> zstd_frame --zstd decompress--> plaintext
```

| 函式 | 行為 | 來源 |
|------|------|------|
| `compress` | `compress_to_vec(data, CompressionLevel::Fastest)`，恆成功（`Ok`） | `compress.rs` `compress` |
| `decompress` | `StreamingDecoder` + `read_to_end`；**任何**錯誤 → `Corrupt` | `compress.rs` `decompress` |
| `pack_payload` | 先 `compress`、再 `crypto::seal`（AEAD 包住 zstd frame） | `compress.rs` `pack_payload` |
| `unpack_payload` | 先 `crypto::open_payload`、再 `decompress`（反序） | `compress.rs` `unpack_payload` |

- **壓縮**：標準 zstd frame（magic `0x28 0xB5 0x2F 0xFD`，`compress.rs` `ZSTD_MAGIC`）。後端是
  **純 Rust `ruzstd` 0.8**（無 C/FFI，native 與 `wasm32-unknown-unknown` 共用同一份程式碼，見
  `compress.rs` 的 module doc）。`ruzstd` 0.8 編碼**目前只實作 `Fastest`**（與 `Uncompressed`）；更高等級尚未
  實作、不要使用（見 `compress` 的 doc comment）。`Fastest` 對重複性高的 log 已有好壓縮比。frame 格式與
  更高等級相容、未來可無縫升級，並能與 C zstd reader 互通。
- **加密**：XChaCha20-Poly1305（`chacha20poly1305` 0.10）。key = §4 推導的 32 bytes、nonce =
  `header.aead.nonce`（24 bytes）、**AAD = 明文 container 前綴**（magic、KIND、container_version、hdr_len、
  以及完整 header CBOR；pack 對 `encode_prefix(...)` 封裝，open 端在 `bundle.rs` 的 `open_bytes` 取
  `bytes[..bytes.len()-payload.len()]` 為 AAD；見 `crypto.rs` 的 `seal` / `open`）。Poly1305 tag 由 crate
  附在密文尾端（密文長 = 明文 + 16-byte tag），FCB 沒有獨立 tag 欄位（`crypto.rs` 的 `seal`）。

順序由測試 `order_is_compress_then_encrypt`（`compress.rs`）鎖定：解密 packed payload 得到的正是 zstd frame，
其前 4 byte == `ZSTD_MAGIC`、而外層密文前 4 byte != `ZSTD_MAGIC`。

> ### 安全特性（務必知道）
>
> - **AEAD 同時認證 payload 與整段明文 header／前綴。** 封裝時把明文 container 前綴（magic、KIND、
>   container_version、hdr_len、完整 header CBOR）綁進 AEAD 的 AAD（`crypto.rs` 的 `seal`、`bundle.rs` 的 `pack_bytes`）。
>   因此竄改 header 任一欄位（含 `case_id`、`bundle_hash`、`meta` 裡的 manifest/task）——只要 passphrase
>   正確、header 仍能解析——`open` 都會失敗為 **`Corrupt`**（測試 `header_tamper_is_corrupt`，`bundle.rs`）。
>   注意結構性檢查仍先行：magic 壞 → `BadMagic`、未知 KIND → `Malformed`。
> - **`.case` 開封會驗證內容定址。** `open_case`（fcb-wasm）對解密後的 canonical payload **重算 `bundle_hash`
>   並和 header 值比對**，不符即 `Corrupt`（`fcb-wasm/src/lib.rs`，`open_case`）。AAD 只保證 `bundle_hash`
>   欄位沒被竄改、不保證它等於 payload 的 hash，故 `.case` 路徑額外重算。**`.casework` 不做此重算**——
>   submission 的 header `bundle_hash` 是綁回其 case 的參照，不是 submission payload 的雜湊。
> - **`key_check` 只認證「key 正確」**、**payload AEAD 認證「payload 未被動」、AAD 認證「header／前綴未被動」**。
> - 證物版本綁定靠 `bundle_hash`（見 §5；data-model §7「Binding」）；`.case` 路徑已由 `open_case` 重算驗證，
>   低階 `compute_bundle_hash` 本身仍不強制涵蓋範圍。

---

## 4. 密碼學

來源：`crypto.rs`、常數 `bundle.rs`。

### KDF：Argon2id

```text
key(32 B) = Argon2id(
    password = passphrase 的 UTF-8 bytes,    // derive_key
    salt     = header.kdf.salt,              // derive_key
    m_cost   = header.kdf.m_cost,            // KiB
    t_cost   = header.kdf.t_cost,            // 迭代
    p_cost   = header.kdf.p_cost,            // 平行度
    version  = 0x13 (Argon2 v1.3),           // derive_key：Version::V0x13
    out_len  = 32,                           // derive_key：Params::new(..., Some(KEY_LEN))
)
```

`derive_key`（`crypto.rs`）**只驗 `kdf.algo == "argon2id"`**，其餘回
`Malformed("unsupported KDF: {algo}")`。`Params::new` 失敗 →
`Malformed("bad argon2 params: …")`、`hash_password_into` 失敗 → `Malformed("argon2 failure: …")`。
`out_len = 32` 被綁定兩次（params `Some(KEY_LEN)` + 32-byte 輸出緩衝，皆在 `derive_key` 內）。

| 常數 | 值 | 來源 |
|------|-----|------|
| `KEY_LEN` | 32 bytes | `crypto.rs` |
| `NONCE_LEN` | 24 bytes | `crypto.rs` |
| `SALT_LEN` | 16 bytes | `bundle.rs` |
| `DEFAULT_M_COST` | 19456（KiB） | `bundle.rs` |
| `DEFAULT_T_COST` | 2（迭代） | `bundle.rs` |
| `DEFAULT_P_COST` | 1（平行度） | `bundle.rs` |

> **`aead.algo` 不會被驗證。** `seal`/`open` 完全忽略它（`crypto.rs` 中沒有任何讀 `aead.algo` 的
> 程式碼），它只是描述性欄位。對照之下 `kdf.algo` **有**驗證。不要假設 `aead.algo` 像 `kdf.algo`
> 一樣有檢查。
>
> **cost 預設不寫死在 `derive_key`。** 它們存在 `bundle.rs` 的 `DEFAULT_*`，並**逐 bundle 寫進明文
> header**（`bundle.rs` 的 `pack_bytes`），所以未來可調整而不破壞既有檔案——reader 一律用 header 裡的值。

### Key-Check Value（KCV）

```text
key_check = SHA256( "FCB-key-check-v1" || key )     // 32 bytes（domain 在前、key 在後）
```

`KCV_DOMAIN = b"FCB-key-check-v1"`（16 ASCII bytes，`crypto.rs` 的 `KCV_DOMAIN`）。`key_check_value` 先
`update(KCV_DOMAIN)` 再 `update(key)`（**domain 前綴先 hash、key 後 hash**，`crypto.rs` 的 `key_check_value`）。
開檔時的二分法（`crypto.rs` 的 `open_payload`）：

- KCV 不符 → `WrongPassphrase`（密碼錯，`open_payload`）。
- KCV 相符但 AEAD 驗證失敗 → `Corrupt`（檔案被竄改／毀損，`open` / `open_payload`）。

比較用 constant-time（`crypto.rs` 的 `ct_eq`）：長度不同立即回 `false`（長度非秘密）。
否則 XOR 累加、無提前跳出。**注意這是手寫累加器、非 `subtle` crate**，但對等長輸入仍是 constant-time。
發佈 key 的 hash 不會實質幫到攻擊者，因為 Argon2id 仍是暴力門檻。

### AEAD：XChaCha20-Poly1305

key = 上面 32 bytes、nonce = 24 bytes（`header.aead.nonce`）、**AAD = 明文 container 前綴**（magic、KIND、
container_version、hdr_len、完整 header CBOR；見 §3）。`nonce_from` 強制 `len == 24`，否則
`Malformed("nonce must be 24 bytes, got N")`（`crypto.rs` 的 `nonce_from`）。tag 附在密文尾端（標準 AEAD 輸出，
`crypto.rs` 的 `seal`）。`seal`／`open` 都收 `aad: &[u8]`；AAD 不符即視為竄改、失敗為 `Corrupt`。

---

## 5. `bundle_hash` 與 binding（信封層摘要）

`compute_bundle_hash(bytes) -> "sha256:" + lower_hex(SHA256(bytes))`（`binding.rs`，小寫 hex）是低階
primitive，對任意 bytes 算 hash；golden vector 的 header 仍用占位假值 `"sha256:deadbeef"`（`vectors.rs`）。

`verify_binding(...)` 回傳三態 `Match` / `CaseMismatch` / `EvidenceVersionMismatch`；work key 慣例
`work_key(case_id) = "fcb:work:{case_id}"`。binding 的完整規則、`.casework` 的 `Submission` schema
與 binding 三態語意一律見 [`fcb-data-model.md`](./fcb-data-model.md) §6（Submission schema）與 §7（Binding）。

**canonical 定義（已凍結）：`bundle_hash = compute_bundle_hash(.case 的明文 payload bytes)`**，亦即壓縮／
加密**之前**的證物序列化位元組。如此同一份證物無論 salt/nonce 為何都得到相同 hash，學生作品
（`.casework`）才能可靠綁回特定證物版本。此定義由 `fcb::case::case_bundle_hash(&CasePayload)` 落實、
由 `pack_case` 自動帶入 header，並以 `vectors.rs` 的 `case_canonical_bundle_hash_is_frozen` 凍結。

---

## 6. 錯誤語意（`error.rs`）

`FcbError` 衍生 `PartialEq, Eq`（`error.rs`）。各變體的觸發點（以具名符號標示）：

| 變體 | 意義 | 主要觸發點 |
|------|------|-----------|
| `BadMagic` | 前 4 bytes 不是 FCB magic | `container.rs` `peek_header` / `read_container` |
| `UnsupportedVersion { min_reader, supported }` | bundle 要求的 reader 版本比本 reader 新 | `container.rs` `peek_header` / `read_container` |
| `Malformed(String)` | 結構性錯誤：truncated u16/u32、missing/unknown KIND、header out of bounds、bad header CBOR、unsupported KDF、bad argon2 params、nonce 長度錯、rng failure、CBOR encode 失敗… | `container.rs`（`from_u8`/`read_u16`/`read_u32`/`header_slice`/`encode_header`/`peek_header`/`read_container`）；`crypto.rs`（`derive_key`/`nonce_from`）；`bundle.rs`（`random_bytes`）；`cbor.rs`（`to_value`/`from_value`/`encode`） |
| `WrongPassphrase` | KCV 不符（密碼錯） | `crypto.rs` `open_payload` |
| `Corrupt` | KCV 相符但 AEAD 失敗、zstd frame 壞、或解密後 payload CBOR 解碼失敗 | `crypto.rs`（`open`）；`compress.rs`（`decompress`）；`cbor.rs`（`decode`） |

> **語意陷阱：CBOR 解碼失敗的兩條路不同。** header 的 CBOR 壞 → `Malformed("bad header CBOR")`
> （明文層，`container.rs` 的 `peek_header` / `read_container`）；但**解密後 payload** 的 CBOR 壞 → `cbor::decode`
> 對映成 **`Corrupt`**（`cbor.rs` 的 `decode`），語意是「解密/解壓後的 payload 壞了」。

由 golden vector 鎖定的錯誤行為：`open_bytes(CASE, "wrong")` → `WrongPassphrase`
（測試 `wrong_passphrase_on_vector_is_rejected`）；翻動 CASE 最後一個 byte → `Corrupt`
（測試 `tampered_vector_is_corrupt`）。

---

## 7. Stream type 派發（信封層觀點）

`.case` 的每條 stream 在 manifest 裡帶一個 namespaced + versioned 的 `type`（CBOR key `"type"`）。
派發規則：

- 內建型別清單 `BUILTIN_STREAM_TYPES = ["fcb.syslog.v1", "fcb.netflow.v1", "fcb.json.v1"]`
  （`evidence.rs` 的 `BUILTIN_STREAM_TYPES`）。`is_builtin_type(t)` = 清單是否包含 `t`（`evidence.rs` 的 `is_builtin_type`）。
- **這不是封閉清單。** 未知 / 第三方 type（如 golden vector 的 `acme.edr.v1`）仍解析成 first-class
  stream，只是 `is_builtin == false`（`FROZEN_CASE_HEX`、`evidence.rs` 的 `decode_streams`），消費端落
  generic table / timeline fallback、**不致命**。
- 三個內建型別皆有凍結 schema：**`fcb.syslog.v1`**（data-model §3.1）、**`fcb.netflow.v1`**（5-tuple +
  bytes/packets + 時間區間，data-model §3.2）、**`fcb.json.v1`**（任意 CBOR map 通用容器，data-model §3.3）；
  各以 `crates/fcb/tests/stream_types.rs` 的 round-trip 測試凍結。

stream 記錄的逐欄 schema、演進／相容規則一律見 [`fcb-data-model.md`](./fcb-data-model.md) §3，本檔不重述。

---

## 8. 打包一個 `.case` 的完整順序（end-to-end，可照做）

把前面各節串成一條流程，對應 `bundle.rs` 的 `pack_bytes`。資料結構細節見
[`fcb-data-model.md`](./fcb-data-model.md)。

1. **準備證物**：每條 stream 整成 `StreamData { id, records }`；`records` 每筆形狀依該 stream 的
   `type`（如 `fcb.syslog.v1`，data-model §3.1）。
2. **組 manifest**：每條 stream 一筆 `StreamManifest { id, type, records = len(records) }`
   （CBOR key 是 `"type"`）。
3. **組 task spec**：`TaskSpec`（**零答案**，data-model §1.2 schema／§8 零答案不變量）。
4. **`meta`**（明文）= CBOR `{ "streams": [manifest…], "task": TaskSpec }`（map(2) → 起頭 `a2`，`streams`
   在前；見 §2 規則 2b）。
5. **`payload_plain`** = CBOR `{ "streams": [StreamData…] }`。此信封是**單欄 struct** 公開型別
   `fcb::case::CasePayload { streams }`，ciborium 編成 map(1) → 起頭 **`a1`**，接 key `"streams"`
   （`67 73747265616d73`）再接 array(n) 的 `StreamData`。`.casework` 此處改為 `Submission`（7 欄 struct →
   map(7) → `a7`，`submission.rs`）。
6. **`bundle_hash`**：canonical = `compute_bundle_hash(payload_plain)`
   （= `"sha256:" + lower_hex(SHA256(payload_plain))`），**已凍結**（§5、§9）；由 `case::case_bundle_hash`
   落實、`pack_case` 自動帶入。
7. **隨機**產生 `salt`(16 B) 與 `nonce`(24 B)（`pack_bytes` 內以 `random_bytes` 呼叫 `getrandom`）。
8. **key** = Argon2id(passphrase UTF-8, salt, m/t/p, version 0x13, out 32 B)（`pack_bytes` 呼叫 `derive_key`）。
9. **key_check** = SHA256(`"FCB-key-check-v1"` ‖ key)（32 B；**加密前**算好，`pack_bytes` 呼叫 `key_check_value`）。
10. **先組 header 與前綴**（step 11 的 AAD 需要它）：`min_reader = 2`、`bundle_hash`、`meta` 等備齊後組成
    `header`（struct），再以 `encode_prefix(KIND, header)` 序列化出明文前綴
    （magic ‖ KIND ‖ container_version ‖ hdr_len ‖ hdr，`pack_bytes` 呼叫 `encode_prefix`）。
11. **compressed** = zstd `Fastest`(payload_plain)。
12. **ciphertext** = XChaCha20-Poly1305 seal(key, nonce, compressed)，**AAD = step 10 的明文前綴**
    （`pack_bytes` 內 `compress::pack_payload(..., &prefix)`）。竄改前綴任一 byte（含 header 任一欄位）都會讓 open 失敗為 `Corrupt`。
13. **輸出位元組** = `前綴 ‖ ciphertext`（前綴即 `magic(89 46 43 42) ‖ KIND(0x01) ‖
    container_version(01 00) ‖ len(hdr) as u32 LE ‖ hdr`，`pack_bytes` 串接 `encode_prefix` 與 ciphertext）。
    header struct 內容為 `{ header_schema_ver: 1, min_reader: 2, case_id, bundle_hash,
    kdf: {algo:"argon2id", salt, m_cost, t_cost, p_cost}, aead: {algo:"xchacha20poly1305", nonce},
    key_check, meta }`（`pack_bytes` 組裝的 `Header`），`hdr` 為其 CBOR（套用 §2 的 `Vec<u8>`→array、欄位順序、
    key 名等規則）。

> ℹ️ **JS/wasm 作者：超 safe-integer 的整數要用 BigInt。** 透過 `fcb-wasm` 的 `packCase`／`packSubmission`
> 打包時，記錄裡若有**絕對值超過 `2^53 - 1`（= 9007199254740991）的整數**，用普通 JS `number` 會被
> serde 編成 CBOR float、和原生整數作者的雜湊發散，故 pack 邊界會以 `malformed` **拒絕**它（訊息提示
> 「supply it as a BigInt」）。改用 `BigInt` 即無損編成 CBOR 整數（最高到 `u64::MAX`）。safe-range 整數與
> 真正的小數 float 都照常接受、且確定。詳見 §9「pack 邊界數值確定性契約」。

> `.casework` 相同，差別只在：`KIND = 0x02`、`meta = {}`（空 map）、`payload_plain = CBOR(Submission)`
> （見 data-model §6）。
>
> ℹ️ **兩個 `.casework` byte-stable 向量。** `FROZEN_WORK_HEX` 凍結的是**測試專用的 3 欄
> `WorkPayload { case_id, bundle_hash, report }`**（歷史向量保留）；library 真正會寫的 7 欄 `Submission`
> （`submission.rs`）則由 **`FROZEN_SUBMISSION_HEX` + `submission_vector_is_byte_stable`**（`vectors.rs`）
> 逐位元釘住，並有 `frozen_submission_vector_decodes_to_expected_structure` 驗 7 欄還原。重寫 codec 想驗
> `Submission` 的 byte-exactness，直接比對 `FROZEN_SUBMISSION_HEX` 即可。

> **想要可直接照抄、會編譯的範例？** Rust 端最小可跑的打包範例見
> `crates/fcb/tests/stream_types.rs` 的 `round_trip`（組 `StreamManifest` → `manifest_to_meta` →
> 自組 `CasePayload { streams }` → `cbor::encode` → `BundleParams::new` → `pack_bytes` → `open_bytes`），
> 以及 `README.md` §「給 case builder 作者的起手建議」的 `use` 匯入範例。

順序要點：`salt`/`nonce` 在推 key 前就要產生，`key_check` 在加密前算好。`header` 要等
`salt`/`nonce`/`key_check`/`bundle_hash`/`meta` 都備齊後才能序列化求 `hdr_len`（`header` 不含
`ciphertext`）。又因 header／前綴是 AEAD 的 AAD，**前綴必須在 seal 之前先序列化**（`encode_prefix` 先行、
seal 才把它當 AAD 綁進 tag），這也是 step 10 先於 step 12 的原因。

> **case builder（建構器）作者建議：** Rust 寫的 case builder 直接相依 `fcb` crate（已是
> `crate-type = ["cdylib", "rlib"]`），呼叫 `bundle::pack_bytes` 就能拿到 byte-相容輸出；
> 非 Rust 實作則要自行複刻 §2 的 ciborium 慣例與本節順序。驗收標準見 §10。

---

## 9. 已知缺口（Known Gaps）與 Non-Goals

> ✅ 已關閉（本批）：**公開 `pack_case` / `CasePayload` helper** 與 **canonical `bundle_hash` 凍結**
> （`.case` 的 `{streams}` 信封現由公開型別 `fcb::case::CasePayload` 統一；`pack_case` 一步封裝並自動
> 帶入 canonical `bundle_hash`，§5／§8）；以及 **`fcb.netflow.v1` / `fcb.json.v1` 記錄 schema 凍結**
> （data-model §3.2／§3.3，`stream_types.rs` round-trip 測試）。

### 已知缺口（誠實標註，尚未實作 / 未凍結）

- **plugin registry 未實作（消費端概念）。** `DecodedStream.is_builtin == false` 的註解提到「或一個
  registered plugin」（`evidence.rs` 的 `DecodedStream.is_builtin` doc comment），但 crate 內**沒有任何
  registry 程式碼**——plugin 派發是消費端（workbench／審閱平台）的事，不在 codec 範圍。codec 只回傳 `is_builtin` 旗標。
- **reader 端 payload 多出 manifest 未列的 stream，行為未測。** `decode_streams` 是 **manifest 驅動**（只迭代
  manifest、用 `id` 去 payload 找對應，`evidence.rs` 的 `decode_streams`）；反過來 payload 若多出 manifest 沒列的
  stream，在 reader 端會被**靜默忽略**，且**沒有測試斷言**這個行為是否刻意（未證實）。相對地，manifest 列了但
  payload 缺對應 stream → `Malformed("payload missing stream {id}")`（同樣在 `decode_streams`）。**注意這只是
  reader 路徑的缺口**；producer 端的 `pack_case` 反而會嚴格擋下多餘 stream（見下一條）。
- **`manifest.records` 僅在 reader 端不核對；producer `pack_case` 已強制（reader-only gap）。** 低階的
  `bundle.rs` `pack_bytes` 路徑只是把 `meta` 當 opaque CBOR 原樣封裝，**不會**拿 `records` 計數和實際記錄數核對；
  但**官方 producer 入口 `case.rs` 的 `pack_case`** 在封裝前會呼叫 `check_manifest_matches_payload`，**強制**
  manifest 與 payload 的 **stream-id 集合雙向相等**＋**逐 stream 的 `records` 計數一致**，任一不符（計數不對、payload
  多餘 stream、manifest 多宣告 stream、重複 id）都會以 `Malformed` 拒絕（`case.rs` 內有對應的 reject 測試）。
  缺口因此**窄化為 reader 端**：`decode_streams` 開封時仍不重新核對 `records` 計數，消費端應以**開封後 payload**
  解出的記錄為準推導筆數，不要信 peek 階段宣告的 `records`。

### 設計取捨（by-design caveat，非缺口）

- **`bundle_hash` 是明文 payload 的確認 oracle（低熵情境）。** `bundle_hash` 是**明文 payload 的 SHA-256**、
  存在不需 passphrase 就能讀的明文 header 裡（§5）。對**低熵／可猜的 payload**，能猜中 payload 的人可藉這個
  雜湊**確認**猜測。高熵／大型 payload 不受影響。這是內容定址綁定的固有取捨，非漏洞。
- **binding 對 re-pack 敏感——發題後請凍結 payload。** 因 `bundle_hash` 是內容定址，case payload **任何**
  重新封裝（即使只改一個 byte）都得到新雜湊，讓既有 submission 的 binding 變成 `evidence-version-mismatch`
  （data-model §7）。要避免誤判，請在發題後**凍結** case payload，別重 pack。

### pack 邊界數值確定性契約（fcb-wasm）

- JS/wasm 經 `packCase`／`packSubmission` 打包時，記錄裡**絕對值超過 `2^53 - 1`（= 9007199254740991）的
  整數**若以普通 JS `number` 提供，serde 會編成 CBOR float、和原生整數作者的 canonical payload／雜湊發散；
  故 pack 邊界會以 `malformed` **拒絕**它，要求改用 `BigInt`（無損編成 CBOR 整數，最高到 `u64::MAX`；超過
  u64 會在 deserialize 階段失敗）。safe-range 整數與真正的小數 float 照常接受、且確定。case stream 記錄與
  submission 的 notes/report/activity 都套用此契約（`fcb-wasm/src/lib.rs`，`check_numeric_determinism`）。

### Non-Goals（本格式刻意不做的事）

- **不驗證 `aead.algo`。** 只認 `kdf.algo == "argon2id"`（§4）。
- **`container_version` 不做 parse-path 分派。** 目前只有 v1 一種佈局，欄位保留給未來（§1）。
- **codec 不解讀 `meta` 的語意。** container 層把 `meta` 當 opaque CBOR；stream/task/submission 的
  意義由上層（data-model）負責。

---

## 10. 相依套件版本與相容性驗收

`crates/fcb/Cargo.toml`：

| 套件 | 版本 | 用途 |
|------|------|------|
| `serde` | 1（features=["derive"]） | CBOR struct 衍生。 |
| `ciborium` | 0.2 | CBOR 編解碼（header、meta、payload）。 |
| `argon2` | 0.5 | Argon2id KDF（`Algorithm::Argon2id`、`Version::V0x13`）。 |
| `chacha20poly1305` | 0.10 | XChaCha20-Poly1305 AEAD。 |
| `ruzstd` | 0.8 | 純 Rust zstd（編碼僅 `Fastest`）。 |
| `sha2` | 0.10 | SHA-256（KCV、`bundle_hash`）。 |
| `thiserror` | 2 | error 衍生。 |
| `getrandom` | 0.2（wasm 加 `js` feature） | salt / nonce 隨機來源。 |
| `hex`（dev-only） | 0.4 | golden vector 的 hex 編解碼。 |

**相容性驗收：** 非 Rust 實作只要產出的位元組能通過 `cargo test -p fcb`，即視為相容——特別是
`case_vector_is_byte_stable`（`hex::encode(build_case()) == FROZEN_CASE_HEX`）、
`work_vector_is_byte_stable` 與
`frozen_case_vector_decodes_to_expected_structure`（皆在 `crates/fcb/tests/vectors.rs`）。round-trip schema
凍結另見 `stream_types.rs`（`syslog_v1_records_round_trip_byte_faithfully` 等）。
