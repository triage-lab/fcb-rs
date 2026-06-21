## Why

Container 解析時以 `pos + hdr_len` 計算標頭切片，其中 `hdr_len` 是從 wire 讀進來、由 attacker 控制的 `u32`（`crates/fcb/src/container.rs` 的 `peek_header`、`read_container`）。在 64-bit target 上這個加法不會溢位（`.get()` 回 `None` → `Malformed`），但在 **32-bit target（wasm32，`usize = u32`）**，`11 + u32::MAX` 會溢位 `usize`：**debug build（overflow-checks）直接 panic／abort——這是實際可觸及的缺陷（等同 DoS）**。release build 會 wrap，但因 `pos` 是固定前綴的小位移（11），wrap 後的 end 必小於 `pos`，`bytes.get(pos..end)` 反而是反向區間、今天就已被 `.get()` 回 `None` → `Malformed` 拒絕。fcb-wasm 橋接層正是編譯到 wasm32 對外提供 `openCase`，因此 debug panic 路徑可被觸及。應在下游（ba-case-builder）於 wasm 上消費 `openCase` 之前先收斂。

## What Changes

- 把 container 標頭長度的界限計算改用 **overflow-safe 的 `checked_add`**：`pos.checked_add(hdr_len)` 溢位時回傳 `FcbError::Malformed`，在所有 target 上都明確拒絕且不 panic（消除 wasm32 debug build 的 overflow panic，並消除日後 `pos` 若變大時 wrap 成錯誤切片的潛在 footgun）。
- 抽出一個**純函式 helper**，讓 `peek_header` 與 `read_container` 共用同一段界限計算，避免兩處各自維護。
- 新增測試：native 紅綠測試（直接對 helper 傳 `pos = usize::MAX` 觸發溢位分支，未修程式在 64-bit debug 會 panic、修後回 `Malformed`）；以及巨大 `hdr_len` 的 container conformance 測試。
- **非 BREAKING**：合法 container 的解析行為與輸出位元組完全不變；frozen golden vectors 不受影響。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `fcb-container-format`: 解析 container 時，對 wire-derived 的長度／位移欄位採用 overflow-safe 計算；任何會溢位 `usize` 或超出緩衝範圍的長度前綴，一律以 `Malformed` 拒絕、且不得 panic（涵蓋 32-bit／wasm32 target）。

## Impact

- Affected specs: `fcb-container-format`
- Affected code:
  - Modified: `crates/fcb/src/container.rs`（界限計算 helper + `peek_header`/`read_container` 改用之 + 測試）
  - Optional: `crates/fcb-wasm/src/lib.rs`（選配的 wasm32 no-panic 端對端測試）
