## Why

fcb-wasm 的 packCase / packSubmission 透過 serde-wasm-bindgen 把任意 JS 值轉成 CBOR。fcb-wasm-bridge spec 既有的「Case Authoring (Pack Case)」要求**已明定**：safe-integer 範圍內的整數其 canonical payload 與 bundle_hash 不依賴 JS number 表示，而**範圍外的整數 MUST 以 BigInt 提供**以保留整數編碼。但這個契約目前**沒有被強制**——若作者把超出 safe-integer 範圍的整數當成普通 JS number 傳入，值在 JS 端就已精度流失，或被 serde-wasm-bindgen 編成 CBOR float（native 作者會用 u64/i64 整數），導致 canonical bytes 與 bundle_hash 與 native 分歧、binding 失效，而且**毫無報錯**。submission 的 record 值（notes / report / activity）同樣有此風險，但 spec 尚未涵蓋。既有 wasm 測試只覆蓋整數 7，是地板而非邊界。

## What Changes

- 在 packCase 與 packSubmission 邊界**落實既有 BigInt 契約**：偵測 record 值中「整數值但超出 JS safe-integer 範圍的普通 number」並以可辨識的 discriminable error 拒絕（authoring 時 loud fail），同時繼續接受 safe-range 整數與真正的浮點數，並讓 BigInt 的超範圍整數無損編成 CBOR 整數。
- 把數值確定性契約由「僅 packCase 的被動敘述」擴及 packSubmission，成為 pack 邊界的強制不變式。
- 新增邊界測試矩陣，覆蓋 2^53-1 / 2^53 / 2^53+1 / 大整數 / 真浮點 各情境，鎖定 JS↔native hash 一致或 loud reject 的行為。
- design 第一步須先以 wasm 測試**實證** serde-wasm-bindgen 對上述各值的實際 CBOR 編碼與 hash 結果，據此釘死偵測述詞；若實證顯示「拒絕」不可行或不必要，改以鎖定文件化行為的測試替代，但仍須消除靜默分歧。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `fcb-wasm-bridge`: 在 pack 邊界新增「數值確定性」強制要求——超範圍整數的普通 number 被 discriminable error 拒絕、safe-range 整數與真浮點被接受且確定性、BigInt 超範圍整數無損編碼；契約適用 packCase 與 packSubmission。

## Impact

- Affected specs: fcb-wasm-bridge
- Affected code:
  - Modified: crates/fcb-wasm/src/lib.rs
  - New: (none)
  - Removed: (none)
