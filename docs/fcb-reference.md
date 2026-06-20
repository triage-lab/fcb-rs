# FCB Reference（機器可解析精確規格速查）

> **本檔職責**：FCB（Forensic Case Bundle）所有結構的單一、無歧義、機器可解析 reference。
> 把 wire-format / data-model 濃縮成欄位表 / 型別 / CBOR 佈局 / 常數表 / 不變量 /
> golden-vector byte map / error 目錄 / 答案安全不變量 / 已知缺口 / 相依版本。
> 是兩份 narrative 文件（[`fcb-wire-format.md`](./fcb-wire-format.md) 外層信封、
> [`fcb-data-model.md`](./fcb-data-model.md) 內層資料結構）的「規格速查鏡像層」——
> narrative 著重「為什麼」，本檔提供「精確的事實與數字」。本檔以**精確表格為主**，並為了可獨立照做（self-contained）保留**極少量白話導語**與一節**端到端步驟**（§11）；深入的設計理由仍請看兩份 narrative。
>
> **同一事實只有一個權威數字。** 本檔與兩份 narrative 不可漂移；衝突時以本檔（及其引用的原始碼）為準。

## 權威來源優先序（衝突時以前者為準）

1. `crates/fcb/src/*.rs` 參考實作
2. `crates/fcb/tests/vectors.rs`、`crates/fcb/tests/stream_types.rs`（byte-exact golden vectors / round-trip）
3. `openspec/specs/fcb-*`（5 個 fcb capability，見 §0）
4. 既有 `docs/`（精修對象，**不是**真相來源）

每個 byte / 數值 / 行為宣稱都附原始碼或 golden vector 出處（`file:line` 或 vector 名）。

## 慣例

- **整數一律 little-endian（LE）。字串一律 UTF-8。**
- 「**case builder**（建構器）」= 產生 `.case` 的工具（可能不只是 CLI）。
- 「**消費端**」= browser workbench（學生）／教師審閱平台。
- 「**未證實**」= 無法從原始碼／golden vector 直接佐證的宣稱。

---

## 0. capability 對照

`openspec/specs/` 下共 **7 個** spec 目錄，其中 **fcb-\* 只有 5 個**，另 2 個非 fcb。
（三份 docs 對此已對齊：README 與兩份 narrative 皆作「7 個 capability、5 個 fcb-\*」並稱產生 `.case` 的工具為「case builder」。）

| spec 目錄 | 是否 fcb-* | 本檔對應小節 | 出處 |
|-----------|:---------:|--------------|------|
| `fcb-container-format` | ✔ | §1（container 佈局）、§4（crypto）、§5（compress）、§7（error） | `openspec/specs/fcb-container-format/spec.md` |
| `fcb-evidence-model` | ✔ | §3（meta + payload + stream dispatch） | `openspec/specs/fcb-evidence-model/spec.md` |
| `fcb-stream-types` | ✔ | §3.4（`fcb.syslog.v1`） | `openspec/specs/fcb-stream-types/spec.md` |
| `fcb-task-spec` | ✔ | §6（TaskSpec + 答案安全） | `openspec/specs/fcb-task-spec/spec.md` |
| `fcb-submission` | ✔ | §6.4（Submission）、§6.5（binding） | `openspec/specs/fcb-submission/spec.md` |
| `plugin-protocol` | ✘（非 fcb） | （消費端 plugin，不在本 codec 範圍） | `openspec/specs/plugin-protocol/spec.md` |
| `query-model` | ✘（非 fcb） | （消費端查詢，不在本 codec 範圍） | `openspec/specs/query-model/spec.md` |

事實：`fcb-* = 5 個`，`total spec = 7 個`，「7 個 fcb-* spec」**不存在**。出處：`openspec/specs/` (ls)。

---

## 1. Container 位元組佈局（on-disk envelope）

**白話：** 每個 FCB 檔案最前面是固定 11 bytes 的前綴（magic + 種類 + 版本 + header 長度），接著是「明文」CBOR header，最後是「加密過」的 payload。前綴與 header 不需要密碼就能讀，是設計使然——因為要先讀到 KDF 的 salt/參數與 nonce，才有辦法算出 key。

**權威來源：** `crates/fcb/src/container.rs`（module doc `container.rs:4-10`、`write_container` `container.rs:137-147`）。

```text
偏移   欄位                大小       值/規則
0      magic               4 B        89 46 43 42  (= "\x89FCB")
4      KIND                1 B        1=.case, 2=.casework
5      container_version   2 B (LE)   目前 1
7      hdr_len             4 B (LE)   header CBOR 的位元組長度
11     header              hdr_len B  明文 CBOR（見 §2）
11+hdr_len  payload        其餘全部    = AEAD(zstd(plaintext_payload))（見 §4、§5）
```

| 偏移 | 欄位 | 大小 | 寫入來源 | 出處 |
|------|------|------|----------|------|
| 0 | `magic` | 4 B | `out.extend_from_slice(&MAGIC)` | container.rs:140 |
| 4 | `KIND` | 1 B | `out.push(kind.to_u8())` | container.rs:141 |
| 5 | `container_version` | 2 B LE | `CONTAINER_VERSION.to_le_bytes()` | container.rs:142 |
| 7 | `hdr_len` | 4 B LE | `(hdr.len() as u32).to_le_bytes()` | container.rs:143 |
| 11 | `header` | `hdr_len` B | `out.extend_from_slice(&hdr)`（`hdr` = `encode_header`） | container.rs:144 |
| 11+hdr_len | `payload` | 其餘 | `out.extend_from_slice(payload)`（verbatim，本層不加工） | container.rs:145 |

寫入順序固定：magic → KIND → container_version → hdr_len → header → payload（container.rs:140-145）。
固定前綴 = 11 bytes（`4 + 1 + 2 + 4`，container.rs:139）。

### 1.1 常數表（container 層，實際數值）

| 常數 | 型別 | 實際值 | 出處 |
|------|------|--------|------|
| `MAGIC` | `[u8; 4]` | `[0x89, b'F', b'C', b'B']` = `89 46 43 42` | container.rs:22 |
| `CONTAINER_VERSION` | `u16` | `1` | container.rs:25 |
| `READER_VERSION` | `u16` | `1` | container.rs:29 |
| `BundleKind::Case` → byte | `u8` | `1` | container.rs:42-43 |
| `BundleKind::Work` → byte | `u8` | `2` | container.rs:44 |

- `0x89` 仿 PNG，偵測 7/8-bit 傳輸損壞、避免文字碰撞（container.rs:19-21）。
- **版本不烤進 magic**——`container_version` 是獨立欄位（container.rs:20-21）。
- 未知 KIND byte → `Malformed("unknown KIND byte {other}")`（container.rs:52）。

### 1.2 讀取路徑與不變量（含對應 error）

| 函式 | 行為摘要 | 出處 |
|------|----------|------|
| `peek_header` | 不複製 payload、解鎖前可讀；驗 magic/KIND/min_reader；`container_version` **讀但丟棄** | container.rs:151-176 |
| `read_container` | 解析完整 frame，保留 `kind`/`container_version`/`header`/`payload` | container.rs:180-214 |
| `write_container` | 序列化 header → 組裝前綴 → append payload | container.rs:137-147 |
| `read_u16` | 越界 → `Malformed("truncated u16")`；`u16::from_le_bytes` | container.rs:112-118 |
| `read_u32` | 越界 → `Malformed("truncated u32")`；`u32::from_le_bytes` | container.rs:120-126 |

讀取檢查順序（`peek_header` / `read_container` 相同）：

1. `len < 4 || bytes[0..4] != MAGIC` → `BadMagic`（container.rs:152-154, 181-183）。
2. 取 KIND；缺 → `Malformed("missing KIND")`；未知值 → `Malformed("unknown KIND byte {other}")`（container.rs:156-160, 52）。
3. 讀 `container_version`（`peek_header` 丟棄 container.rs:162；`read_container` 保留 container.rs:191）。
4. 讀 `hdr_len`（container.rs:163）。
5. `bytes[pos..pos+hdr_len]` 越界 → `Malformed("header length out of bounds")`（container.rs:164-166）。
6. `ciborium::from_reader` 解 header；失敗 → `Malformed("bad header CBOR: {e}")`（container.rs:167-168）。
7. `header.min_reader > READER_VERSION` → `UnsupportedVersion { min_reader, supported: READER_VERSION }`（container.rs:169-174, 199-204）。

**容器層明確界線（本層不做的檢查）：**

| 不檢查的事項 | 由誰負責 | 出處 |
|--------------|----------|------|
| salt / nonce / key_check **長度** | crypto 層（nonce 須 ==24，`nonce_from`） | container.rs（僅 `Vec<u8>`）；crypto.rs:54-62 |
| `kdf.algo` 值（須 `argon2id`） | crypto 層 `derive_key` | crypto.rs:29-31 |
| `aead.algo` 值 | **無人驗證**（描述性） | bundle.rs:77（寫入）；crypto 全層不讀 |
| `bundle_hash` 內容 | 生產端 | binding.rs:14-22（不驗證涵蓋範圍） |
| `header_schema_ver` | **不檢查**（只檢 `min_reader`） | container.rs:169（僅 `min_reader`） |
| `container_version` 值 | **不分派**（v1 是目前唯一已知佈局；即使 ≠1 也照 v1 解、不報錯） | container.rs:205-206 |

