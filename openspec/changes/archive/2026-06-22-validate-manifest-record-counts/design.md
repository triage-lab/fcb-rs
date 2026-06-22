## Context

`crates/fcb/src/case.rs::pack_case` 目前對 `CaseInput { manifest, payload, task, case_id }` 只做一個檢查（`manifest.is_empty()` → Malformed），接著 `payload.to_canonical_bytes()` → `compute_bundle_hash` → 把 manifest 嵌進 header meta → `bundle::pack_bytes`。

資料形狀：
- `StreamManifest { id: String, stream_type: String, records: u64 }`（`records` 是**計數**，`crates/fcb/src/evidence.rs`）。
- `CasePayload { streams: Vec<StreamData> }`，`StreamData { id: String, records: Vec<Value> }`（實際筆數 = `records.len()`）。

缺口（前批 risk 3）：pack_case 不驗 manifest 宣告（id 集合、每 stream `records` 計數）與 payload 實際 streams 是否一致。open 端 `evidence::decode_streams` 只檢查單一方向（manifest 的 id 在 payload 找不到 → Malformed），不檢查筆數、不檢查 payload 多出的 stream。

## Goals / Non-Goals

**Goals:**

- pack_case 在封裝前 fail-fast 拒絕 manifest 與 payload 不一致的輸入（id 集合不符或筆數不符）。
- 放 core，使 native 與 fcb-wasm pack 皆生效。
- 對一致（合法）輸入位元組完全不變。

**Non-Goals:**

- 不改 `pack_work`／`pack_submission`（`.casework` 無 manifest）。
- 不改 open／`decode_streams` 端（既有單向檢查保留；不在本 change 重複實作 consumer 端筆數檢查）。
- 不改 wire format、header 結構、bundle_hash 演算法、golden vectors。

## Decisions

**D1 — 不變式放在 core `case.rs::pack_case`，緊接 `manifest.is_empty()` 檢查之後、計算 hash 之前。**
此處 `input.manifest` 與 `input.payload.streams` 都已具備，fail-fast 可在任何昂貴運算前拒絕。fcb-wasm 的 `pack_case` wrapper 只是先跑 `check_numeric_determinism` 再呼叫 core `fcb_pack_case`，故 core 的不變式自動套用到 wasm pack——無需改 wasm。
- 替代方案：放 fcb-wasm 邊界。否決——只保護 JS 作者，native 作者仍可產出不一致 case。

**D2 — 用「consume-map」演算法，一次涵蓋缺漏／多餘／重複／筆數四種不一致。**

```
use std::collections::HashMap;
// 1) payload id -> 實際筆數；payload 內重複 id 即不一致
let mut counts: HashMap<&str, usize> = HashMap::new();
for s in &input.payload.streams {
    if counts.insert(s.id.as_str(), s.records.len()).is_some() {
        return Err(Malformed("payload declares stream id {id} more than once"));
    }
}
// 2) 逐 manifest entry 比對並「消耗」對應 payload id
for m in &input.manifest {
    match counts.remove(m.id.as_str()) {
        None => return Err(Malformed("manifest stream {id} has no matching payload stream")), // 含 manifest 重複 id（第二次 remove 失敗）
        Some(actual) if actual as u64 != m.records =>
            return Err(Malformed("manifest declares {m.records} records for {id} but payload carries {actual}")),
        Some(_) => {}
    }
}
// 3) 剩餘未被消耗的 payload stream = manifest 未宣告的多餘 stream
if let Some((extra, _)) = counts.iter().next() {
    return Err(Malformed("payload carries stream {extra} not declared in the manifest"));
}
```

四種不一致都被覆蓋：payload 重複 id（步驟 1）、manifest 宣告但 payload 缺（步驟 2 None）、筆數不符（步驟 2 Some 比對）、manifest 重複 id（步驟 2 第二次 remove → None）、payload 多餘 stream（步驟 3）。
- 替代方案：分別建兩個 HashSet 比對 + 另跑筆數迴圈。否決——consume-map 單次走訪即涵蓋全部、且自然偵測重複。

**D3 — 錯誤型別沿用 `FcbError::Malformed`，訊息可辨識（含 stream id 與兩邊數字）。**
與 `pack_case` 既有「case has no streams」一致，對呼叫端零破壞。

## Implementation Contract

**Behavior（可觀察）：**
- `pack_case`（及經由它的 fcb-wasm `packCase`）對下列任一輸入回 `Err(FcbError::Malformed(_))`，且**不**產生任何 bundle：
  (a) manifest 宣告的 stream id 集合與 payload 的 stream id 集合不相等（payload 缺漏或多餘）；
  (b) manifest 內或 payload 內出現重複 stream id；
  (c) 某 stream 的 `payload.records.len()` 與 manifest 該 stream 的 `records` 計數不符。
- 對一致（合法）輸入：`pack_case` 的輸出位元組與本 change 前**逐位元組相同**（含 bundle_hash、header、payload）。

**Interface / data shape：**
- `pack_case` 公開簽名與回傳型別不變；新增的是其內部、封裝前的驗證。
- 不新增 error variant；沿用 `FcbError::Malformed(String)`。

**Failure modes：**
- 不一致 → `Malformed`（discriminable 訊息含 stream id 與兩邊數字）。不靜默、不回退、不 panic。

**Acceptance criteria：**
- 新增單元測試（mirror `pack_case_rejects_empty_manifest`）：
  - records 計數不符 → Malformed；
  - payload 多出未宣告 stream → Malformed；
  - manifest 宣告但 payload 缺該 stream → Malformed；
  - 重複 id（manifest 或 payload）→ Malformed。
- 既有測試全綠：`pack_case_round_trips_and_binds_hash`、`crates/fcb/tests/vectors.rs` 的 `case_vector_is_byte_stable`、`frozen_case_*`（**位元組不變**）、fcb-wasm `js_authored_case_hashes_identically_to_native`。
- `cargo test --workspace` 全綠；`cargo clippy --all-targets --all-features -- -D warnings` exit 0；`cargo build -p fcb -p fcb-wasm --target wasm32-unknown-unknown` OK。

**Scope boundaries：**
- In scope：`crates/fcb/src/case.rs` 的 `pack_case` 不變式 + 單元測試。
- Out of scope：`pack_work`／`pack_submission`、open／`decode_streams`、wire format／header／bundle_hash 演算法／golden vectors、fcb-wasm 程式（自動繼承 core）。

## Risks / Trade-offs

- [誤拒合法 case（false reject）] → consume-map 語意明確；以 `pack_case_round_trips_and_binds_hash`（s0=2、s1=1 相符）與 frozen vector 守門，確認合法輸入照常 pack。
- [改動誤動 happy path 位元組] → frozen vector byte-stable 測試 + bundle_hash frozen 測試守門；不變式只在不一致時回 Err，不觸碰序列化。
- [漏掉某種不一致] → consume-map 四象限（缺/多/重複/筆數）+ adversarial bypass lens 覆核。
