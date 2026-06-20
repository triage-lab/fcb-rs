# FCB 協定開發文件

針對想實作 **FCB（Forensic Case Bundle）** 生產端／消費端的開發者，尤其是 **encoder CLI**
（教師出題、打包 `.case`）。本目錄說明如何產生／讀取與 `crates/fcb` 參考實作 **byte-compatible**
的 `.case` 與 `.casework`。

## 文件

- [`fcb-wire-format.md`](./fcb-wire-format.md) — **外層信封**：container 位元組佈局、passphrase
  密碼學（Argon2id + XChaCha20-Poly1305）、compress-then-encrypt、CBOR 編碼規則與互通陷阱。
- [`fcb-data-model.md`](./fcb-data-model.md) — **內層資料結構**：header `meta`（stream manifest +
  task spec）、`.case` payload 信封、各 stream type 的記錄 schema（含 `fcb.syslog.v1` **草案**）、
  `.casework`（Submission）、binding 與答案安全不變量。

## 權威來源順序

1. **`crates/fcb/` 原始碼**（參考實作）— 任何衝突以此為準。
2. **`crates/fcb/tests/vectors.rs`** — `FROZEN_CASE_HEX` / `FROZEN_WORK_HEX` 兩個 byte-exact
   golden vectors，任何 FCB 實作都必須能解、重建時必須產生相同位元組。
3. **`openspec/specs/fcb-*`**（6 個 capability）— 正規行為契約。
4. **本目錄文件** — 補上 specs 未涵蓋的 byte-level / crypto 細節，並提出尚未正式化的記錄 schema 草案。

## 給 encoder 作者的第一個建議

> **Rust 寫的 encoder 直接相依 `fcb` crate**（它已是 `crate-type = ["cdylib", "rlib"]`），呼叫
> `bundle::pack_bytes`、`evidence` / `task` 的 `*_to_meta` helper 組 header，零 CBOR 漂移風險。
> 只有在用**非 Rust 語言**重寫 codec 時，才需要照 [`fcb-wire-format.md`](./fcb-wire-format.md)
> 逐位元對齊，並以 golden vectors 驗證。

> ⚠️ 目前 `fcb` crate **沒有**「組 `.case` payload 信封 `{ streams: [...] }`」與「算 `bundle_hash`
> 的正規定義」的公開 helper（見 data-model 文件「已知缺口」）。實作 encoder 時這兩塊要嘛自己補、
> 要嘛回頭在 crate 加 helper（建議後者，讓生產／消費兩端共用同一份程式碼）。
