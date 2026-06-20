## 1. 凍結 schema（round-trip 測試，比照 syslog）

- [x] 1.1 （Requirement: fcb.netflow.v1 record schema）在 crates/fcb/tests/stream_types.rs 新增 `fcb.netflow.v1` 的 worked-example 記錄與 `netflow_v1_records_round_trip_byte_faithfully` 測試：建兩筆記錄——(a) HTTPS TCP flow（含全部 9 必填 + `tcp_flags=26` + `app="tls"`，值同 spec 範例）、(b) DNS UDP flow（僅 9 必填、`proto=17`、省略選填）——以既有 `round_trip` helper 打包→開封，斷言 `decoded.streams[0].records == originals`（欄位集／key 名／value 型別 byte-faithful）、manifest type=`fcb.netflow.v1` 且 `is_builtin` 為 `true`。完成定義：兩筆 netflow 記錄逐位元存活、is_builtin 真。驗證：`cargo test -p fcb --test stream_types netflow_v1_records_round_trip_byte_faithfully`。
- [x] 1.2 （Requirement: fcb.json.v1 record schema）在 crates/fcb/tests/stream_types.rs 新增 `fcb.json.v1` 的 worked-example 記錄與 `json_v1_records_round_trip_byte_faithfully` 測試：建兩筆任意 CBOR map——(a) 巢狀 alert 物件 `{kind, score:float, tags:[...], meta:{asn}}`（值同 spec 範例）、(b) minimal `{k:v}`——round-trip 後斷言 `decoded.streams[0].records == originals`（巢狀 map／array／float／int 全保留）、manifest type=`fcb.json.v1` 且 `is_builtin` 為 `true`。完成定義：任意巢狀物件逐位元存活、is_builtin 真。驗證：`cargo test -p fcb --test stream_types json_v1_records_round_trip_byte_faithfully`。

## 2. 文件

- [x] 2.1 將 docs/fcb-data-model.md §3.2 從「fcb.netflow.v1 / fcb.json.v1（內建但尚未定義）」改寫為正式 schema：為兩個 type 各補一張欄位表（欄位／CBOR 型別／值域約束／必填／說明），並附與測試一致的 worked-example，交叉連結 `crates/fcb/tests/stream_types.rs` 的新凍結測試與 `fcb-stream-types` spec。完成定義：§3.2 不再標「尚未定義」、含兩張 schema 表與範例。驗證：人工審閱 §3.2 內容與測試一致、`rg "尚未定義|尚未凍結" docs/fcb-data-model.md` 不再命中 netflow/json。
- [x] 2.2 移除 docs 內「netflow/json 無記錄 schema」的 Known Gap 條目並標記已關閉：docs/README.md §已知缺口、docs/fcb-wire-format.md §9、docs/fcb-reference.md §9 三處對應條目。完成定義：三檔皆不再把 netflow/json schema 列為未定義缺口。驗證：`rg "netflow.*schema|json.*schema" docs/README.md docs/fcb-wire-format.md docs/fcb-reference.md` 反映已關閉狀態、人工審閱。

## 3. 品質關卡

- [x] 3.1 跑品質關卡並全過：`cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`（零警告）、`cargo test --workspace`（含既有所有 `*_vector_is_byte_stable` 與 syslog 凍結測試持續綠）、`wasm-pack build crates/fcb-wasm --target nodejs`。完成定義：四關全綠。驗證：四道指令各自 exit 0。