---

## 2. 明文 Header（CBOR text-key map）

**白話：** header 是把 Rust `Header` struct 用 ciborium 序列化的結果。ciborium 把 struct 編成「以欄位名為文字 key 的 CBOR map」，順序就是欄位宣告順序。8 個欄位 → CBOR map(8)，起頭 byte `a8`。

**權威來源：** `crates/fcb/src/container.rs:83-101`。**沒有** `#[serde(rename)]` / `rename_all`，故 CBOR key = Rust 欄位名原樣。

### 2.1 `Header`（8 欄位 → `a8`）

| # | 欄位 | Rust 型別 | CBOR key（text） | 寫入值（`bundle.rs`） | 出處 |
|---|------|-----------|------------------|----------------------|------|
| 1 | `header_schema_ver` | `u16` | `"header_schema_ver"` | `1` | container.rs:85-86；bundle.rs:71 |
| 2 | `min_reader` | `u16` | `"min_reader"` | `1` | container.rs:87-88；bundle.rs:72 |
| 3 | `case_id` | `String` | `"case_id"` | 由 `BundleParams` 帶入 | container.rs:89-90 |
| 4 | `bundle_hash` | `String` | `"bundle_hash"` | 由 `BundleParams` 帶入（格式 `"sha256:<hex>"`，見 §6.5） | container.rs:91-92 |
| 5 | `kdf` | `KdfParams` | `"kdf"` | 見 §2.2 | container.rs:93 |
| 6 | `aead` | `AeadParams` | `"aead"` | 見 §2.3 | container.rs:93-94 |
| 7 | `key_check` | `Vec<u8>` | `"key_check"` | 32 B KCV（見 §4.2） | container.rs:95-98 |
| 8 | `meta` | `ciborium::value::Value` | `"meta"` | `.case`={streams,task}；`.casework`={}（見 §3） | container.rs:99-100 |

- Derives：`Debug, Clone, PartialEq, Serialize, Deserialize`（**無 `Eq`**，因 `meta: Value` 不是 `Eq`）（container.rs:83）。

### 2.2 `KdfParams`（5 欄位 → `a5`）

| # | 欄位 | 型別 | CBOR key | 語意/單位 | 出處 |
|---|------|------|----------|-----------|------|
| 1 | `algo` | `String` | `"algo"` | KDF 識別碼，固定 `"argon2id"` | container.rs:60-61 |
| 2 | `salt` | `Vec<u8>` | `"salt"` | per-bundle 隨機 salt（16 B，編為 array of uint） | container.rs:62-63 |
| 3 | `m_cost` | `u32` | `"m_cost"` | Argon2 記憶體成本（**KiB**） | container.rs:64-65 |
| 4 | `t_cost` | `u32` | `"t_cost"` | Argon2 時間成本（**迭代次數**） | container.rs:66-67 |
| 5 | `p_cost` | `u32` | `"p_cost"` | Argon2 **平行度（lanes）** | container.rs:68-69 |

Derives：`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`（container.rs:58）。

### 2.3 `AeadParams`（2 欄位 → `a2`）

| # | 欄位 | 型別 | CBOR key | 語意 | 出處 |
|---|------|------|----------|------|------|
| 1 | `algo` | `String` | `"algo"` | AEAD 識別碼，固定 `"xchacha20poly1305"`（**描述性、不驗證**） | container.rs:75-76 |
| 2 | `nonce` | `Vec<u8>` | `"nonce"` | per-bundle 隨機 nonce（24 B，編為 array of uint） | container.rs:77-78 |

Derives：`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`（container.rs:73）。

---

## 2A. ciborium 編碼慣例與互通陷阱（必背）

| 規則 | 行為 | 範例 | 出處 |
|------|------|------|------|
| `Vec<u8>` → **CBOR array of uint**（major type 4），**不是** byte string（major type 2） | ciborium 對 serde `Vec<u8>` 走 `serialize_seq`，每個 byte 一個 unsigned integer | 16-byte salt 起頭 `0x90`（array(16)）後接 16 個 uint | container.rs:63/78/98；golden hex vectors.rs:28 |
| struct → **text-key map**，順序 = 宣告序 | 無 `rename_all` 時 key = Rust 欄位名 | `Header` → map(8) 起頭 `a8` | container.rs:83-101 |
| **text string（key 或 value）→ major type 3**，長度前綴見下表「CBOR text string 計數編碼」 | 所有欄位名、enum 小寫變體、`case_id`/`algo`/`format` 等 text value 皆走此規則；長度以 **UTF-8 byte 數**（非字元數）計 | `"header_schema_ver"` 17 chars → `0x71` 後接 17 byte；`"case_id"` 7 chars → `0x67` | golden hex vectors.rs:28 |
| `ciborium::value::Value::Map` → **依內部 `Vec<(key,value)>` 的插入順序原樣輸出**，**不做** canonical 排序 | 與 struct 不同：`Value::Map` 不套「宣告序」規則，順序由建構該 vec 的程式決定（直接決定 byte-exactness） | `header.meta`（`{streams,task}` 的順序）、`fcb.syslog.v1` 的 `sd`（外層 SD-ID、內層 param 順序）皆屬此類 | container.rs:99-100；stream_types.rs:36-39 |
| `#[serde(rename = "x")]` → CBOR key 改名 | `StreamManifest.stream_type` 標 `rename="type"` → key 為 `"type"` | 見 §3.1 | evidence.rs:30-31 |
| `#[serde(rename_all = "lowercase")]` enum → **小寫 text** | `ReportMode::Steps` → `"steps"` | 見 §6.3 | task.rs:22-27 |
| `Option<T>` + `skip_serializing_if = "Option::is_none"` → `None` 時**整個 key 省略** | 不寫 null、直接不出現 | `TaskMeta.task` = `None` → 無 `task` key | task.rs:49-50 |

### CBOR unsigned integer（major type 0）計數編碼（佐證所有整數欄位 + array 內每個 byte 元素）

所有純量整數欄位（`m_cost` / `t_cost` / `p_cost` / `records` / `pid` / `severity` / `facility` / `header_schema_ver` / `min_reader`）皆走 major type 0；`Vec<u8>`（salt/nonce/key_check）array 內**每個 ≥24 的 byte 元素**也照同一規則編成 `0x18 xx`。多 byte 長度前綴一律 **big-endian**（與 LE 的 container 前綴相反）。

| 值 n | 起頭編碼 | 範例 |
|------|----------|------|
| 0–23 | 單 byte `0x00 + n` | `1`（`min_reader`）→ `0x01`；`23` → `0x17` |
| 24–255 | `0x18` + 1-byte（big-endian） | `m_cost=32` → `18 20`（golden，vectors.rs:52）；`24` → `18 18`；`255` → `18 ff` |
| 256–65535 | `0x19` + 2-byte（big-endian） | **`m_cost=19456`（生產預設，bundle.rs:16）→ `19 4c 00`**；`256` → `19 01 00` |
| 65536–2³²−1 | `0x1a` + 4-byte（big-endian） | `65536` → `1a 00 01 00 00` |
| 2³²–2⁶⁴−1 | `0x1b` + 8-byte（big-endian） | （`u64 records` 超大值；FCB 內無此量級） |

> ⚠️ 生產預設 `m_cost=19456` 落在 256–65535 區段 → `19 4c 00`，**不是** golden vector 那個快測值 `18 20`（`m_cost=32`）。重實作者照 golden hex 唯一的整數例（`18 20`）無法外推 19456，故此表獨立列出。佐證：bundle.rs:16（`DEFAULT_M_COST=19456`）、vectors.rs:52（frozen `18 20`）。

### CBOR array 計數編碼（佐證 `Vec<u8>` 起頭 byte）

| 元素數 n | 起頭編碼 | 範例 |
|----------|----------|------|
| 0–23 | 單 byte `0x80 + n` | 16-byte salt → `0x90` |
| 24–255 | `0x98` + 1-byte count | 24-byte nonce → `98 18`；32-byte key_check → `98 20` |

### CBOR text string（major type 3）計數編碼（佐證每個 text key / value 的起頭 byte）

UTF-8 byte 長度 = `len`（**非字元數**）。所有 struct 欄位名、`#[serde(rename_all=lowercase)]` enum 變體、`case_id` / `bundle_hash` / `algo` / `format` 等 text value 一律照此編碼。

| byte 長度 len | 起頭編碼 | 範例（皆來自 `FROZEN_CASE_HEX`，vectors.rs:28） |
|---------------|----------|------|
| 0–23 | 單 byte `0x60 + len` | `"kdf"`（3）→ `0x63`；`"steps"`（5）→ `0x65`；`"case_id"`（7）→ `0x67`；`"bundle_hash"`（11）→ `0x6b`；`"sha256:deadbeef"`（15）→ `0x6f`；`"header_schema_ver"`（17）→ `0x71`；`"xchacha20poly1305"`（17）→ `0x71` |
| 24–255 | `0x78` + 1-byte len | FCB header 內目前**無**此長度的 text（所有 key/value 皆 ≤17 byte）；舉例：24-byte text → `78 18` |
| 256–65535 | `0x79` + 2-byte len（**big-endian**） | （FCB 內無此長度的 text，列出供完整性） |

