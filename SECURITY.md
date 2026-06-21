# 安全政策（Security Policy）

`fcb-rs` 做密碼學封裝（passphrase KDF、AEAD、雜湊），所以加密或驗證相關的 bug 影響特別大。發現疑似漏洞請循下列**私密**管道回報、先別公開——我們採協調式揭露。

## 支援版本（Supported Versions）

專案目前處於 `0.x` 早期階段，僅維護最新的 `0.1.x`。

| 版本 | 是否支援安全修補 |
| ---- | :--------------: |
| `0.1.x` | ✅ |
| `< 0.1` | ❌ |

## 回報漏洞（Reporting a Vulnerability）

**請不要透過 public GitHub issue、discussion 或 PR 回報安全漏洞**——這會在修補前先公開風險。

請改用 **GitHub 私密漏洞回報（Private Vulnerability Reporting）**：

1. 前往本 repo 的 **Security** 分頁。
2. 點選 **Report a vulnerability**，依表單填寫。

這會開啟一條只有維護者看得到的私密通道（GitHub Security Advisory）。

若你無法使用 GitHub 私密回報，可改以 email 聯繫維護者：**claude@fhsh.tp.edu.tw**，主旨請標註 `[fcb-rs security]`。

回報時請盡量提供：

- 受影響的版本 / commit。
- 重現步驟或 PoC（proof of concept）。
- 影響評估（例如：可繞過密碼驗證、洩漏明文、竄改未被偵測等）。

## 處理時程（Disclosure Handling）

- **確認收到**：我們會盡力在 **3 個工作天**內回覆確認。
- **修補與揭露**：確認後會與回報者協調修補與公開時程；預設採**協調式揭露**，待修補釋出後再公開細節。
- 修補釋出時會在 release note / `CHANGELOG.md` 標註，並（若回報者同意）致謝。

## 範疇（Scope）

本政策涵蓋 `crates/fcb`（codec）與 `crates/fcb-wasm`（WASM/JS bridge）的程式碼。

需特別說明的**既知設計邊界**（非漏洞，屬設計取捨，詳見 [`docs/fcb-wire-format.md`](./docs/fcb-wire-format.md)）：

- 明文 header **未被 AEAD 認證**（無 AAD）——header 竄改不由 codec 偵測，需上層自行保護。
- `bundle_hash` 涵蓋範圍由生產端負責，低階 `compute_bundle_hash` 不驗證。

若你認為上述邊界在特定情境構成實際風險，仍歡迎循上述私密管道回報討論。
