## Why

`fcb.netflow.v1` 與 `fcb.json.v1` 兩個 stream type 已列在 `BUILTIN_STREAM_TYPES`（`crates/fcb/src/evidence.rs`），`is_builtin_type` 對它們回傳 `true`，代表 codec 宣稱「有內建處理」。但兩者**沒有凍結的記錄 schema**——spec 與測試皆未定義其欄位集，也沒有對應的 byte-faithful round-trip 凍結（目前 `crates/fcb/tests/stream_types.rs` 只凍 `fcb.syslog.v1`）。這是 docs 的 Known Gap：宣告為內建卻無權威 schema，生產端與消費端各自臆測欄位、易漂移。本 change 比照 `fcb.syslog.v1` 的流程把這兩個 schema 定義並凍結。

## What Changes

- 定義並凍結 **`fcb.netflow.v1`** 記錄 schema：必填欄位為五元組（`src_ip`、`dst_ip`、`src_port`、`dst_port`、`proto`）加流量計數（`bytes`、`packets`）與時間區間（`ts_start`、`ts_end`，RFC 3339 UTC、結尾 `Z`）；選填欄位 `tcp_flags`（uint 累積旗標）、`app`（text，L7/應用標籤）。`proto` 為 IANA 協定號；無 port 的協定 `src_port`/`dst_port` 以 `0` 表示。值域約束（如 port 0–65535）為 spec-level、codec 不強制（與 `fcb.syslog.v1` 的 `severity`/`facility` 一致）。
- 定義並凍結 **`fcb.json.v1`** 記錄 schema：每筆記錄為**任意 CBOR map**（通用物件容器），無必填 key、值為任意 CBOR、逐位元保留；作為沒有專屬 schema 時的萬用容器。
- 在 `crates/fcb/tests/stream_types.rs` 新增 worked-example round-trip 凍結測試（比照 `syslog_v1_records_round_trip_byte_faithfully`），證明兩 schema 的欄位集／key 名／value 型別 byte-faithful 存活、且 `is_builtin` 為 `true`。
- `docs/fcb-data-model.md §3.2` 從「尚未定義」改為正式 schema 表。
- `docs/README.md`、`docs/fcb-wire-format.md §9`、`docs/fcb-reference.md §9` 移除「netflow/json 無記錄 schema」的 Known Gap 條目。

## Non-Goals (optional)

- **不改 codec 邏輯**：codec 已能 round-trip 任意 CBOR 記錄（`decode_streams` 不檢查記錄形狀），兩 type 也已在 `BUILTIN_STREAM_TYPES`；本 change 僅以測試凍結 schema、擴充 spec 與 docs，不動 `evidence.rs` 的派發或 `pack`/`open` 路徑。
- **不做值域強制**：port/proto 的值域是 spec-level 約束，codec 不驗證（與 syslog 一致）。
- **不新增 byte-stable golden 向量檔**（hex 凍結屬 §8 golden vectors / 其他 change）；本 change 用 round-trip 凍結（與 syslog 同模式）。
- **不解析**：不提供 netflow/json 的 parser／encoder helper，schema 僅定義記錄形狀。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `fcb-stream-types`: 新增 `fcb.netflow.v1` 與 `fcb.json.v1` 兩個記錄 schema requirement（附 worked-example），與既有 `fcb.syslog.v1`、演進規則、generic fallback 並列。

## Impact

- Affected specs: 修改 `fcb-stream-types`。
- Affected code:
  - Modified: crates/fcb/tests/stream_types.rs, docs/fcb-data-model.md, docs/README.md, docs/fcb-wire-format.md, docs/fcb-reference.md
  - New: (none)
  - Removed: (none)