> ⚠️ 修正常見誤解：上表「0–23」涵蓋到 **23 byte** 為止，故 `"header_schema_ver"`（17 byte）走 `0x60+17 = 0x71`，**不是** `0x78`。只有 **≥24 byte** 才改用 `0x78`+1-byte count。佐證：golden hex `FROZEN_CASE_HEX` 開頭 `a8 71 6865616465725f736368656d615f766572`（`a8`=map(8)、`71`=text(17)、後接 `"header_schema_ver"` 的 17 個 UTF-8 byte），vectors.rs:28。

> **互通陷阱：** 非 Rust 的 case builder 若把 `salt`/`nonce`/`key_check` 寫成 CBOR byte string
> （起頭 `0x50`/`0x58`），ciborium 反序列化成 `Vec<u8>` 時預期 array，將**不相容**。**務必寫成 array of uint。**
> （此編碼行為由 ciborium 0.2 決定；原始碼層僅 `Vec<u8>` 宣告 + golden vector 證實。）

---

## 3. Evidence 模型：meta、payload 信封、stream 派發

**權威來源：** `crates/fcb/src/evidence.rs`。

### 3.0 兩個 CBOR 形狀（一明文、一加密）

```text
明文 header.meta（.case）= map(2)→a2  { "streams": [ StreamManifest, ... ], "task": TaskSpec }
                                                              (task 由 task 層加，見 §6)
加密 .case payload       = map(1)→a1  { "streams": [ StreamData, ... ] }   // CasePayload 單欄 struct
明文 header.meta（.casework）= map(0)→a0  {}（空 map）
加密 .casework payload   = map(7)→a7  Submission（7 欄，見 §6.4）
```

> 每個會落到 wire 的 CBOR map 起頭 byte 一覽（major type 5 = `0xa0 + n`，n ≤ 23）：
> `Header`=map(8)→`a8`（§2.1）、`KdfParams`=map(5)→`a5`（§2.2）、`AeadParams`=map(2)→`a2`（§2.3）、
> `.case meta`=map(2)→`a2`、`.case payload 信封`=map(1)→`a1`、
> `StreamManifest`=map(3)→`a3`（§3.1）、`StreamData`=map(2)→`a2`（§3.2）、
> `.casework meta`=map(0)→`a0`、`Submission`=map(7)→`a7`（§6.4）、`Student`=map(2)→`a2`（§6.4）、
> `TaskSpec`=map(3)→`a3`（§6.2）、`TaskStep`=map(3)→`a3`（§6.3）。
> 出處：`CasePayload{streams}` 單欄 struct → map(1)（vectors.rs:86-92、stream_types.rs:18-21）；`Student{id,name}` → map(2)（submission.rs:17-21）。

- `StreamManifest`（manifest）只在**明文** meta，攜帶 `type`，可在解鎖前讀。
- `StreamData`（記錄）只在**加密** payload，**不**攜帶 `type`，靠 `id` 與 manifest join。
- 出處：evidence.rs:30-32, 61-65（meta）；evidence.rs:38-39（payload）；vectors.rs:86-92。

> **meta 是 `Value::Map`、entry 順序決定 byte-exactness（§2A）。** golden CASE meta 為 `a2` 且 `streams` 在前、`task` 在後（vectors.rs:85，源自 `CaseMeta` struct 宣告序）。若改用 `evidence::manifest_to_meta` + `task::task_to_meta` 合併兩個片段，合併出的 `Value::Map` 順序取決於**合併程式的插入序**、而非 struct 宣告序——須確保 `streams` 在 `task` 之前，才能對齊 golden vector。出處：evidence.rs:61-65、task.rs:54-56、vectors.rs:31-35,85。

### 3.1 `StreamManifest`（header manifest 一筆 → map(3)）

| Rust 欄位 | 型別 | serde attr | CBOR key | 必填？ | 出處 |
|-----------|------|------------|----------|:------:|------|
| `id` | `String` | （無） | `"id"` | ✔ | evidence.rs:28 |
| `stream_type` | `String` | `#[serde(rename = "type")]` | **`"type"`** | ✔ | evidence.rs:30-31 |
| `records` | `u64` | （無） | `"records"` | ✔ | evidence.rs:32 |

Derives：`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`（evidence.rs:26）。無 `default` → 三欄全必填。
> ⚠️ CBOR key 是 **`type`** 不是 `stream_type`。

### 3.2 `StreamData`（加密 payload 一筆 → map(2)）

| Rust 欄位 | 型別 | CBOR key | 出處 |
|-----------|------|----------|------|
| `id` | `String` | `"id"` | evidence.rs:38 |
| `records` | `Vec<Value>`（`ciborium::value::Value`） | `"records"` | evidence.rs:39 |

Derives：`Debug, Clone, PartialEq, Serialize, Deserialize`（**無 `Eq`**，因 `Vec<Value>`）（evidence.rs:36）。**不含 `type`**——type 只在 manifest，payload 靠 `id` join。

### 3.3 `DecodedStream`（join 結果，**僅記憶體、不序列化到 wire**）

| Rust 欄位 | 型別 | 出處 |
|-----------|------|------|
| `id` | `String` | evidence.rs:46 |
| `stream_type` | `String` | evidence.rs:47 |
| `records` | `Vec<Value>` | evidence.rs:48 |
| `is_builtin` | `bool` | evidence.rs:49-51 |

Derives：`Debug, Clone, PartialEq`（**非** Serialize/Deserialize）（evidence.rs:44）。
`is_builtin == false` ⇒ 無內建 handler，消費端落 generic table/timeline fallback（或註冊的 plugin）。

### 3.4 stream type 派發（built-in 集合）

| 常數 | 實際值 | 型別 | 出處 |
|------|--------|------|------|
| `BUILTIN_STREAM_TYPES` | `["fcb.syslog.v1", "fcb.netflow.v1", "fcb.json.v1"]`（順序固定，3 個） | `pub const &[&str]` | evidence.rs:18 |

- `is_builtin_type(t)` = `BUILTIN_STREAM_TYPES.contains(&t)`——**精確字串比對**（大小寫敏感，無 namespace/版本模糊比對）。`fcb.syslog.v2` → `false`。出處：evidence.rs:21-23。
- **非封閉清單**：此集合只區分「有內建 handler」vs「需 plugin / generic fallback」，新 type **不需**列入。出處：evidence.rs:15-17。
- **未知 type 不致命**：仍 decode，只是 `is_builtin = false`。出處：evidence.rs:77-93；測試 `unknown_type_does_not_abort_other_streams` evidence.rs:136-152（`#[test]` 屬性在 135）。
- **唯一結構性 error**：manifest 有列但 payload 找不到對應 `id` → `Malformed("payload missing stream {id}")`。反向（payload 有但 manifest 無）**不檢查**，多餘 payload stream 被靜默忽略（iteration 由 manifest 驅動）。出處：evidence.rs:81-84, 78-92。
- 只有 `fcb.syslog.v1` 有凍結記錄 schema（§3.5）；`fcb.netflow.v1` / `fcb.json.v1` **無記錄 schema 定義**（已知缺口，見 §9）。出處：evidence.rs:18；docs/fcb-data-model.md:210-214。

### 3.4a evidence 函式

| 函式 | 行為 | error | 出處 |
|------|------|-------|------|
| `manifest_to_meta(&[StreamManifest]) -> Result<Value>` | 包成 `StreamsMeta{streams}` → `{ "streams": [...] }` | `Malformed`（cbor encode 失敗） | evidence.rs:61-65 |
| `manifest_from_meta(&Value) -> Result<Vec<StreamManifest>>` | 解 `StreamsMeta`，回 `streams`；明文可讀；容忍多餘 key（如 `task`）；缺 `streams` → 空 vec | `Malformed`（cbor decode 失敗） | evidence.rs:69-72 |
| `decode_streams(&[StreamManifest], &[StreamData]) -> Result<Vec<DecodedStream>>` | 按 manifest 順序 join；保序；長度 == manifest 長度 | `Malformed("payload missing stream {id}")` | evidence.rs:77-93 |

> `StreamsMeta`（私有 wrapper）：`{ streams: Vec<StreamManifest> }`，`#[serde(default)]` → 缺 `streams` key 解成空 vec、不報錯。出處：evidence.rs:54-58。

### 3.5 `fcb.syslog.v1` 記錄 schema（每筆 = CBOR map）

**必填（REQUIRED）：`ts`、`host`、`msg`。** 其餘選填（OPTIONAL）。
**權威來源：** `openspec/specs/fcb-stream-types/spec.md:11-25`；契約測試 `crates/fcb/tests/stream_types.rs:34-89`。

> ⚠️ **「CBOR 型別（wire）」與「值域約束（spec-level）」分兩欄讀。** 值域欄是 spec 約束、**codec 不強制**——ciborium 解碼接受任意 uint / 任意 text，crate 層**不檢查** `severity`/`facility` 值域，也不檢查 `format` 是否三選一。機器抽取「wire 解析約束」時只看「CBOR 型別」欄；值域欄屬 validation、不可當 wire-level 硬約束。佐證：crypto/evidence 原始碼全無 `severity`/`facility`/`format` 值域檢查（grep 零命中）；docs/fcb-data-model.md:202-203,213-214,217。

