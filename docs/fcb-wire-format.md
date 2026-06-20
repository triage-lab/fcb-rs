# FCB 線上格式（wire format）

> **先講脈絡（為什麼有這個格式）。** FCB（Forensic Case Bundle）是一個數位鑑識**出題／調查平台**的
> 「證物＋題目」打包格式。流程是：**case builder**（建構器）把證物與題目打包成 `.case` 發給學生 →
> 學生在 **browser workbench**（瀏覽器調查台）解鎖、調查、作答 → 學生端產出 `.casework`（學生作品）→
> **教師審閱平台**讀回 `.casework` 批改。因為 `.case` 會散佈給多名學生、又要能在沒輸入 passphrase 前
> 先顯示 case 資訊，所以才需要本檔描述的這套「明文 header + 加密 payload」信封、以及 `case_id` / `task` /
> 答案安全等欄位。脈絡更完整見 [`README.md`](./README.md) 與 `crates/fcb/src/lib.rs:3-7`。

`.case`（教師指派的題目）與 `.casework`（學生作品）共用同一個 container 信封。本文件描述
**byte-level 外層佈局**、**明文 header 的 CBOR 編碼規則**，以及 **compress-then-encrypt 密碼學管線**。
語意層資料結構（stream 記錄 schema、TaskSpec、Submission 等）放在
[`fcb-data-model.md`](./fcb-data-model.md)，本檔只在「信封層」交代它們長什麼樣、塞在哪裡。

權威來源（衝突時以前者為準）：

1. 參考實作 `crates/fcb/src/{container,crypto,compress,bundle,binding,cbor,error}.rs`
2. byte-exact golden vectors `crates/fcb/tests/vectors.rs`、round-trip `crates/fcb/tests/stream_types.rs`
3. 行為契約 `openspec/specs/fcb-*`（共 **7** 個 capability，逐一列表見
   [`README.md`](./README.md) §「7 個 capability spec」，本檔不重述以免漂移）

> 完整的權威來源優先序與 7 個 capability 清單以 [`README.md`](./README.md) 為單一出處；本檔聚焦信封層
> byte／crypto 細節，三份文件衝突時一律回上面三層原始來源。

> **整數一律 little-endian（LE）。** 字串為 UTF-8。本檔每一條 byte / 數值 / 行為宣稱都能對應到
> `crates/fcb` 原始碼（`file:line`）或具名 golden vector。

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
        ▲ header 明文、未被 AEAD 認證          ▲ 只有這層被 AEAD 認證
```

兩個設計直覺先講在前面，後面各節都圍著它打轉：

- **header 為什麼明文？** 因為解鎖前你就需要 KDF 的 `salt`/`params` 與 AEAD 的 `nonce` 才能推 key；
  這些本來就不是秘密。也因此消費端（browser workbench / 教師審閱平台）能在「還沒輸入 passphrase」
  時先 `peek_header` 顯示 case 資訊。代價是：**明文 header 不被 AEAD 認證**（見 §3 安全特性）。
- **payload 為什麼先壓再加密？** 密文近似亂數、壓不動；先 zstd 壓縮重複性高的 log 再 AEAD 封裝，
  才省得到空間（見 §3）。

---

## 1. Container 佈局（byte-level）

```text
magic(4) | KIND(u8) | container_version(u16 LE) | hdr_len(u32 LE)
         | header (hdr_len 個 byte，明文 CBOR)
         | payload (其餘全部；= AEAD(zstd(plaintext_payload)))
