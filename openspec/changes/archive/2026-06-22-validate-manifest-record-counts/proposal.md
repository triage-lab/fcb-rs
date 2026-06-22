## Why

`pack_case` 把呼叫端提供的 stream manifest（宣告每條 stream 的 `records` 計數）嵌入明文 header，並把實際的 record stream 封進 payload，卻**不驗證兩者一致**。呼叫端（native 或 JS via fcb-wasm）因此能產出 manifest 宣告與 payload 實際筆數／id 集合不符的 `.case`。這種不一致今天只在 open／decode 端才可能浮現，而 `decode_streams` 只檢查單一方向（manifest 宣告的 id 在 payload 找不到）。屬前批信任硬化的 risk 3，原本裁定只進文件、未寫 code fix；本 change 補上 producer 端不變式，讓不一致在封裝當下就 loud reject。

## What Changes

- core `crates/fcb/src/case.rs::pack_case` 在計算 bundle_hash／封裝前，新增 producer 不變式（fail-fast）：
  - manifest 與 payload 的 stream **id 集合雙向相等**（payload 無缺漏、無多餘；manifest 內或 payload 內**重複 id** 亦視為不一致）。
  - 每個 id 對應的 `payload_stream.records.len() as u64 == manifest_entry.records`。
  - 任一不符 → `FcbError::Malformed`（可辨識訊息，含 stream id 與兩邊數字）。
- 不變式放在 **core**，故 native 與 fcb-wasm 的 pack 皆自動生效；`pack_work`／`pack_submission` 不動（`.casework` 無 manifest）。
- 對**合法（已相符）輸入位元組完全不變**；frozen golden vectors 與 byte-stable 測試不受影響。
- 新增 reject 測試：records 計數不符、payload 多出 stream、payload 缺 stream、重複 id。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `fcb-case-builder`: `pack_case` 對「manifest 宣告（stream id 集合與每 stream 的 `records` 計數）與 payload 實際 streams 不一致」的輸入 SHALL 以 `Malformed` 拒絕；對一致（合法）輸入的行為與輸出位元組不變。

## Impact

- Affected specs: `fcb-case-builder`
- Affected code:
  - Modified: `crates/fcb/src/case.rs`（producer 不變式 + 單元測試）
