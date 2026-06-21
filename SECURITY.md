# 安全政策（Security Policy）

`fcb-rs` 做的是密碼學封裝（passphrase KDF、AEAD、雜湊），一旦加密或驗證相關的程式出 bug，波及面會特別大。所以發現疑似漏洞時，先別公開，請循下面的**私密**管道回報。我們採協調式揭露。

## 支援版本（Supported Versions）

專案還在 `0.x` 的早期階段，目前只維護最新的 `0.1.x`。

| 版本 | 是否支援安全修補 |
| ---- | :--------------: |
| `0.1.x` | ✅ |
| `< 0.1` | ❌ |

## 回報漏洞（Reporting a Vulnerability）

**請不要透過 public GitHub issue、discussion 或 PR 回報安全漏洞**。這等於在修補完成前就把風險攤在陽光下。

請改用 **GitHub 私密漏洞回報（Private Vulnerability Reporting）**：

1. 前往本 repo 的 **Security** 分頁。
2. 點選 **Report a vulnerability**，依表單填寫。

這樣會開啟一條只有維護者看得到的私密通道（GitHub Security Advisory）。

要是你沒辦法用 GitHub 私密回報，也可以寄 email 給維護者：**claude@fhsh.tp.edu.tw**，主旨請標上 `[fcb-rs security]`。

回報時請盡量提供：

- 受影響的版本 / commit。
- 重現步驟或 PoC（proof of concept）。
- 影響評估（例如：可繞過密碼驗證、洩漏明文、竄改未被偵測等）。

## 處理時程（Disclosure Handling）

- **確認收到**：我們會盡力在 **3 個工作天**內回覆，告訴你已經收到。
- **修補與揭露**：確認後，會跟回報者一起敲定修補與公開的時程。預設採**協調式揭露**，等修補釋出後才公開細節。
- 修補釋出時會在 release note / `CHANGELOG.md` 標註，回報者若同意，也會一併致謝。

## 範疇（Scope）

本政策涵蓋 `crates/fcb`（codec）與 `crates/fcb-wasm`（WASM/JS bridge）的程式碼。

需特別說明的**既知設計邊界**（非漏洞，屬設計取捨，詳見 [`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md)）：

- 明文 header **已被 AEAD 認證**：整段明文 container 前綴（magic、KIND、container_version、hdr_len、以及完整 header CBOR）會綁進 XChaCha20-Poly1305 的 AAD（additional authenticated data）。竄改 header 任一欄位（含 task prompt、manifest、`case_id`、`bundle_hash`）都會讓 `open` 失敗為 `Corrupt`。此外 `.case` 開封（`open_case`）會用解密後的 canonical payload **重算 `bundle_hash` 並和 header 值比對**，不符即 `Corrupt`，等於驗證了內容定址。
- `bundle_hash` 是**明文 payload 的 SHA-256**，存在不需 passphrase 就能讀的明文 header 裡。對**低熵／可猜的 payload**，它等於一個**確認 oracle**——能猜中 payload 的人可藉雜湊確認猜測。高熵／大型 payload 不受影響。
- **binding 對 re-pack 敏感**：因 `bundle_hash` 是內容定址，case payload **任何**重新封裝（即使只改一個 byte）都會得到新雜湊，讓既有 submission 的 binding 變成 `evidence-version-mismatch`。發題後請**凍結** case payload。
- **manifest.records 僅供參考**：stream manifest 宣告的 `records` 筆數是 advisory——pack 與 open 都**不會**拿它和實際記錄數核對；消費端應以開封後的 payload 為準推導筆數，不要信 peek 階段宣告的值。

如果你覺得這些邊界在某些情境下會變成實際風險，還是很歡迎走前面的私密管道來回報討論。