```

來源：`container.rs` 的 module doc（`container.rs:4-10`）與 `write_container`
（`container.rs:137-147`）/ `read_container`（`container.rs:180-214`）/ `peek_header`
（`container.rs:151-176`）。固定前綴恰 `4 + 1 + 2 + 4 = 11` bytes（`container.rs:139`）。

| 偏移 | 欄位 | 大小 | 值 | 來源 | 說明 |
|------|------|------|-----|------|------|
| 0 | `magic` | 4 B | `0x89 0x46 0x43 0x42`（`\x89FCB`） | `container.rs:22, 140` | 首 byte `0x89` 仿 PNG：偵測 7/8-bit 傳輸損壞、避免文字碰撞。**版本不烤進 magic**（`container.rs:19-21`）。 |
| 4 | `KIND` | 1 B | `1`=`.case`、`2`=`.casework` | `container.rs:41-44, 141` | `BundleKind`。其他值 → `Malformed("unknown KIND byte {other}")`（`container.rs:52`，placeholder 名為 `{other}`）。 |
| 5 | `container_version` | 2 B LE | 目前 `1`（`CONTAINER_VERSION`） | `container.rs:25, 142` | 保留給未來 parse-path 分派；**目前不驗證**（見下方註）。 |
| 7 | `hdr_len` | 4 B LE | header CBOR 的位元組長度 | `container.rs:143` | 寫成 `hdr.len() as u32`，**未檢查是否超過 u32::MAX**（`as` 截斷；`container.rs:143`）。 |
| 11 | `header` | `hdr_len` B | 明文 CBOR（見 §2） | `container.rs:144` | KDF salt/params 與 AEAD nonce 必須在「還沒 key」時就讀得到，故刻意明文。 |
| 11+`hdr_len` | `payload` | 其餘 | `AEAD(zstd(plaintext))`（見 §3） | `container.rs:145, 207` | 在 container 層**逐位元組原樣寫入／讀出（verbatim）**，壓縮加密在 compress/crypto 層。 |

寫入順序固定為 magic → KIND → container_version → hdr_len → header → payload
（`container.rs:140-145`）。

> **`container_version` 目前不分派也不驗證。** `read_container` 讀進 `container_version` 後保留在
> `Container` 結構，但原始碼明文註解「reserved for future parse-path dispatch; v1 is the only known
> layout today」（`container.rs:205-206`）。即使 `container_version != 1` 也不報錯、照 v1 解析。
> 控制相容性的是 **`header.min_reader`**（見 §2 / §6），不是 `container_version`。

### 兩個讀取進入點：`peek_header` vs `read_container`

| 函式 | 用途 | 是否複製 payload | 檢查項 | 來源 |
|------|------|------------------|--------|------|
| `peek_header(bytes)` | **解鎖前**只讀 header，顯示 case 資訊（不需要 passphrase） | 否 | magic / KIND / `min_reader` | `container.rs:151-176` |
| `read_container(bytes)` | 解析完整 frame，交給 `open_bytes` 解密 | 是（`bytes[pos..]`） | 同上，外加保留 `kind` / `container_version` | `container.rs:180-214` |

兩者檢查順序一致：magic → KIND → container_version（`peek` 丟棄、`read` 保留）→ `hdr_len` →
header 範圍 → header CBOR → `min_reader`。`min_reader > READER_VERSION`（目前 `1`，`container.rs:29`）
→ `UnsupportedVersion`（`container.rs:169-173, 199-203`）。

`read_u16` / `read_u32` 越界 → `Malformed("truncated u16"/"truncated u32")`
（`container.rs:112-126`）；header 範圍越界 → `Malformed("header length out of bounds")`
（`container.rs:165, 194`）。

### Golden vector 前綴拆解

逐位元組對照 `crates/fcb/tests/vectors.rs` 凍結的 hex（解碼自字面字串，已 byte-for-byte 驗證）。

`FROZEN_CASE_HEX`（`.case`，全長 578 bytes，`vectors.rs:28`）：

```text
89 46 43 42   magic
01            KIND = 1 (.case)
01 00         container_version = 1   (u16 LE)
dc 01 00 00   hdr_len = 0x000001dc = 476   (u32 LE)
a8 ...        header CBOR：map(8 entries)…（接 §2）
              （header 476 B 之後是 91 B 的 AEAD(zstd(payload)))
```

`FROZEN_WORK_HEX`（`.casework`，全長 423 bytes，`vectors.rs:29`）：

```text
89 46 43 42   magic
02            KIND = 2 (.casework)
01 00         container_version = 1
25 01 00 00   hdr_len = 0x00000125 = 293   (u32 LE)
a8 ...        header CBOR：map(8 entries)…
              （header 293 B 之後是 119 B 的 AEAD(zstd(payload)))
```

兩者 header 都以 `a8` 起頭，因為 `Header` 永遠是 8 欄位的 CBOR map（`container.rs:83-101`，含 `#[derive(…Serialize, Deserialize)]` 行）。
總長拆解：`.case` = 11 前綴 + 476 header + 91 payload = 578；`.casework` = 11 + 293 + 119 = 423。

> ⚠️ **凍結向量用的是「測試用快速 cost」，不是 production 預設。** 凍結 header 內的 Argon2 cost 是
> `m_cost=32`（CBOR `18 20`）/ `t_cost=1` / `p_cost=1`（`vectors.rs:49-55`），**不是** 函式庫預設的
> `19456 / 2 / 1`（`bundle.rs:16-18`）。`build()` 為了讓測試跑得快才寫死小 cost；正式打包用 §4 的預設。
> 同理 `bundle_hash = "sha256:deadbeef"` 是占位假值（`vectors.rs:63`），不是真 SHA-256。

---

## 2. Header（明文 CBOR）

`header` 是 `ciborium::into_writer` 對 `Header` struct 的序列化結果（`encode_header`，
`container.rs:129-134`）。ciborium 把 struct 編成 **以欄位名為 text key 的 CBOR map**，
**key 順序即 Rust 欄位宣告順序**。`Header` 有 8 個欄位 → map header byte `a8`。