| 欄位 | CBOR 型別（wire） | 值域約束（spec-level，codec 不強制） | 必填 | 語意 | 出處 |
|------|-----------------|--------------------------------------|:----:|------|------|
| `ts` | text | RFC 3339，正規化 UTC、結尾 `Z`、毫秒精度 | ✔ | originator 回報的事件時間 | spec.md:13 |
| `host` | text | — | ✔ | 來源主機（hostname/FQDN/IP），照擷取保留 | spec.md:14 |
| `msg` | text | — | ✔ | 解析後人類可讀訊息本文 | spec.md:15 |
| `raw` | text | — | | 原始未解析整行、逐字保留（**authoritative source / 無損權威真相**） | spec.md:16 |
| `app` | text | — | | 來源 app/程式（RFC 5424 APP-NAME / RFC 3164 TAG） | spec.md:17 |
| `pid` | unsigned integer | — | | 來源 process id | spec.md:18 |
| `severity` | unsigned integer | **0–7**（0=Emergency, 7=Debug）；codec 不驗證值域 | | syslog severity 數字碼 | spec.md:19 |
| `facility` | unsigned integer | **0–23**；codec 不驗證值域 | | syslog facility 數字碼 | spec.md:20 |
| `msgid` | text | — | | 訊息型別識別碼（RFC 5424 MSGID） | spec.md:21 |
| `sd` | CBOR map，依 SD-ID 分組（外層 key=SD-ID，內層 param→string） | — | | RFC 5424 STRUCTURED-DATA | spec.md:22 |
| `format` | text | 三選一 `rfc3164` / `rfc5424` / `other`；codec 不驗證 | | 來源 wire format | spec.md:23 |

額外不變量：
- `severity` / `facility` 只存**數字碼**；人類可讀名稱**不存**、由消費端從數字碼衍生（spec.md:25，與下一句共用此行）。
- producer 須把 `ts` 正規化為 UTC；缺年份/時區時推斷並保留原始行於 `raw`（spec.md:25，與上一句共用此行）。
- `sd` 是 `ciborium::value::Value::Map`（外層 SD-ID → 內層 param map），其 entry 順序依**建構時的插入序**原樣輸出（§2A 的 `Value::Map` 規則）、不做 canonical 排序，因此外層 SD-ID 與內層 param 的順序皆會影響 byte-exactness。佐證：`sd = Value::Map(vec![("ex@32473", Map(vec![("iut","3")]))])`（stream_types.rs:36-39）。

**演進 / 相容規則（4 條）：**

| 規則 | 內容 | 出處（Requirement 標題行 → 本文行） |
|------|------|------|
| Raw line 為權威真相 | `raw` 在場時無損；解析欄位皆 best-effort，消費端須能從 `raw` 重新衍生 | spec.md:81（Requirement 標題）→ 本文 83 |
| 同版本加法式演進 | 同 type version 內僅新增 **OPTIONAL** 欄位；**消費端忽略未知欄位**，選填缺漏不失敗；producer 缺值即省略該 key | spec.md:105（Requirement 標題）→ 本文 107 |
| 破壞性變更升型別版本 | 改既有欄位型別/語意或移除必填欄位 → 發新版本（如 `fcb.syslog.v2`），**不就地改** | spec.md:133（Requirement 標題）→ 本文 135 |
| 未知型別/版本 fallback | 無 handler 的 reader 遇未知 type/版本 → 落 generic table/timeline fallback、不致命 | spec.md:135（同上 Requirement 本文末句）；Scenario「reader without a handler」spec.md:137-141 |

**ECS 對照（crosswalk）—— 僅在 docs，spec.md 本身無此表**（出處：docs/fcb-data-model.md §3.1.2；spec.md 全文無 "ECS"）：

| 本 schema | ECS 欄位 | 出處 |
|-----------|----------|------|
| `ts` | `@timestamp` | docs/fcb-data-model.md:299 |
| `host` | `host.name` / `log.syslog.hostname` | docs/fcb-data-model.md:300 |
| `app` | `log.syslog.appname` / `process.name` | docs/fcb-data-model.md:301 |
| `pid` | `process.pid` / `log.syslog.procid` | docs/fcb-data-model.md:302 |
| `severity` | `log.syslog.severity.code` | docs/fcb-data-model.md:303 |
| `facility` | `log.syslog.facility.code` | docs/fcb-data-model.md:304 |
| `msgid` | `log.syslog.msgid` | docs/fcb-data-model.md:305 |
| `sd` | `log.syslog.structured_data` | docs/fcb-data-model.md:306 |
| `msg` | `message` | docs/fcb-data-model.md:307 |
| `raw` | `event.original` | docs/fcb-data-model.md:308 |

契約測試凍結的三筆 record（schema-freeze，非新 hex）：RFC5424（含 `sd={"ex@32473":{"iut":"3"}}`、`severity=2`、`facility=4`、`format="rfc5424"`）、RFC3164、minimal（僅 `ts`/`host`/`msg`）。出處：stream_types.rs:34-89, 122-152。

---

## 4. 密碼學（crypto）

**權威來源：** `crates/fcb/src/crypto.rs`。

### 4.1 常數與 Argon2id KDF

| 常數/參數 | 實際值 | 出處 |
|-----------|--------|------|
| `KEY_LEN` | `32` bytes | crypto.rs:21 |
| `NONCE_LEN` | `24` bytes | crypto.rs:23 |
| `KCV_DOMAIN` | `b"FCB-key-check-v1"`（16 ASCII bytes） | crypto.rs:25 |
| Argon2 演算法 | `Algorithm::Argon2id` | crypto.rs:34 |
| Argon2 版本 | `Version::V0x13`（= Argon2 v1.3 / `0x13`） | crypto.rs:34 |
| Argon2 `out_len` | `Some(KEY_LEN)` = `Some(32)` | crypto.rs:32 |

`derive_key(passphrase: &str, kdf: &KdfParams) -> Result<[u8; 32]>`（crypto.rs:28）：

```text
key(32 B) = Argon2id(
    password = passphrase.as_bytes()  // &str 的 UTF-8 bytes
    salt     = kdf.salt               // header 原始 bytes
    m_cost   = kdf.m_cost  (KiB)
    t_cost   = kdf.t_cost  (迭代)
    p_cost   = kdf.p_cost  (平行度)
    version  = 0x13
    out_len  = 32
)
```

| 步驟 | 行為 / error | 出處 |
|------|--------------|------|
| algo 檢查 | `kdf.algo != "argon2id"` → `Malformed("unsupported KDF: {algo}")` | crypto.rs:29-31 |
| params 建構 | `Params::new(m_cost, t_cost, p_cost, Some(32))`；失敗 → `Malformed("bad argon2 params: {e}")` | crypto.rs:32-33 |
| 輸出 | `hash_password_into` 填 `[0u8; 32]`；失敗 → `Malformed("argon2 failure: {e}")` | crypto.rs:35-38 |

> **`kdf.algo` 會被驗證；`aead.algo` 永遠不驗證。** 這是對稱的反例——不要假設 `aead.algo` 有檢查。出處：crypto.rs:29-31 vs 全 crypto 不讀 `aead.algo`。

### 4.2 Key-Check Value（KCV）

```text
key_check = SHA256( KCV_DOMAIN || key )
          = SHA256( b"FCB-key-check-v1" || key )    // 32-byte digest
```

- **domain prefix 先 hash、key 後 hash**（`h.update(KCV_DOMAIN)` then `h.update(key)`）。出處：crypto.rs:44-47。
- 輸出 32 bytes（SHA-256），明文存於 `Header.key_check: Vec<u8>`。出處：crypto.rs:43；container.rs:98。

### 4.3 AEAD：XChaCha20-Poly1305（**無 AAD**）

| 函式 | 簽名重點 | nonce | AAD | error | 出處 |
|------|----------|:-----:|:---:|-------|------|
| `cipher_for` | `XChaCha20Poly1305::new(Key::from_slice(key))` | — | — | — | crypto.rs:50-52 |
| `nonce_from` | 拒絕 `len != 24` | 24 B | — | `Malformed("nonce must be 24 bytes, got N")` | crypto.rs:54-62 |
| `seal` | `(&[u8;32], nonce, plaintext) -> Vec<u8>`；`.encrypt(nonce, plaintext)`（2-arg） | 24 B | **none** | `Malformed("AEAD encryption failure")` | crypto.rs:65-70 |
| `open` | `.decrypt(nonce, ciphertext)`（2-arg） | 24 B | **none** | `Corrupt`（任何失敗） | crypto.rs:74-79 |
| `open_payload` | `(key, expected_kcv, nonce, ciphertext)`；先 KCV 比對再 `open` | 24 B | none | WrongPassphrase / Corrupt（見 §4.4） | crypto.rs:83-93 |

- **完全不傳 AAD**——`encrypt`/`decrypt` 只給 `(nonce, payload)`。**明文 header 未被 AEAD 認證。** 出處：crypto.rs:68, 77。
- Poly1305 tag 由 `chacha20poly1305` crate 附在密文尾端（密文長度 = 明文 + 16-byte tag），FCB **無**獨立 tag 欄位。出處：crypto.rs:68（隱含）。

