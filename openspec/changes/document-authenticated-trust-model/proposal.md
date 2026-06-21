## Why

風險 2（authenticate-header-as-aad）與風險 1（enforce-pack-numeric-contract）落地後，既有使用者文件會與實作不符且不完整：SECURITY.md 的「known design boundaries」目前敘述「plaintext header 未經 AEAD AAD 認證」「bundle_hash 由 producer 覆蓋」——這在 Change A 後變成**錯的**；docs/fcb-wire-format.md 的加密管線敘述未提 header 被當 AAD 認證、case open 重算 bundle_hash 的內容定址驗證；docs/fcb-cookbook.md 缺少 binding 對 re-pack 敏感、bundle_hash 作為低熵 payload 確認 oracle 的使用者 caveat；風險 3（manifest.records 為非強制的 advisory 宣告值）尚未以 by-design 註記說明。A、B 兩個 BREAKING 變更也需 CHANGELOG 記錄。

## What Changes

- 更新 SECURITY.md 的信任邊界敘述：明確記錄 container 的 AEAD 認證範圍（payload 加上整個 plaintext header via AAD）、以及保留的 by-design 邊界（低熵 payload 的 bundle_hash 確認 oracle、binding 身分對 re-pack 敏感、manifest.records 為 advisory 非強制）。
- 更新 docs/fcb-wire-format.md 與 docs/fcb-data-model.md：反映 header 被當 AAD 認證、以及 case open 重算 bundle_hash 的內容定址驗證。
- 為 docs/fcb-cookbook.md 補風險 4（hash oracle caveat）與風險 5（binding re-pack 敏感）的使用者 guidance recipe。
- 在 CHANGELOG.md 記錄 A、B 兩個 BREAKING 變更（header 經 AAD 認證、min_reader 1→2、pack 邊界數值契約）。
- 全程遵守 doc-language-standard：台灣慣用語、技術名詞保留英文、保留 hard-constraint tokens。

## Non-Goals

- 不改任何 codec 程式碼或行為（由 A、B 負責）。
- 不觸碰 normative 來源（openspec/specs 由各 change 的 spec delta 與 archive sync 負責）與 golden vectors。
- 不新增或更動 SECURITY.md 既有的 reporting channel / 支援版本政策（既有要求不變）。
- 不重寫既有正確且不受 A/B 影響的文件段落。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `oss-project-docs`: 新增「Security policy 記錄密碼學信任邊界」要求——SECURITY.md 須說明 AEAD 認證範圍與保留的 by-design 邊界。
- `user-reference-and-changelog`: 新增「reference 與 changelog 反映已認證的 container 模型」要求——CHANGELOG 記錄信任模型 BREAKING 變更；cookbook / reference 記錄 bundle_hash 內容定址驗證與 oracle caveat、以及 binding 對 re-pack 的敏感。

## Impact

- Affected specs: oss-project-docs, user-reference-and-changelog
- Affected code:
  - Modified: SECURITY.md, CHANGELOG.md, docs/fcb-wire-format.md, docs/fcb-data-model.md, docs/fcb-cookbook.md
  - New: (none)
  - Removed: (none)