```text
Header = {                          // CBOR map(8) → 起頭 a8
  "header_schema_ver": u16,         // 目前 1（container.rs:86；寫入 bundle.rs:71）
  "min_reader":        u16,         // 目前 1（container.rs:88；寫入 bundle.rs:72）
  "case_id":           text,        // 教師指派、穩定的題目識別碼（container.rs:90）
  "bundle_hash":       text,        // 綁定證物版本，格式 "sha256:<hex>"（container.rs:92；見 §5）
  "kdf":  { … 5 欄位 → a5 },         // 見下（container.rs:93）
  "aead": { … 2 欄位 → a2 },         // 見下（container.rs:94）
  "key_check": [u8…],               // 32 bytes，見 §4（container.rs:98）
  "meta": <任意 CBOR>               // .case={streams,task}；.casework={}（空 map）。見下（container.rs:100）
}
```

> **沒有 `#[serde(rename …)]` 或 `rename_all`** 在 `Header`/`KdfParams`/`AeadParams` 上
> （`container.rs:58-101`），所以 CBOR key = Rust 欄位名原樣。`rename` 之類的陷阱出現在 `meta`
> 裡的子結構（見下方「ciborium 慣例與陷阱」）。

### 欄位表（含 CBOR key 與來源）

| # | 欄位 | Rust 型別 | CBOR key | 來源 |
|---|------|-----------|----------|------|
| 1 | `header_schema_ver` | `u16` | `"header_schema_ver"` | `container.rs:85-86` |
| 2 | `min_reader` | `u16` | `"min_reader"` | `container.rs:87-88` |
| 3 | `case_id` | `String` | `"case_id"` | `container.rs:89-90` |
| 4 | `bundle_hash` | `String` | `"bundle_hash"` | `container.rs:91-92` |
| 5 | `kdf` | `KdfParams` | `"kdf"` | `container.rs:93` |
| 6 | `aead` | `AeadParams` | `"aead"` | `container.rs:94` |
| 7 | `key_check` | `Vec<u8>` | `"key_check"` | `container.rs:95-98` |
| 8 | `meta` | `ciborium::value::Value` | `"meta"` | `container.rs:99-100` |

**`kdf` = `KdfParams`（巢狀 map(5) → `a5`，`container.rs:58-70`）：**

| 欄位 | 型別 | CBOR key | 凍結向量值 |
|------|------|----------|-----------|
| `algo` | `String` | `"algo"` | `"argon2id"`（`vectors.rs:50`） |
| `salt` | `Vec<u8>` | `"salt"` | array(16)，起頭 `90`（`vectors.rs:51`） |
| `m_cost` | `u32`（KiB） | `"m_cost"` | `32` = `18 20`（測試 cost，`vectors.rs:52`） |
| `t_cost` | `u32`（迭代） | `"t_cost"` | `1` = `01`（`vectors.rs:53`） |
| `p_cost` | `u32`（平行度） | `"p_cost"` | `1` = `01`（`vectors.rs:54`） |

**`aead` = `AeadParams`（巢狀 map(2) → `a2`，`container.rs:72-79`）：**

| 欄位 | 型別 | CBOR key | 凍結向量值 |
|------|------|----------|-----------|
| `algo` | `String` | `"algo"` | `"xchacha20poly1305"`（`vectors.rs:66`） |
| `nonce` | `Vec<u8>` | `"nonce"` | array(24)，起頭 `98 18`（`vectors.rs:67`） |

正式打包時（`bundle.rs:57-84`）：`kdf.algo` 寫死 `"argon2id"`、`kdf.salt` = 16 bytes 隨機、
cost 取 `BundleParams` 的 `m_cost`/`t_cost`/`p_cost`（預設 `19456`/`2`/`1`，`bundle.rs:16-18, 48-50`）；
`aead.algo` 寫死 `"xchacha20poly1305"`、`aead.nonce` = 24 bytes 隨機。

### 一段連續的 header hex 拆解（可逐 byte diff）

上面的欄位表是分散的 marker 速查；下面把 `FROZEN_CASE_HEX`（`vectors.rs:28`）的 header **從頭連續拆到
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
  01                                   value 1
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
> 同一份 header 的完整 byte 仍以 `FROZEN_CASE_HEX` 字面字串為唯一真相（`vectors.rs:28`）。

### `meta` 欄位的形狀（信封層觀點）

`meta` 在 container 層是 opaque CBOR（`ciborium::value::Value`），上層才解讀。兩種 KIND 形狀不同：

| KIND | `meta` 形狀 | CBOR | 來源 |
|------|-----------|------|------|
| `.case` | `{ "streams": [StreamManifest…], "task": TaskSpec }` | map(2) → `a2` | `vectors.rs:85`、data-model §1 |
| `.casework` | `{}`（空 map） | `a0` | `vectors.rs:103`、data-model §0 |

`StreamManifest` 的 CBOR key 是 `id` / **`type`**（Rust 欄位 `stream_type` 標 `#[serde(rename = "type")]`，
`evidence.rs:29-30`）/ `records`。`StreamManifest`、`TaskSpec`、`Submission` 等深層 schema 與其
演進規則一律見 [`fcb-data-model.md`](./fcb-data-model.md)，本檔不重述以免漂移。