### 4.4 WrongPassphrase vs Corrupt 決策樹

```text
open_payload(key, expected_kcv, nonce, ciphertext):
  if !ct_eq(key_check_value(key), expected_kcv)  -> Err(WrongPassphrase)   // KCV 不符
  else open(key, nonce, ciphertext):
       AEAD decrypt 失敗（tag mismatch）          -> Err(Corrupt)           // KCV 符但竄改
```

| 條件 | error 變體 | 出處 |
|------|-----------|------|
| KCV 不符（密碼錯） | `WrongPassphrase` | crypto.rs:89-91 |
| KCV 符但 AEAD/tag 失敗（竄改） | `Corrupt` | crypto.rs:74-78 |
| nonce 長度 ≠ 24 | `Malformed(...)` | crypto.rs:54-62 |
| zstd frame 解壓失敗 | `Corrupt` | compress.rs:35-37 |

`ct_eq(a, b)`（constant-time 比對，crypto.rs:96-105）：長度不符立即回 `false`（長度非秘密）；否則 `diff |= x ^ y` 全程累加、無 early exit、回 `diff == 0`。**手刻累加器，非 `subtle` crate**；對等長輸入仍為 constant-time。

---

## 5. 壓縮（compress）：compress-then-encrypt

**權威來源：** `crates/fcb/src/compress.rs`。

| 常數/參數 | 實際值 | 出處 |
|-----------|--------|------|
| `ZSTD_MAGIC` | `[0x28, 0xB5, 0x2F, 0xFD]`（標準 zstd frame magic） | compress.rs:21 |
| 壓縮等級 | `CompressionLevel::Fastest`（ruzstd 0.8 編碼僅實作 Fastest 與 Uncompressed） | compress.rs:29-31 |

```text
pack_payload(key, nonce, plaintext):       // compress 先、encrypt 後
    compressed = compress(plaintext)        // zstd Fastest -> 標準 frame
    seal(key, nonce, compressed)            // AEAD 包住 zstd frame

unpack_payload(key, kcv, nonce, ciphertext):  // 反向
    compressed = open_payload(key, kcv, nonce, ciphertext)
    decompress(compressed)
```

| 函式 | 行為 | 出處 |
|------|------|------|
| `compress` | `compress_to_vec(data, Fastest)`（ruzstd 0.8 encoder）；恆成功（`Ok`） | compress.rs:29-31 |
| `decompress` | `StreamingDecoder` + `read_to_end`；**任何錯誤 → `Corrupt`** | compress.rs:34-39 |
| `pack_payload` | compress **先**，再 `crypto::seal` | compress.rs:42-45 |
| `unpack_payload` | `crypto::open_payload` **先**，再 `decompress` | compress.rs:48-56 |

- 後端純 Rust `ruzstd` 0.8（無 C/FFI），native 與 `wasm32-unknown-unknown` 同一份 crate；產出標準 zstd frame，與 C-zstd reader 互通。出處：compress.rs:6-10。
- **順序證明測試** `order_is_compress_then_encrypt`：解密後得到的正是 zstd frame——內層前 4 bytes == `ZSTD_MAGIC`，外層前 4 bytes != `ZSTD_MAGIC`。出處：compress.rs:89-105（`fn` 起點；`#[test]` 屬性在 88）。

---

## 6. Task、Submission、Binding

### 6.1 Bundle 打包流程（`bundle.rs`）

| 常數 | 實際值 | 單位 | 出處 |
|------|--------|------|------|
| `DEFAULT_M_COST` | `19456` | Argon2 記憶體 KiB | bundle.rs:16 |
| `DEFAULT_T_COST` | `2` | 迭代 | bundle.rs:17 |
| `DEFAULT_P_COST` | `1` | 平行度 | bundle.rs:18 |
| `SALT_LEN` | `16` | bytes（隨機 salt） | bundle.rs:19 |

`pack_bytes(&BundleParams, payload: &[u8], passphrase: &str) -> Result<Vec<u8>>`（bundle.rs:57-84）執行順序：

| # | 步驟 | 細節 | 出處 |
|---|------|------|------|
| 1 | salt | `random_bytes(16)`（`getrandom`） | bundle.rs:60 |
| 2 | nonce | `random_bytes(24)` | bundle.rs:65 |
| 3 | `KdfParams` | `algo="argon2id"`, salt, m/t/p（來自 params） | bundle.rs:58-64 |
| 4 | key | `derive_key(passphrase, &kdf)` → 32 B | bundle.rs:66 |
| 5 | key_check | `key_check_value(&key)`（**加密前**算） | bundle.rs:67 |
| 6 | ciphertext | `pack_payload(&key, &nonce, payload)`（compress-then-encrypt） | bundle.rs:68 |
| 7 | header | `Header{ ver:1, min_reader:1, case_id, bundle_hash, kdf, aead{algo:"xchacha20poly1305", nonce}, key_check, meta }` | bundle.rs:70-82 |
| 8 | frame | `write_container(params.kind, &header, &ciphertext)` | bundle.rs:83 |

- RNG 失敗 → `Malformed("rng failure: {e}")`（bundle.rs:21-24）。
- `BundleParams` 欄位：`kind, case_id, bundle_hash, meta, m_cost, t_cost, p_cost`；`BundleParams::new(...)` seed 預設 cost `19456/2/1`（bundle.rs:29-53）。
- `open_bytes(&[u8], &str) -> Result<(BundleKind, Header, Vec<u8>)>`：`read_container` → `derive_key` → `unpack_payload`（bundle.rs:88-98）。

### 6.2 `TaskSpec`（map(3)）— `task.rs:39-45`

| 欄位 | 型別 | CBOR key | serde default | 必填？ | 出處 |
|------|------|----------|---------------|:------:|------|
| `report_mode` | `ReportMode`（enum） | `"report_mode"` | 無 | ✔ | task.rs:41 |
| `instructions` | `String` | `"instructions"` | 無 | ✔ | task.rs:42 |
| `steps` | `Vec<TaskStep>` | `"steps"` | `#[serde(default)]` → 空 `Vec` | optional（預設 `[]`） | task.rs:43-44 |

Derives：`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`（task.rs:39）。**只有 `steps` 有 default。**

### 6.3 `TaskStep`（map(3)）+ `ReportMode`

`TaskStep`（task.rs:31-36）：

| 欄位 | 型別 | CBOR key | 必填？ | 出處 |
|------|------|----------|:------:|------|
| `id` | `String` | `"id"` | ✔ | task.rs:33 |
| `prompt` | `String` | `"prompt"` | ✔ | task.rs:34 |
| `answer_type` | `String` | `"answer_type"` | ✔ | task.rs:35 |

**刻意無答案/expected-value 欄位**（task.rs:30, 32-36）。三欄全必填。

`ReportMode` enum（task.rs:21-28）：`#[serde(rename_all = "lowercase")]`，變體 `Steps` / `Freeform` → 序列化為 `"steps"` / `"freeform"`。Derives `Eq`。

`TaskMeta`（私有 wrapper，task.rs:47-51）：`{ task: Option<TaskSpec> }`，`#[serde(default, skip_serializing_if = "Option::is_none")]` → `None` 時 `task` key 整個省略。

| 函式 | 行為 | 出處 |
|------|------|------|
| `task_to_meta(&TaskSpec) -> Result<Value>` | 包成 `TaskMeta{task: Some(...)}` → `{ "task": TaskSpec }` | task.rs:54-56 |
| `task_from_meta(&Value) -> Result<Option<TaskSpec>>` | 解 `TaskMeta`，回 `task`；容忍其他 key（如 `streams`）；缺 `task` → `Ok(None)` | task.rs:60-63 |
| `contains_answer_fields(&Value) -> bool` | 遞迴（Map/Array）；防呆 assert（見 §6.6） | task.rs:67-79 |

> **`task` 寫入分歧（互通陷阱）：** 測試的 `CaseMeta{streams, task}`（vectors.rs:31-35）`task` 是非 `Option` 欄位 → **永遠寫入**；library 的 `TaskMeta`（task.rs:47-56）`task=None` → **整個省略**。故 golden CASE meta 是 `a2`（含 `task`）；用 `CaseMeta` 當模板會永遠 emit `task`。讀側兩者皆容忍。出處：vectors.rs:31-35,85 vs task.rs:47-56。

### 6.4 `Submission`（map(7)）+ `Student`

`Submission`（submission.rs:25-40）：

| 欄位 | 型別 | CBOR key | 對 container 透明？ | 出處 |
|------|------|----------|:------------------:|------|
| `case_id` | `String` | `"case_id"` | 否 | submission.rs:28 |
| `bundle_hash` | `String` | `"bundle_hash"` | 否 | submission.rs:30 |
| `student` | `Student` | `"student"` | 否（typed） | submission.rs:31 |
| `notes` | `Vec<Value>` | `"notes"` | **是**（每元素 opaque CBOR Value） | submission.rs:33 |
| `report` | `Value` | `"report"` | **是**（單一 opaque Value：steps 模式為陣列，freeform 為文字） | submission.rs:35 |
| `activity` | `Vec<Value>` | `"activity"` | **是**（每元素 opaque） | submission.rs:37 |
| `exported_at` | `String` | `"exported_at"` | 否 | submission.rs:39 |

