## 1. CHANGELOG 與 cookbook

- [x] 1.1 （Requirement: Project keeps a changelog）新增 `CHANGELOG.md`（Keep a Changelog 格式，`Unreleased` 起頭）：記錄從 browser-arena 抽離，及本批 Added/Changed——`fcb::case`（pack_case + CasePayload + 凍結 canonical bundle_hash）、`fcb.netflow.v1`/`fcb.json.v1` schema 凍結、`Submission` byte-stable 向量（並註明「`Submission` golden-vector 驗證自本批起」）、OSS 文件、授權由 MIT OR Apache-2.0 改 ECL-2.0、整合指南、cookbook/rustdoc。完成定義：含 Unreleased 區、抽離事件、本批變更分組。驗證：人工審閱、`rg "Unreleased|browser-arena|ECL-2.0|pack_case" CHANGELOG.md` 命中。
- [x] 1.2 （Requirement: Cookbook provides task-oriented recipes）新增 `docs/fcb-cookbook.md`：至少 5 個 recipe（開封+驗 submission binding、重發 case 偵測 EvidenceVersionMismatch、解碼某 stream type、用 golden vector 驗相容、區分 wrong-passphrase vs corrupt），每個 recipe 標目標+涉及的 API 呼叫，交叉連結 `fcb-integration-guide.md` 與 `fcb-reference.md`。完成定義：≥5 recipe、各有目標與呼叫、含交叉連結。驗證：人工核對 API 名稱與 crates/* 一致、連結可解析。

## 2. rustdoc 補強

- [x] 2.1 （Requirement: Public API rustdoc builds warning-free with a runnable example）修 `crates/fcb-wasm/src/lib.rs` 對 `wasm_api` 的 broken intra-doc link（`cfg(wasm32)`-gated，native doc build 會壞）。完成定義：`RUSTDOCFLAGS="-D warnings" cargo doc -p fcb-wasm --no-deps` 不再因該連結失敗。驗證：該指令 exit 0。
- [x] 2.2 （Requirement: Public API rustdoc builds warning-free with a runnable example）在 `crates/fcb/src/lib.rs` 加一段 crate-level 可執行 doctest（`case::pack_case` → `bundle::open_bytes` round-trip，斷言 kind/case_id），並補強關鍵公開項目 doc comment。完成定義：doctest 存在且通過、rustdoc 全 workspace 零警告。驗證：`cargo test -p fcb --doc` 綠、`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` exit 0。

## 3. reference 層一致化

- [x] 3.1 （Requirement: Reference layer is cross-linked and consistent）`docs/README.md` 文件清單與 root `README.md` 文件入口納入 `docs/fcb-cookbook.md` 與 `CHANGELOG.md`；確認 user-facing docs 交叉連結無 dangling。完成定義：兩處索引含 cookbook 與 CHANGELOG、所有交叉連結指向存在檔案。驗證：`rg "fcb-cookbook|CHANGELOG" docs/README.md README.md` 命中、連結目標 `ls` 存在。

## 4. 品質關卡（本 phase 動到 .rs）

- [x] 4.1 跑品質關卡並全過：`cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`（零警告）、`cargo test --workspace`（含 doctest 與所有 `*_vector_is_byte_stable`）、`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`（零警告）、`wasm-pack build crates/fcb-wasm --target nodejs`（通過）。完成定義：五道全過。驗證：各指令 exit 0。