> **`task` 的互通陷阱（library vs 測試向量）：** 函式庫的 `TaskMeta.task` 是
> `Option<TaskSpec>` + `#[serde(default, skip_serializing_if = "Option::is_none")]`
> （`task.rs:47-56`），`None` 時**整個 `task` key 會被省略**；但凍結向量用的測試結構
> `CaseMeta` 把 `task` 寫成非 Option 欄位，**永遠寫出 `task`**（`vectors.rs:31-35, 85`）。所以凍結
> `.case` 的 meta 是 `a2`（含 `task`）；若以 `CaseMeta` 為樣板複製，會和 `task_to_meta(None)` 的輸出
> 不一致。讀取端兩種都容忍（`manifest_from_meta` 只讀 `streams`、`task_from_meta` 只讀 `task`）。

### ⚠️ ciborium 慣例與陷阱（互通關鍵）

非 Rust 的 case builder（建構器）想產出 byte-相容的 header，必須複刻 ciborium 0.2 的這些行為：

1. **`Vec<u8>` 編成 CBOR array of uint，不是 byte string。**
   `salt` / `nonce` / `key_check` 都是 `Vec<u8>`（`container.rs:63, 78, 98`）。ciborium 對 serde 的
   `Vec<u8>` 走 `serialize_seq`，產出 **CBOR array（major type 4），每個 byte 是一個 unsigned integer**，
   **不是** byte string（major type 2，起頭會是 `0x40`/`0x50`/`0x58`）。
   CBOR array（major type 4）的計數編碼規則（由 golden vector 驗證，`vectors.rs:28`）：

   | 元素數 | 起頭 byte | 例（本檔欄位） |
   |--------|-----------|----------------|
   | 0–23 | 單 byte `0x80 + n` | 16-byte salt → `0x90`（array(16)） |
   | 24–255 | `0x98` + 1-byte count | 24-byte nonce → `98 18`；32-byte key_check → `98 20` |

   且每個 byte 值 ≥ 24 時，array 元素本身也以 `18 xx` 兩 byte 表示（uint 的小整數編碼規則，完整門檻見下表）。
   **若把這些欄位寫成 byte string，ciborium 反序列化成 `Vec<u8>` 時預期 array → 不相容。**

   **CBOR unsigned integer（major type 0）的計數編碼規則。** 這套規則同時適用於兩處：（a）header 內的整數
   **value** 欄位——`m_cost` / `t_cost` / `p_cost`（`KdfParams`，`container.rs:64-69`）、以及 `meta` 內
   `StreamManifest.records`（`u64`，`evidence.rs:32`）、`fcb.syslog.v1` 的 `pid` / `severity` / `facility`；
   （b）上面 `Vec<u8>` array 內**每個值 ≥ 24 的 byte 元素**（如 `salt` / `nonce` / `key_check`）：

   | 數值 n | 起頭 byte | 後接 | 例 |
   |--------|-----------|------|-----|
   | 0–23 | 單 byte `0x00 + n`（值即本身） | — | `t_cost = 1` → `01`；byte `0x17`=23 → `17` |
   | 24–255 | `0x18` + 1-byte | 1 個 byte（大端等同單 byte） | 測試 cost `m_cost = 32` → `18 20`（`vectors.rs:52`）；byte `0x53`=83 → `18 53` |
   | 256–65535 | `0x19` + 2-byte（**big-endian**） | 2 個 byte | **production 預設 `m_cost = 19456`（`bundle.rs:16`）→ `19 4c 00`** |
   | 65536–2³²−1 | `0x1a` + 4-byte（big-endian） | 4 個 byte | `records = 100000` → `1a 00 01 86 a0` |
   | 2³²–2⁶⁴−1 | `0x1b` + 8-byte（big-endian） | 8 個 byte | （`u64` `records` 上界） |

   > ⚠️ **注意整數本身的多 byte 編碼是 big-endian**，與 §1 container 前綴的 `container_version` / `hdr_len`
   > （**little-endian**）相反——前綴是裸 LE 整數、不是 CBOR；header 內的整數才走 CBOR major type 0 的
   > big-endian。凍結向量唯一的整數 worked example 是測試 cost `m_cost = 32 → 18 20`（`vectors.rs:28, 52`）；
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

   例（皆取自 `FROZEN_CASE_HEX`，`vectors.rs:28`）：key `"header_schema_ver"`（17 bytes）→ `0x71`（=`0x60+17`）
   後接該 17 個 byte；key `"bundle_hash"`（11 bytes）→ `0x6b`；value `"argon2id"`（8 bytes）→ `0x68`；
   value `"acme-ir-2026-03"`（`case_id`，15 bytes）→ `0x6f`；value `"xchacha20poly1305"`（17 bytes）→ `0x71`。
   **注意 CBOR text string 與 CBOR array／byte string 是三種不同的 major type，count 前綴各自不同（text=`0x60+`、
   array=`0x80+`、byte string=`0x40+`），不可混用。**（text/array 計數編碼皆由 ciborium 0.2 決定，由
   golden vector `vectors.rs:28` 逐 byte 佐證。）