Derives：`Debug, Clone, PartialEq, Serialize, Deserialize`（**無 `Eq`**，因含 `Value`）（submission.rs:25）。**無 `default`** → 7 欄全必填。`notes`/`report`/`activity` 的 schema 由 workbench 擁有、對 container 不透明（submission.rs:23-24）。

`Student`（submission.rs:17-21）→ **map(2)/`a2`**：`{ id: String, name: String }`，CBOR key `"id"`/`"name"`。Derives `Eq`。

| 函式 | 行為 | 出處 |
|------|------|------|
| `pack_submission(&Submission, &str) -> Result<Vec<u8>>` | `cbor::encode(work)` → `BundleParams::new(Work, case_id, bundle_hash, Value::Map(vec![]))` → `pack_bytes`。**meta = 空 map `{}`**；預設 Argon2 cost | submission.rs:43-52 |
| `open_submission(&[u8], &str) -> Result<Submission>` | `open_bytes`；**KIND-gated**：`kind != Work` → `Malformed("not a .casework (KIND != work)")`；否則 `cbor::decode` | submission.rs:55-61 |

> `case_id` / `bundle_hash` **雙重存放**：既在明文 header（透過 `pack_submission` params）又在加密 `Submission` payload。出處：submission.rs:28,30,47-48。

### 6.5 Binding（`binding.rs`）

| 常數/格式 | 實際值 | 出處 |
|-----------|--------|------|
| bundle hash 前綴 | `"sha256:"`（7 chars） | binding.rs:17 |
| hex 格式 | `{b:02x}`（**小寫**、零填補、每 byte 2 位） | binding.rs:19 |
| `work_key` 格式 | `"fcb:work:{case_id}"` | binding.rs:55 |

| 函式 | 行為 | 出處 |
|------|------|------|
| `compute_bundle_hash(&[u8]) -> String` | `"sha256:" + lower_hex(SHA256(bytes))`，總長 **71 chars**（7 + 64）；deterministic | binding.rs:14-22 |
| `verify_binding(work_case_id, work_bundle_hash, case_id, case_bundle_hash) -> BindingCheck` | 見下決策樹 | binding.rs:37-50 |
| `work_key(case_id) -> String` | `"fcb:work:{case_id}"`，純 ASCII（無 hash/escape） | binding.rs:54-56 |

```text
verify_binding（順序）:
  if work_case_id != case_id          -> CaseMismatch              // case 身分優先
  else if work_bundle_hash != case_bundle_hash -> EvidenceVersionMismatch
  else                                -> Match
```

`BindingCheck` enum（binding.rs:25-33）：`Match` / `CaseMismatch` / `EvidenceVersionMismatch`，derives `Debug, Clone, Copy, PartialEq, Eq`。
- `Match` = 同 case 且同 evidence 版本。
- `CaseMismatch` = 完全不同的 challenge。
- `EvidenceVersionMismatch` = 同 `case_id`、不同 evidence 版本（重新 issue 的 bundle）。
- **case_id 先於 bundle_hash 檢查**——不同 case 永遠不會回 version mismatch（binding.rs:43-46）。

> codec **不規定** `compute_bundle_hash` 的 `bytes` 是什麼——golden vector 用假值 `"sha256:deadbeef"`。涵蓋範圍是生產端責任。**建議慣例（未凍結）：** `bundle_hash = compute_bundle_hash(.case 明文 payload bytes)`（壓縮/加密前），使同一份證物無論 salt/nonce 為何皆得相同 hash。出處：binding.rs:12-22（無驗證）；vectors.rs:63。

### 6.6 答案安全不變量（answer-safety）

| 常數 | 實際值 | 出處 |
|------|--------|------|
| `FORBIDDEN_ANSWER_KEYS` | `["answer", "answer_key", "rubric", "solution", "expected"]`（順序固定，5 個） | task.rs:17-18 |

- **零答案 / typed-model 強制**：`.case` 由學生 client 完整解密，內容全可見。`TaskSpec` / `TaskStep` **沒有任何欄位能承載答案/rubric/solution**，故透過 typed model 解碼會自然丟掉任何洩漏的答案欄位。正確答案只存在於學生 build **之外**。出處：task.rs:1-8, 30-36；spec.md:59（fcb-task-spec 的 SHALL-NOT 本文；完整 Requirement + Scenario 為 spec.md:57-72）。
- `contains_answer_fields(v)`（防呆 assert，task.rs:67-79）：
  - `Value::Map`：若任一 entry 的 `Value::Text` key 在 `FORBIDDEN_ANSWER_KEYS` 中 → `true`；或任一 value 遞迴含答案欄位 → `true`。
  - `Value::Array`：任一元素遞迴含 → `true`。
  - 其他 variant（Text/Integer/Bytes/Float/Bool/Null/Tag）→ `false`。
  - **只比對 `Value::Text` key**；forbidden 字當非 text key 不會 match。
- 測試 `answer_fields_are_stripped_on_decode`：含 `answer` 的 dirty map 被偵測（`true`）→ 解成 `TaskSpec` 丟掉 → 重編 `false`。出處：task.rs:103-130。

---

## 7. Error 目錄（`error.rs`）

`#[derive(Debug, Error, PartialEq, Eq)] pub enum FcbError`（error.rs:8-9）。`pub type Result<T> = Result<T, FcbError>`（error.rs:32）。

| 變體 | `#[error]` 文字 | 語意 | 出處 |
|------|----------------|------|------|
| `BadMagic` | `"not an FCB container (bad magic)"` | 前 4 bytes 非 FCB magic | error.rs:11-12 |
| `UnsupportedVersion { min_reader: u16, supported: u16 }` | `"unsupported FCB version: bundle requires reader >= {min_reader}, this reader supports {supported}"` | bundle 的 `min_reader` 比本 reader 新 | error.rs:14-16 |
| `Malformed(String)` | `"malformed FCB container: {0}"` | 結構性無效 | error.rs:18-20 |
| `WrongPassphrase` | `"wrong passphrase"` | KCV 不符（密碼錯） | error.rs:22-24 |
| `Corrupt` | `"corrupt or tampered bundle"` | KCV 符但 AEAD 失敗（竄改），或壞 zstd frame | error.rs:26-28 |

`WrongPassphrase` 與 `Corrupt` 刻意分開——皆源自 AEAD 驗證，但對學生/operator 意義不同（error.rs:5-7）。

**觸發點對照（exhaustive）：**

| 條件 | 變體 | 出處 |
|------|------|------|
| 前 4 bytes 非 magic | `BadMagic` | container.rs:153, 182 |
| `min_reader > READER_VERSION` | `UnsupportedVersion` | container.rs:169-174, 199-204 |
| truncated u16/u32 | `Malformed("truncated u16/u32")` | container.rs:115, 123 |
| missing KIND | `Malformed("missing KIND")` | container.rs:159 |
| 未知 KIND byte | `Malformed("unknown KIND byte {other}")` | container.rs:52 |
| header out of bounds | `Malformed("header length out of bounds")` | container.rs:165, 194 |
| bad header CBOR | `Malformed("bad header CBOR: {e}")` | container.rs:168, 198 |
| encode header 失敗 | `Malformed("encode header: {e}")` | container.rs:132 |
| 未知 KDF algo（≠ argon2id） | `Malformed("unsupported KDF: {algo}")` | crypto.rs:29-31 |
| nonce 長度 ≠ 24 | `Malformed("nonce must be 24 bytes, got N")` | crypto.rs:54-62 |
| AEAD encrypt 失敗 | `Malformed("AEAD encryption failure")` | crypto.rs:65-70 |
| RNG 失敗 | `Malformed("rng failure: {e}")` | bundle.rs:21-24 |
| manifest id 無對應 payload | `Malformed("payload missing stream {id}")` | evidence.rs:84 |
| `open_submission` 收到非 Work KIND | `Malformed("not a .casework (KIND != work)")` | submission.rs:57-59 |
| KCV 不符 | `WrongPassphrase` | crypto.rs:89-91 |
| KCV 符但 AEAD decrypt 失敗 | `Corrupt` | crypto.rs:74-78 |
| zstd 解壓失敗 | `Corrupt` | compress.rs:34-39 |
| `cbor::decode` 失敗（payload 層） | `Corrupt` | cbor.rs:39 |

> **CBOR error 映射陷阱：** `cbor.rs` 的 `encode`/`to_value`/`from_value` 失敗映射成 `Malformed`，但 `decode`（payload 層）失敗映射成 **`Corrupt`**（非 `Malformed`）。出處：`Malformed` 的 `map_err` 在 cbor.rs:17、18、25、26、33（函式級範圍 cbor.rs:14-35）；`Corrupt` 在 cbor.rs:39（`decode` 函式 cbor.rs:38-39）。

---

## 8. Golden vector byte map（byte-exact baseline）

**權威來源：** `crates/fcb/tests/vectors.rs`。固定 salt/nonce 產生；任何 FCB 重實作必須能解、重建必須產生**完全相同**位元組。

### 8.1 凍結常數

