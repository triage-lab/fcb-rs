## 1. 實作 overflow-safe 界限計算

- [ ] 1. 在 `crates/fcb/src/container.rs` 新增私有純函式 helper（語意：`bytes, pos, hdr_len` → `Result<&[u8]>`，內部 `pos.checked_add(hdr_len).and_then(|end| bytes.get(pos..end)).ok_or(FcbError::Malformed(...))`），並讓 `peek_header` 與 `read_container` 改呼叫它取標頭切片；保持合法 container 行為位元組不變。

## 2. 測試（TDD：先寫會紅的測試）

- [ ] 2. 新增 native 紅綠測試：直接呼叫 helper 傳 `pos = usize::MAX, hdr_len = 1, bytes = &[]`，斷言回 `Err(FcbError::Malformed(_))`。此測試對「未修的樸素 `pos + hdr_len`」在 64-bit debug 會 panic（紅），改 `checked_add` 後通過（綠）。
- [ ] 3. 新增 conformance 測試：craft 一個宣稱 `hdr_len = 0xFFFFFFFF` 的短 container（magic+KIND+container_version+hdr_len，後接少量 bytes），斷言 `read_container` 回 `Err(FcbError::Malformed(_))`、不 panic。
- [ ] 4. （選配）在 `crates/fcb-wasm` 新增 wasm32 no-panic 測試：餵 crafted bytes 給 `openCase`，斷言回 `Err` 而非 panic（`wasm-pack test --node`）。

## 3. 驗證

- [ ] 5. 跑 `cargo test --workspace`（含新測試 + 既有 `truncated_header_is_malformed`、frozen vectors 全綠）、`cargo clippy --all-targets --all-features -- -D warnings`（exit 0）、`cargo build -p fcb -p fcb-wasm --target wasm32-unknown-unknown`（OK）。