2. **struct → text-key map，key 順序 = 宣告順序。** 見上方 `Header`/`KdfParams`/`AeadParams`。
   text key 本身的長度前綴照上表編碼。

2b. **`ciborium::value::Value::Map` 依內部 `Vec<(key,value)>` 的插入順序原樣輸出，不做 canonical 排序。**
   這條與規則 2 不同：規則 2 的「順序=宣告序」只對 **struct** 成立；`header.meta`（`ciborium::value::Value`，
   `container.rs:100`）與 `fcb.syslog.v1` 的 `sd`（`Value::Map`，`stream_types.rs:36-39`）在型別上是
   `Value::Map`，**不是** struct，ciborium 一律照建構時 `Vec` 的插入序輸出。這直接決定 byte-exactness：
   - `header.meta`（`.case`）必須是 `{ "streams", "task" }` **這個順序**（`streams` 在前），golden vector 才對得上
     （`vectors.rs:85`）。若改用 `manifest_to_meta`（`evidence.rs:61`）＋ `task_to_meta`（`task.rs:54`）合併，
     必須自己保證 `streams` 在 `task` 之前。
   - `sd` 的外層 SD-ID（如 `"ex@32473"`）與內層 param（如 `"iut"`）順序皆由建構 `vec` 的插入序決定
     （`stream_types.rs:36-39`），消費端與生產端須一致。

3. **`#[serde(rename = "...")]` 改 key 名。** 例：`StreamManifest.stream_type` 標
   `#[serde(rename = "type")]` → CBOR key 為 `"type"`（`evidence.rs:29-30`）。

4. **`#[serde(rename_all = "lowercase")]` 的 enum → 小寫 text。** 例：`TaskSpec.report_mode`
   的 `ReportMode` → `"steps"` / `"freeform"`（見 data-model §1.2）。

5. **`Option<T>` + `skip_serializing_if = "Option::is_none"` → `None` 時整個 key 省略。**
   例：`TaskMeta.task`（`task.rs:47-56`，見上方 task 陷阱）。

> 上述行為由 ciborium 0.2 決定；原始碼層級只看得到 `Vec<u8>` / struct / `#[serde(...)]` 宣告，
> 確切 byte 由 golden vector（`vectors.rs:28`）佐證。

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
| `compress` | `compress_to_vec(data, CompressionLevel::Fastest)`，恆成功（`Ok`） | `compress.rs:29-31` |
| `decompress` | `StreamingDecoder` + `read_to_end`；**任何**錯誤 → `Corrupt` | `compress.rs:34-39` |
| `pack_payload` | 先 `compress`、再 `crypto::seal`（AEAD 包住 zstd frame） | `compress.rs:42-45` |
| `unpack_payload` | 先 `crypto::open_payload`、再 `decompress`（反序） | `compress.rs:48-56` |

- **壓縮**：標準 zstd frame（magic `0x28 0xB5 0x2F 0xFD`，`ZSTD_MAGIC`，`compress.rs:21`）。後端是
  **純 Rust `ruzstd` 0.8**（無 C/FFI，native 與 `wasm32-unknown-unknown` 共用同一份程式碼，
  `compress.rs:6-10`）。`ruzstd` 0.8 編碼**目前只實作 `Fastest`**（與 `Uncompressed`）；更高等級尚未
  實作、不要使用（`compress.rs:24-28`）。`Fastest` 對重複性高的 log 已有好壓縮比，且 frame 格式與
  更高等級相容、未來可無縫升級，並能與 C zstd reader 互通。
- **加密**：XChaCha20-Poly1305（`chacha20poly1305` 0.10）。key = §4 推導的 32 bytes、nonce =
  `header.aead.nonce`（24 bytes）、**無 AAD**（`seal`/`open` 只傳 `(nonce, plaintext)`，`crypto.rs:68, 77`）。
  Poly1305 tag 由 crate 附在密文尾端（密文長 = 明文 + 16-byte tag），FCB 沒有獨立 tag 欄位
  （`crypto.rs:68`）。

順序由測試 `order_is_compress_then_encrypt` 鎖定：解密 packed payload 得到的正是 zstd frame，
其前 4 byte == `ZSTD_MAGIC`、而外層密文前 4 byte != `ZSTD_MAGIC`（`compress.rs:89-105`，`fn` 起點）。

> ### 安全特性（務必知道）
>
> - **AEAD 只認證 payload。** 明文 header（含 `case_id`、`bundle_hash`、`meta` 裡的 manifest/task）
>   **未被 AEAD 認證**（沒有 AAD，`crypto.rs:68, 77`）——竄改 header 不會被 AEAD 偵測到。
> - **`key_check` 只認證「key 正確」**、**payload AEAD 只認證「payload 未被動」**。兩者各管一段。
> - 證物版本綁定靠 `bundle_hash`（見 §5、data-model §5），但 **codec 不會驗證 `bundle_hash` 是否
>   真的等於 payload 的 hash**——那是生產端（case builder）的責任，涵蓋範圍也由生產端定義。