| 名稱 | 值 | 出處 |
|------|----|------|
| `PASS` | `"lab-pass"` | vectors.rs:22 |
| `SALT`（`[u8;16]`） | `[0x53,0x41,0x4c,0x54,1,2,3,4,5,6,7,8,9,10,11,12]`（前 4 = ASCII `"SALT"`） | vectors.rs:23 |
| `NONCE`（`[u8;24]`） | `[0,1,2,…,23]` | vectors.rs:24-26 |
| `FROZEN_CASE_HEX` | 578 bytes，起頭 `89464342010100dc010000a8…` | vectors.rs:28 |
| `FROZEN_WORK_HEX` | 423 bytes，起頭 `8946434202010025010000a8…` | vectors.rs:29 |

### 8.2 前綴拆解

**`FROZEN_CASE_HEX`（578 bytes 總計 = 11 前綴 + 476 header + 91 payload）：**

| 欄位 | hex | 解碼值 | 出處 |
|------|-----|--------|------|
| magic | `89 46 43 42` | `\x89FCB` | vectors.rs:28 |
| KIND | `01` | `1` = `.case` | vectors.rs:28 |
| container_version | `01 00` | `1`（u16 LE） | vectors.rs:28 |
| hdr_len | `dc 01 00 00` | `0x000001dc` = **476**（u32 LE） | vectors.rs:28 |
| header CBOR 起頭 | `a8` | map(8) = Header 8 欄位 | vectors.rs:28 |
| payload | （byte 487 後） | 91 bytes 的 `AEAD(zstd(plaintext))` | vectors.rs:28 |

**`FROZEN_WORK_HEX`（423 bytes 總計 = 11 + 293 header + 119 payload）：**

| 欄位 | hex | 解碼值 | 出處 |
|------|-----|--------|------|
| magic | `89 46 43 42` | `\x89FCB` | vectors.rs:29 |
| KIND | `02` | `2` = `.casework` | vectors.rs:29 |
| container_version | `01 00` | `1` | vectors.rs:29 |
| hdr_len | `25 01 00 00` | `0x00000125` = **293** | vectors.rs:29 |
| header CBOR 起頭 | `a8` | map(8) | vectors.rs:29 |
| payload | （byte 304 後） | 119 bytes | vectors.rs:29 |

兩 header 皆起頭 `a8`，因 Header 恆為 8 欄位 CBOR map。

### 8.3 header 內 CBOR marker 對照（從 `FROZEN_CASE_HEX` 解出）

> 出處欄行號標示：marker/值由 `build()` 內的 `Header` 建構決定（vectors.rs:48-73），凍結後的位元組則整段存於 `FROZEN_CASE_HEX` 字串（vectors.rs:28）。下表「出處」優先指向 `build()` 內對應欄位的賦值行（`vectors.rs:50-69`）；`:28` 僅作為「該位元組落在 frozen hex 字串裡」的指引。

| Header 欄位 | CBOR marker / 值 | 解碼 | 出處（build 賦值行） |
|-------------|------------------|------|------|
| `header_schema_ver` | `01` | uint 1 | vectors.rs:60 |
| `min_reader` | `01` | uint 1 | vectors.rs:61 |
| `case_id` | text | `"acme-ir-2026-03"` | vectors.rs:62 |
| `bundle_hash` | text | `"sha256:deadbeef"`（**假佔位值**） | vectors.rs:63 |
| `kdf`（map(5)） | `a5` | — | vectors.rs:28 |
| `kdf.algo` | text | `"argon2id"`（hex `686172676f6e326964`） | vectors.rs:50 |
| `kdf.salt` | `90` + 16 uint | array(16) = SALT bytes | vectors.rs:51 |
| `kdf.m_cost` | `18 20` | uint **32** | vectors.rs:52 |
| `kdf.t_cost` | `01` | uint **1** | vectors.rs:53 |
| `kdf.p_cost` | `01` | uint **1** | vectors.rs:54 |
| `aead`（map(2)） | `a2` | — | vectors.rs:28 |
| `aead.algo` | text | `"xchacha20poly1305"`（hex `786368616368613230706f6c7931333035`） | vectors.rs:66 |
| `aead.nonce` | `98 18` + 24 uint | array(24) = NONCE bytes | vectors.rs:67 |
| `key_check` | `98 20` + 32 uint | array(32) = KCV bytes | vectors.rs:69 |
| `meta`（case） | `a2` | map(2)：`{streams, task}` | vectors.rs:85 |
| `meta`（work） | `a0` | **空 map** `{}` | vectors.rs:103 |

> ⚠️ golden vector 的 Argon2 cost 是**快測值** `m_cost=32`（`18 20`）/ `t_cost=1` / `p_cost=1`（vectors.rs:49-55），**非** library 生產預設 `19456/2/1`（§6.1）。frozen hex 的 `18 20` 證明之。
> ⚠️ `salt`/`nonce`/`key_check` 皆 **array of uint**（marker `90`/`98 18`/`98 20`），非 byte string——直接從 frozen hex 驗證（§2A）。

### 8.4 golden CASE 結構（`build_case`，vectors.rs:75-94）

- manifest（在 meta）：`{id:"s0", type:"fcb.syslog.v1", records:2}`、`{id:"s1", type:"acme.edr.v1", records:1}`（vectors.rs:77-78）。
- task（在 meta）：`TaskSpec{ report_mode:Steps, instructions:"Investigate the host.", steps:[TaskStep{id:"q1", prompt:"source IP?", answer_type:"ip"}] }`（vectors.rs:80-83）。
- payload（加密）：`{streams:[ StreamData{s0:["evt1","evt2"]}, StreamData{s1:[7]} ]}`（vectors.rs:86-92）。
- 解碼斷言：`manifest.len()==2`；`task.report_mode==Steps`；`steps[0].id=="q1"`；`streams[0]` = `fcb.syslog.v1` `is_builtin=true`；`streams[1]` = `acme.edr.v1` `is_builtin=false`（vectors.rs:116-139）。

### 8.5 golden WORK 結構（`build_work`，vectors.rs:96-104）

- payload：`WorkPayload{ case_id:"acme-ir-2026-03", bundle_hash:"sha256:deadbeef", report:Text("freeform report") }`；meta = 空 map（vectors.rs:96-104）。
- binding 斷言 `Match`（`verify_binding(work.case_id, work.bundle_hash, header.case_id, header.bundle_hash)`，vectors.rs:149-152）。

> ⚠️ **`FROZEN_WORK_HEX` 凍結的是測試用 3 欄 `WorkPayload`（`case_id`/`bundle_hash`/`report`），不是 library 的 7 欄 `Submission`（§6.4）。** `WorkPayload` 是 test-local struct（vectors.rs:40-45），與真正寫入 `.casework` 的 `Submission`（submission.rs:25-40）欄位集**不同**。目前**沒有任何 golden vector 釘住真實 `Submission` 的 on-disk 位元組**——`Submission` 只有 `submission_random_round_trip`（random salt/nonce，vectors.rs:175-189）覆蓋，只證 round-trip、**不證 byte-stability**。非 Rust 重實作者若要驗 `Submission` byte-exactness，須自補向量。出處：vectors.rs:40-45,96-104（`WorkPayload`）vs submission.rs:25-40（`Submission`）。

### 8.6 佔位值（**非真實生產資料**）

| 項目 | 凍結值 | 狀態 | 出處 |
|------|--------|------|------|
| `bundle_hash` | `"sha256:deadbeef"` | 假佔位（非真 SHA-256） | vectors.rs:63,99 |
| syslog records（s0） | `Text("evt1")`, `Text("evt2")` | 佔位，**非**真 `fcb.syslog.v1` schema | vectors.rs:88 |
| edr record（s1） | `Integer(7)` | 佔位 | vectors.rs:89 |
| work report | `Text("freeform report")` | 佔位 | vectors.rs:100 |

### 8.7 golden vector 測試

| 測試 | 斷言 | 出處 |
|------|------|------|
| `case_vector_is_byte_stable` | `hex::encode(build_case()) == FROZEN_CASE_HEX` | vectors.rs:106-109 |
| `work_vector_is_byte_stable` | `hex::encode(build_work()) == FROZEN_WORK_HEX` | vectors.rs:111-114 |
| `frozen_case_vector_decodes_to_expected_structure` | 見 §8.4 | vectors.rs:116-139 |
| `frozen_work_vector_decodes_to_expected_structure` | kind=Work、binding=Match | vectors.rs:141-153 |
| `wrong_passphrase_on_vector_is_rejected` | `open_bytes(CASE,"wrong")` → `WrongPassphrase` | vectors.rs:155-162 |
| `tampered_vector_is_corrupt` | 翻 CASE 末 byte → `Corrupt` | vectors.rs:164-173 |
| `submission_random_round_trip` | `pack_submission`/`open_submission` 往返 | vectors.rs:175-189 |
| `case_random_round_trip` | `pack_bytes`/`open_bytes`（payload `b"evidence"`） | vectors.rs:191-201 |

---

## 9. 已知缺口（Known Gaps）與 Non-Goals

**誠實標註——不誇大已實作程度。**

