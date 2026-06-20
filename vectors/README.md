# FCB golden vectors（語言中性 fixture）

本目錄是 FCB wire format 的 **byte-exact golden vectors**，以語言中性形式（純 hex）保存，
供任何語言的 FCB 實作對齊 byte 相容性，不限 Rust。

| 檔案 | 內容 | 大小 |
|------|------|------|
| `frozen_case.hex` | 一個 `.case`（KIND=1）bundle 的完整位元組（lower-hex） | 578 bytes |
| `frozen_work.hex` | 一個 `.casework`（KIND=2）bundle 的完整位元組（lower-hex） | 423 bytes |

## 建出這些向量的決定性輸入

- **passphrase**：`lab-pass`
- **salt**（16 B，固定）：`53 41 4c 54 01 02 03 04 05 06 07 08 09 0a 0b 0c`
- **nonce**（24 B，固定）：`00 01 02 … 17`
- **KDF/AEAD**：Argon2id（m_cost=32、t_cost=1、p_cost=1，測試用快速參數）+ XChaCha20-Poly1305
- **bundle_hash**：placeholder `sha256:deadbeef`（向量不規範 bundle_hash 涵蓋範圍）

> ⚠️ `frozen_work.hex` 的 payload 是測試專用的 3 欄 `WorkPayload { case_id, bundle_hash, report }`，
> **不是** library 真正寫入 `.casework` 的 7 欄 `Submission`。目前沒有釘住真實 `Submission`
> on-disk 位元組的向量（見 `../docs/fcb-reference.md`）。

## 契約

任何 FCB 實作 **必須**能解開上述兩個 bundle；若以相同決定性輸入重建，**必須**產生逐位元相同的輸出。

- 權威 Rust 測試（凍結這些向量、含 byte-stability 斷言）：`../crates/fcb/tests/vectors.rs`
- 逐 byte 拆解與欄位佈局：`../docs/fcb-reference.md`、`../docs/fcb-wire-format.md`