---

## 4. 密碼學

來源：`crypto.rs`、常數 `bundle.rs`。

### KDF：Argon2id

```text
key(32 B) = Argon2id(
    password = passphrase 的 UTF-8 bytes,    // crypto.rs:37
    salt     = header.kdf.salt,              // crypto.rs:37
    m_cost   = header.kdf.m_cost,            // KiB
    t_cost   = header.kdf.t_cost,            // 迭代
    p_cost   = header.kdf.p_cost,            // 平行度
    version  = 0x13 (Argon2 v1.3),           // crypto.rs:34 Version::V0x13
    out_len  = 32,                           // crypto.rs:32 Some(KEY_LEN)
)
```

`derive_key`（`crypto.rs:28-40`）**只驗 `kdf.algo == "argon2id"`**，其餘回
`Malformed("unsupported KDF: {algo}")`（`crypto.rs:29-31`）。`Params::new` 失敗 →
`Malformed("bad argon2 params: …")`、`hash_password_into` 失敗 → `Malformed("argon2 failure: …")`。
`out_len = 32` 被綁定兩次（params `Some(KEY_LEN)` + 32-byte 輸出緩衝，`crypto.rs:32, 35`）。

| 常數 | 值 | 來源 |
|------|-----|------|
| `KEY_LEN` | 32 bytes | `crypto.rs:21` |
| `NONCE_LEN` | 24 bytes | `crypto.rs:23` |
| `SALT_LEN` | 16 bytes | `bundle.rs:19` |
| `DEFAULT_M_COST` | 19456（KiB） | `bundle.rs:16` |
| `DEFAULT_T_COST` | 2（迭代） | `bundle.rs:17` |
| `DEFAULT_P_COST` | 1（平行度） | `bundle.rs:18` |

> **`aead.algo` 不會被驗證。** `seal`/`open` 完全忽略它（`crypto.rs` 中沒有任何讀 `aead.algo` 的
> 程式碼），它只是描述性欄位。對照之下 `kdf.algo` **有**驗證。不要假設 `aead.algo` 像 `kdf.algo`
> 一樣有檢查。
>
> **cost 預設不寫死在 `derive_key`。** 它們存在 `bundle.rs` 的 `DEFAULT_*`，並**逐 bundle 寫進明文
> header**（`bundle.rs:58-64`），所以未來可調整而不破壞既有檔案——reader 一律用 header 裡的值。

### Key-Check Value（KCV）

```text
key_check = SHA256( "FCB-key-check-v1" || key )     // 32 bytes（domain 在前、key 在後）
```

`KCV_DOMAIN = b"FCB-key-check-v1"`（16 ASCII bytes，`crypto.rs:25`）。`key_check_value` 先
`update(KCV_DOMAIN)` 再 `update(key)`（**domain 前綴先 hash、key 後 hash**，`crypto.rs:44-47`）。
開檔時的二分法（`open_payload`，`crypto.rs:83-93`）：

- KCV 不符 → `WrongPassphrase`（密碼錯，`crypto.rs:89-90`）。
- KCV 相符但 AEAD 驗證失敗 → `Corrupt`（檔案被竄改／毀損，`crypto.rs:78, 92`）。

比較用 constant-time（`ct_eq`，`crypto.rs:96-105`）：長度不同立即回 `false`（長度非秘密）；
否則 XOR 累加、無提前跳出。**注意這是手寫累加器、非 `subtle` crate**，但對等長輸入仍是 constant-time。
發佈 key 的 hash 不會實質幫到攻擊者，因為 Argon2id 仍是暴力門檻。

### AEAD：XChaCha20-Poly1305

key = 上面 32 bytes、nonce = 24 bytes（`header.aead.nonce`）、**無 AAD**。`nonce_from` 強制
`len == 24`，否則 `Malformed("nonce must be 24 bytes, got N")`（`crypto.rs:54-62`）。tag 附在密文尾端
（標準 AEAD 輸出，`crypto.rs:68`）。

---

## 5. `bundle_hash` 與 binding（信封層摘要）

`compute_bundle_hash(bytes) -> "sha256:" + lower_hex(SHA256(bytes))`（`binding.rs`，小寫 hex）是低階
primitive，對任意 bytes 算 hash；golden vector 的 header 仍用占位假值 `"sha256:deadbeef"`（`vectors.rs`）。

`verify_binding(...)` 回傳三態 `Match` / `CaseMismatch` / `EvidenceVersionMismatch`；work key 慣例
`work_key(case_id) = "fcb:work:{case_id}"`。binding 的完整規則、`.casework` 的 `Submission` schema
與 binding 三態語意一律見 [`fcb-data-model.md`](./fcb-data-model.md) §5。