| 缺口 | 現況 | 出處 |
|------|------|------|
| **無 `pack_case` / `CasePayload` 寫入 helper** | crate 只提供「讀側」（`StreamData` 型別 + `decode_streams`），**無**組 `{streams:[...]}` 信封的公開寫入 helper。生產端須手組 CBOR 或自呼 `bundle::pack_bytes`。`CasePayload` 只是 test-local struct（兩個 test 各自定義） | evidence.rs（全檔無 `pack_case`/`CasePayload`）；vectors.rs:36-39；stream_types.rs:18-21 |
| **`bundle_hash` 正規定義未凍結** | `compute_bundle_hash` 接受任意 bytes，**不綁定**特定 payload；codec 不驗證涵蓋範圍；golden 用假值。建議慣例見 §6.5，但**未在程式碼強制** | binding.rs:14-22；vectors.rs:63 |
| **`fcb.netflow.v1` / `fcb.json.v1` 無記錄 schema** | 列在 `BUILTIN_STREAM_TYPES` 但 spec/crate 皆未定義其 record schema | evidence.rs:18；docs/fcb-data-model.md:210-214 |
| **WASM 綁定最小** | `wasm.rs` 只導出 `fcb_version() -> String`（回 `CARGO_PKG_VERSION`），**無** `openBundle`/`packSubmission` 綁定 | wasm.rs:6-13 |
| **plugin registry 未實作** | `DecodedStream.is_builtin == false` 註解提及「a registered plugin」，但本 crate **無** registry 程式碼——是消費端（前端）概念 | evidence.rs:50（**未證實**有 registry） |
| **多餘 payload stream 是否拒絕未測** | 原始碼顯示靜默忽略（manifest 驅動 iteration），但無測試斷言任一方向 | evidence.rs:78-92（**未證實**） |

**Non-Goals（本 codec 不負責）：** `bundle_hash` 涵蓋範圍驗證、IndexedDB 實體 store（`work_key` 只給 key-derivation 邏輯，binding.rs:6-8）、消費端 plugin/query（屬 `plugin-protocol`/`query-model`，非 fcb codec）。

---

## 10. 相依套件版本（`crates/fcb/Cargo.toml`）

Package：`fcb` `0.1.0`，edition `2021`，license `MIT OR Apache-2.0`，`crate-type = ["cdylib", "rlib"]`（Cargo.toml:1-9）。

| 套件 | 版本（caret 範圍） | features | 用途 | 出處 |
|------|---------------------|----------|------|------|
| `serde` | `1` | `["derive"]` | derive 序列化 | Cargo.toml:12 |
| `ciborium` | `0.2` | — | CBOR 編解碼（header/meta/payload） | Cargo.toml:13 |
| `argon2` | `0.5` | — | Argon2id KDF（`Argon2id` / `V0x13`） | Cargo.toml:14 |
| `chacha20poly1305` | `0.10` | — | XChaCha20-Poly1305 AEAD | Cargo.toml:15 |
| `ruzstd` | `0.8` | — | 純 Rust zstd（編碼僅 `Fastest`） | Cargo.toml:16 |
| `sha2` | `0.10` | — | SHA-256（KCV / bundle_hash） | Cargo.toml:17 |
| `thiserror` | `2` | — | error derive | Cargo.toml:18 |
| `getrandom` | `0.2` | （wasm 加 `["js"]`） | salt / nonce 隨機來源 | Cargo.toml:19, 23 |
| `wasm-bindgen`（僅 wasm32） | `0.2` | — | WASM 綁定 | Cargo.toml:22 |
| `hex`（dev） | `0.4` | — | golden vector hex 編解碼 | Cargo.toml:26 |

> 版本字串為 Cargo semver caret 範圍（`"0.2"` = `^0.2`），非鎖定版；實際解析版本見 `Cargo.lock`（**未證實**具體 patch 版）。

---

## 11. 端到端打包流程（end-to-end，可照做）

組一個 `.case`（資料結構細節見對應小節）：

1. **準備證物**：每條 stream → `StreamData { id, records }`；`records` 每筆形狀依 `type`（如 `fcb.syslog.v1`，§3.5）。
2. **組 manifest**：每條 → `StreamManifest { id, type, records = len(records) }`（§3.1，注意 CBOR key 是 `type`）。
3. **組 task spec**：`TaskSpec`（**零答案**，§6.6）。
4. **meta**（明文）= CBOR `{ "streams": [manifest…], "task": TaskSpec }`（§3.0）。
5. **payload_plain** = CBOR `{ "streams": [StreamData…] }`（§3.0）。
6. **bundle_hash**：建議 = `compute_bundle_hash(payload_plain)`（= `"sha256:" + lower_hex(SHA256(payload_plain))`，§6.5）。⚠️ 正規涵蓋範圍未凍結（§9）。
7. **隨機**產 `salt`(16 B) 與 `nonce`(24 B)（§6.1）。
8. **key** = Argon2id(passphrase UTF-8, salt, m/t/p, version 0x13, out 32 B)（§4.1）。
9. **key_check** = SHA256(`"FCB-key-check-v1"` ‖ key)（32 B，§4.2）。
10. **compressed** = zstd `Fastest`(payload_plain)（§5）。
11. **ciphertext** = XChaCha20-Poly1305 seal(key, nonce, compressed)（**無 AAD**，§4.3）。
12. **header**（struct）= `{ header_schema_ver:1, min_reader:1, case_id, bundle_hash, kdf:{algo:"argon2id", salt, m_cost, t_cost, p_cost}, aead:{algo:"xchacha20poly1305", nonce}, key_check, meta }`（§2）。
13. **hdr** = CBOR(header)（守 §2A 的 `Vec<u8>`→array、欄位順序、key 名規則）。
14. **輸出** = `magic(89 46 43 42) ‖ KIND(01) ‖ container_version(01 00) ‖ len(hdr) as u32 LE ‖ hdr ‖ ciphertext`（§1）。

`.casework` 相同，差別：`KIND = 02`、`meta = {}`（空 map）、`payload_plain = CBOR(Submission)`（§6.4）。

順序要點：`salt`/`nonce` 在推 key 前產生；`key_check` 在加密前算；`header` 在 `salt`/`nonce`/`key_check`/`bundle_hash`/`meta` 備齊後才能序列化求 `hdr_len`（`header` **不含** `ciphertext`）。

> **給 case builder 作者的捷徑：** Rust 寫的 case builder 直接相依 `fcb` crate（已是 `cdylib`+`rlib`），呼叫 `bundle::pack_bytes` + `evidence`/`task` 的 `*_to_meta` helper，零 CBOR 漂移風險。只有用**非 Rust** 重寫 codec 時才需逐位元對齊本檔並以 golden vectors（§8）驗證。注意：`{streams:[...]}` payload 信封與 `bundle_hash` 正規定義目前無公開 helper（§9），這兩塊要自補或回頭在 crate 加 helper。

---

## 不變量總表（cross-reference）

1. 所有整數 LE；字串 UTF-8。（§1；container.rs:4）
2. 固定前綴 11 bytes（magic 4 + KIND 1 + version 2 + hdr_len 4）。（§1）
3. `Vec<u8>`（salt/nonce/key_check）→ CBOR array of uint，**非** byte string。（§2A；vectors.rs:28）
4. struct → text-key map、順序 = 宣告序；無 `rename_all` 時 key = 欄位名。（§2A）
5. 管線順序：pack = compress→encrypt；open = decrypt→decompress。（§5；compress.rs:42-56）
6. AEAD **無 AAD** → 明文 header（含 case_id/bundle_hash/meta）**未被 AEAD 認證**。（§4.3；crypto.rs:68,77）
7. `kdf.algo` **驗證**（≠argon2id → Malformed）；`aead.algo` **永不驗證**。（§4.1）
8. KCV = `SHA256(domain ‖ key)`——domain 先、key 後。（§4.2；crypto.rs:44-47）
9. WrongPassphrase ⟺ KCV 不符；Corrupt ⟺ KCV 符但 AEAD/zstd 失敗；Malformed ⟺ nonce 長度錯。（§4.4）
10. salt = 16 B、nonce = 24 B，每次 `pack_bytes` 經 `getrandom` 新生。（§6.1）
11. Argon2 預設 `19456/2/1`（m KiB / t 迭代 / p lanes）存於 per-bundle 明文 header，非 hard-wire 於 `derive_key`。（§6.1）
12. `BUILTIN_STREAM_TYPES` 是**非封閉**集合；未知 type 不致命（`is_builtin=false`）。（§3.4）
13. `decode_streams` 保 manifest 順序、長度 == manifest；唯一結構性 error = manifest id 無對應 payload。（§3.4）
14. 答案安全：`TaskSpec`/`TaskStep` 無答案欄位；typed-model 解碼丟棄洩漏答案；`FORBIDDEN_ANSWER_KEYS` 固定 5 個。（§6.6）
15. `open_submission` KIND-gated（只收 Work）；submission meta 恆為空 map。（§6.4）
16. binding 優先序：case 身分 > evidence 版本（case_id 先檢查）。（§6.5）
17. `bundle_hash` = `"sha256:" + lower_hex(SHA256(bytes))`，總長 71 chars；涵蓋範圍由生產端負責、codec 不驗證。（§6.5）
18. golden vector byte 穩定：固定 salt/nonce 重建須產生相同 hex，否則「format drifted」測試失敗。（§8.7）
