## 1. SECURITY.md 信任邊界（Security policy documents the cryptographic trust boundaries）

- [x] 1.1 更新 SECURITY.md 的 known design boundaries：移除「plaintext header 未經 AEAD AAD 認證」的錯誤敘述，改記錄 AEAD 認證範圍（payload 加上整個 plaintext header via AAD）與保留的 by-design 邊界（bundle_hash 低熵 payload 確認 oracle、binding 對 re-pack 敏感、manifest.records 為 advisory 非強制），落地 `Security policy documents the cryptographic trust boundaries`。驗證：SECURITY.md 明確含上述四點且不再出現 header unauthenticated 敘述；改動段落經 humane-prose-audit 與 ai-slop-auditor PASS。

## 2. wire-format 與 data-model 反映認證模型

- [x] 2.1 更新 docs/fcb-wire-format.md 加密管線段落：說明整個 plaintext header 與 framing prefix（magic / KIND / container_version / hdr_len）被當 AEAD AAD 認證、header 任一 byte 竄改 → corrupt。驗證：內容含 AAD 認證敘述且與實作一致、文件索引 cross-link 無 dead link。
- [x] 2.2 更新 docs/fcb-data-model.md：說明 case open 重算 bundle_hash 的內容定址驗證，以及 .case 與 .casework 的差異（submission 的 bundle_hash 為 binding 參照、不重算）。驗證：內容與實作一致、cross-link 解析正常。

## 3. cookbook 使用者 caveat（Reference and changelog reflect the authenticated-container model）

- [x] 3.1 在 docs/fcb-cookbook.md 補兩則 caveat recipe：(a) bundle_hash 內容定址與低熵 payload 的確認 oracle 注意事項；(b) re-pack case payload 會使既有 submission binding 失效（與既有 evidence-version-mismatch recipe 串接）。驗證：兩則 recipe 各述明 goal 與相關 API、並 cross-link 整合指南／reference；改動段落經 prose 稽核 PASS。

## 4. CHANGELOG 記錄 BREAKING（Reference and changelog reflect the authenticated-container model）

- [x] 4.1 在 CHANGELOG.md 的 Unreleased 下新增 BREAKING 條目：plaintext header 經 AAD 認證、min_reader 1→2、pack 邊界數值契約（超 safe-range Number reject），落地 `Reference and changelog reflect the authenticated-container model` 的 changelog 面向。驗證：CHANGELOG 含三條 breaking 條目、符合 Keep-a-Changelog 風格、用台灣慣用語。

## 5. 文件一致性與語言標準

- [x] 5.1 確認所有改動遵守 doc-language-standard（台灣慣用語、技術名詞保留英文、hard-constraint tokens 不被改動），且 root README.md 與 docs/README.md 索引的 cross-link 無 dead link。驗證：全部改動段落經 humane-prose-audit PASS（0 Critical／0 High）、ai-slop-auditor AI-likelihood 落到 low，且 cross-link 檢查通過。