**canonical 定義（已凍結）：`bundle_hash = compute_bundle_hash(.case 的明文 payload bytes)`**，亦即壓縮／
加密**之前**的證物序列化位元組。如此同一份證物無論 salt/nonce 為何都得到相同 hash，學生作品
（`.casework`）才能可靠綁回特定證物版本。此定義由 `fcb::case::case_bundle_hash(&CasePayload)` 落實、
由 `pack_case` 自動帶入 header，並以 `vectors.rs` 的 `case_canonical_bundle_hash_is_frozen` 凍結。

---

## 6. 錯誤語意（`error.rs`）

`FcbError` 衍生 `PartialEq, Eq`（`error.rs:8`）。各變體的觸發點（含 file:line）：

| 變體 | 意義 | 主要觸發點 |
|------|------|-----------|
| `BadMagic` | 前 4 bytes 不是 FCB magic | `container.rs:153, 182` |
| `UnsupportedVersion { min_reader, supported }` | bundle 要求的 reader 版本比本 reader 新 | `container.rs:169-173, 199-203` |
| `Malformed(String)` | 結構性錯誤：truncated u16/u32、missing/unknown KIND、header out of bounds、bad header CBOR、unsupported KDF、bad argon2 params、nonce 長度錯、rng failure、CBOR encode 失敗… | `container.rs:52,115,123,132,159,165,168,194,198`；`crypto.rs:30,33,38,56`；`bundle.rs:23`；`cbor.rs:17,18,25,26,33` |
| `WrongPassphrase` | KCV 不符（密碼錯） | `crypto.rs:90` |
| `Corrupt` | KCV 相符但 AEAD 失敗、zstd frame 壞、或解密後 payload CBOR 解碼失敗 | `crypto.rs:78`；`compress.rs:35,37`；`cbor.rs:39` |

> **語意陷阱：CBOR 解碼失敗的兩條路不同。** header 的 CBOR 壞 → `Malformed("bad header CBOR")`
> （明文層，`container.rs:168, 198`）；但**解密後 payload** 的 CBOR 壞 → `cbor::decode` 對映成
> **`Corrupt`**（`cbor.rs:38-40`），語意是「解密/解壓後的 payload 壞了」。

由 golden vector 鎖定的錯誤行為：`open_bytes(CASE, "wrong")` → `WrongPassphrase`
（`vectors.rs:155-162`）；翻動 CASE 最後一個 byte → `Corrupt`（`vectors.rs:164-173`）。

---

## 7. Stream type 派發（信封層觀點）

`.case` 的每條 stream 在 manifest 裡帶一個 namespaced + versioned 的 `type`（CBOR key `"type"`）。
派發規則：

- 內建型別清單 `BUILTIN_STREAM_TYPES = ["fcb.syslog.v1", "fcb.netflow.v1", "fcb.json.v1"]`
  （`evidence.rs:18`）。`is_builtin_type(t)` = 清單是否包含 `t`（`evidence.rs:21-22`）。
- **這不是封閉清單。** 未知 / 第三方 type（如 golden vector 的 `acme.edr.v1`）仍解析成 first-class
  stream，只是 `is_builtin == false`（`vectors.rs:137-138`、`evidence.rs:77-93`），消費端落
  generic table / timeline fallback、**不致命**。
- 三個內建型別皆有凍結 schema：**`fcb.syslog.v1`**（data-model §3.1）、**`fcb.netflow.v1`**（5-tuple +
  bytes/packets + 時間區間，data-model §3.2）、**`fcb.json.v1`**（任意 CBOR map 通用容器，data-model §3.3）；
  各以 `crates/fcb/tests/stream_types.rs` 的 round-trip 測試凍結。

stream 記錄的逐欄 schema、演進／相容規則一律見 [`fcb-data-model.md`](./fcb-data-model.md) §3，本檔不重述。

---

## 8. 打包一個 `.case` 的完整順序（end-to-end，可照做）

把前面各節串成一條流程，對應 `bundle.rs` 的 `pack_bytes`（`bundle.rs:57-84`）。資料結構細節見
[`fcb-data-model.md`](./fcb-data-model.md)。

1. **準備證物**：每條 stream 整成 `StreamData { id, records }`；`records` 每筆形狀依該 stream 的
   `type`（如 `fcb.syslog.v1`，data-model §3.1）。
2. **組 manifest**：每條 stream 一筆 `StreamManifest { id, type, records = len(records) }`
   （CBOR key 是 `"type"`）。
3. **組 task spec**：`TaskSpec`（**零答案**，data-model §6）。
4. **`meta`**（明文）= CBOR `{ "streams": [manifest…], "task": TaskSpec }`（map(2) → 起頭 `a2`，`streams`
   在前；見 §2 規則 2b）。
5. **`payload_plain`** = CBOR `{ "streams": [StreamData…] }`。此信封是**單欄 struct** 公開型別
   `fcb::case::CasePayload { streams }`，ciborium 編成 map(1) → 起頭 **`a1`**，接 key `"streams"`
   （`67 73747265616d73`）再接 array(n) 的 `StreamData`。`.casework` 此處改為 `Submission`（7 欄 struct →
   map(7) → `a7`，`submission.rs`）。
