## Context

`crates/fcb/src/container.rs` 解析 container 時，`peek_header` 與 `read_container` 都用 `bytes.get(pos..pos + hdr_len)` 取標頭切片，其中 `hdr_len = read_u32(...) as usize` 來自不受信任的 wire bytes。位移 `pos` 在讀完 magic(4)+KIND(1)+container_version(2)+hdr_len(4) 後為 11。

- **64-bit target**：`11 + u32::MAX ≈ 4.3e9 ≪ usize::MAX`，不會溢位；`.get()` 對超界範圍回 `None`，落到既有的 `Malformed` 分支。所以 64-bit 本來就安全。
- **32-bit target（wasm32，`usize = u32`）**：`11 + u32::MAX` 溢位 `usize`。debug build（含 overflow-checks）會 **panic**；release build 則 wrap 成一個「start>end 或界內但錯誤」的範圍。fcb-wasm 編到 wasm32 並對外提供 `openCase`，故此路徑可達。

## Goals / Non-Goals

**Goals:**

- container 解析對 wire-derived 長度欄位採 overflow-safe 計算：溢位或超界 → `FcbError::Malformed`，且在任何 target（含 wasm32 debug）都**不 panic**。
- 對合法 container 的解析結果與輸出位元組**完全不變**。
- 提供一個能在 64-bit native 上就**真正紅綠**的回歸測試（不依賴實際在 wasm32 跑出溢位）。

**Non-Goals:**

- 不改 wire format、不動 `READER_VERSION`、不動 `container_version`；非 BREAKING。
- 不重生任何 golden vector。
- 不處理 `read_u16`/`read_u32`（其 `*pos + 2/4` 在呼叫時 `pos` 恆為小值 5／7，永不溢位）——僅在 Decisions 記為「已分析、安全」。

## Decisions

**D1 — 抽出純函式 helper 統一界限計算。**
新增一個私有純函式（簽名意義固定，名稱以實作為準），語意為：

```
fn header_slice(bytes: &[u8], pos: usize, hdr_len: usize) -> Result<&[u8]>
    = pos.checked_add(hdr_len)
         .and_then(|end| bytes.get(pos..end))
         .ok_or_else(|| FcbError::Malformed("header length out of bounds".into()))
```

`peek_header`、`read_container` 都改呼叫它取標頭切片。`read_container` 取得切片後再 `pos += hdr_len`——因為切片成立保證 `pos + hdr_len ≤ bytes.len() ≤ usize::MAX`，故該加法與其後的 `bytes[pos..]`（L222）恆在界內、無需另改。
- 替代方案：在兩處各自 inline `checked_add`。否決——重複邏輯、易漂移，且 helper 讓測試能直接餵任意 `pos`（見 D2）。

**D2 — 用「可在 64-bit 觸發溢位分支」的測試策略，避開空轉盲點。**
因為 64-bit 上沒有任何 u32-衍生的 `hdr_len` 能使 `pos(=11) + hdr_len` 溢位，端對端 container 測試在 64-bit 上**無法**證明修補（前後都回 `Malformed`）。對策：helper 把 `pos` 當參數，測試直接傳 `pos = usize::MAX, hdr_len = 1, bytes = &[]`，強制走 `checked_add` 的 `None` 分支——
- 未修（`pos + hdr_len`）：64-bit debug 會 **panic** → 測試 fail（紅）。
- 修後（`checked_add`）：回 `Malformed` → 測試 pass（綠）。
這讓回歸測試在開發機（64-bit native）就真正紅綠。
- 替代方案：只靠 `wasm-pack test` 在 wasm32 驗 no-panic。保留為**選配補強**，但不作為唯一證據（CI／本地未必每次跑 wasm）。

**D3 — 錯誤型別沿用 `FcbError::Malformed`。**
與既有「header length out of bounds」「truncated u16/u32」一致，不新增 enum variant；對呼叫端零破壞。

## Implementation Contract

**Behavior（可觀察）：**
- 餵入任何標頭長度前綴會導致 `pos + hdr_len` 溢位 `usize`、或 `pos + hdr_len > bytes.len()` 的 container，`peek_header`/`read_container`（以及經由它們的 `open_case`/`open` 路徑）一律回 `Err(FcbError::Malformed(_))`，**在所有 target（含 wasm32 debug build）都不 panic**。
- 合法 container：`peek_header`/`read_container` 的回傳（kind/version/header/payload）與修補前逐位元組相同。

**Interface / data shape：**
- 新增一個私有界限計算 helper，語意等同 D1（吃 `bytes, pos, hdr_len`，回 `Result<&[u8]>`，內部 `checked_add` + `bytes.get`）。函式為 crate-internal，不影響公開 API。
- 公開 API（`peek_header`/`read_container`/`open*`）簽名與回傳型別不變。

**Failure modes：**
- 溢位／超界：`FcbError::Malformed`（沿用既有訊息語意）。
- 不引入新的 panic 路徑；不吞錯、不靜默回退。

**Acceptance criteria：**
- 新增 helper 紅綠測試：`pos = usize::MAX, hdr_len = 1` → `Err(Malformed)`；該測試在 revert helper 為樸素 `pos + hdr_len` 後於 64-bit debug 會 panic／fail（adversarial 以 revert-and-run 證非空轉）。
- 新增 conformance 測試：craft `hdr_len = 0xFFFFFFFF` 的短 container → `read_container` 回 `Err(Malformed)`。
- 既有測試全綠：`truncated_header_is_malformed`、`reader_version_is_two_for_aad_format`、`roundtrip_preserves_kind_version_and_header`、`crates/fcb/tests/vectors.rs` 全部 frozen vector 測試位元組不變。
- `cargo test --workspace` 全綠；`cargo clippy --all-targets --all-features -- -D warnings` exit 0；`cargo build -p fcb -p fcb-wasm --target wasm32-unknown-unknown` OK。

**Scope boundaries：**
- In scope：`container.rs` 標頭長度切片的 `peek_header`/`read_container` 兩處 + helper + 測試；選配 wasm32 no-panic 測試。
- Out of scope：`read_u16`/`read_u32`（已分析安全）、wire format／版本欄位、golden vectors、payload 解密/解壓路徑、2b 的 manifest 不變式。

## Risks / Trade-offs

- [回歸測試空轉（看似綠卻沒驗到修補）] → D2 的 helper-參數化 `pos=usize::MAX` 直驗 + adversarial revert-and-run 證紅。
- [誤改合法路徑行為] → frozen vector 位元組不變測試 + round-trip 測試守門。
- [遺漏其他溢位站點] → Decisions 已逐一列舉（hdr_len 兩處為唯一 attacker-controlled 大值；read_u16/u32 的 pos 恆小）；adversarial completeness lens 再覆核。