6. **`bundle_hash`**：canonical = `compute_bundle_hash(payload_plain)`
   （= `"sha256:" + lower_hex(SHA256(payload_plain))`），**已凍結**（§5、§9）；由 `case::case_bundle_hash`
   落實、`pack_case` 自動帶入。
7. **隨機**產生 `salt`(16 B) 與 `nonce`(24 B)（`bundle.rs:60, 65`，`getrandom`）。
8. **key** = Argon2id(passphrase UTF-8, salt, m/t/p, version 0x13, out 32 B)（`bundle.rs:66`）。
9. **key_check** = SHA256(`"FCB-key-check-v1"` ‖ key)（32 B；**加密前**算好，`bundle.rs:67`）。
10. **compressed** = zstd `Fastest`(payload_plain)。
11. **ciphertext** = XChaCha20-Poly1305 seal(key, nonce, compressed)（**無 AAD**，`bundle.rs:68`）。
12. **header**（struct）= `{ header_schema_ver: 1, min_reader: 1, case_id, bundle_hash,
    kdf: {algo:"argon2id", salt, m_cost, t_cost, p_cost}, aead: {algo:"xchacha20poly1305", nonce},
    key_check, meta }`（`bundle.rs:70-82`）。
13. **hdr** = CBOR(header)（套用 §2 的 `Vec<u8>`→array、欄位順序、key 名等規則）。
14. **輸出位元組** = `magic(89 46 43 42) ‖ KIND(0x01) ‖ container_version(01 00) ‖
    len(hdr) as u32 LE ‖ hdr ‖ ciphertext`（`container.rs:140-145`）。

> `.casework` 相同，差別只在：`KIND = 0x02`、`meta = {}`（空 map）、`payload_plain = CBOR(Submission)`
> （見 data-model §4）。
>
> ⚠️ **`FROZEN_WORK_HEX` 凍結的不是 `Submission`。** 唯一的 `.casework` byte-stable 向量
> （`vectors.rs:29`）其 payload 是用**測試專用的 3 欄 `WorkPayload { case_id, bundle_hash, report }`**
> 建出來的（`vectors.rs:40-45, 96-104`），**不是** library 真正會寫的 7 欄 `Submission`
> （`submission.rs:25-40`）。換言之，目前**沒有任何 golden vector 釘住 `Submission` 的 on-disk 位元組**；
> `Submission` 只由 random-salt 的 `submission_random_round_trip`（`vectors.rs:175-189`）覆蓋——那只證
> round-trip、不證 byte-stability。重寫 codec 想驗 `Submission` 的 byte-exactness，須自行補向量。

> **想要可直接照抄、會編譯的範例？** Rust 端最小可跑的打包範例見
> `crates/fcb/tests/stream_types.rs:93-120`（`round_trip`：組 `StreamManifest` → `manifest_to_meta` →
> 自組 `CasePayload { streams }` → `cbor::encode` → `BundleParams::new` → `pack_bytes` → `open_bytes`），
> 以及 `README.md` §「給 case builder 作者的起手建議」的 `use` 匯入範例。

順序要點：`salt`/`nonce` 在推 key 前就要產生；`key_check` 在加密前算好；`header` 要等
`salt`/`nonce`/`key_check`/`bundle_hash`/`meta` 都備齊後才能序列化求 `hdr_len`（`header` 不含
`ciphertext`）。

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
  registered plugin」（`evidence.rs:49-50`），但 crate 內**沒有任何 registry 程式碼**——plugin 派發是
  消費端（workbench／審閱平台）的事，不在 codec 範圍。codec 只回傳 `is_builtin` 旗標。
- **payload 多出 manifest 未列的 stream，行為未測。** `decode_streams` 是 **manifest 驅動**（只迭代
  manifest、用 `id` 去 payload 找對應，`evidence.rs:77-93`）；反過來 payload 若多出 manifest 沒列的
  stream，會被**靜默忽略**，且**沒有測試斷言**這個行為是否刻意（未證實）。相對地，manifest 列了但
  payload 缺對應 stream → `Malformed("payload missing stream {id}")`（`evidence.rs:84`）。

### Non-Goals（本格式刻意不做的事）

- **不認證明文 header。** AEAD 沒有 AAD，header 竄改不會被 codec 偵測（§3）；防竄改要靠上層
  （例如把整檔簽章），不在 codec 範圍。
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
`case_vector_is_byte_stable`（`hex::encode(build_case()) == FROZEN_CASE_HEX`，`vectors.rs:106-109`）、
`work_vector_is_byte_stable`（`vectors.rs:111-114`）與
`frozen_case_vector_decodes_to_expected_structure`（`vectors.rs:116-139`）。round-trip schema
凍結另見 `stream_types.rs`（`syslog_v1_records_round_trip_byte_faithfully` 等）。
