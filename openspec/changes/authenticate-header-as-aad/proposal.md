## Why

`.case` / `.casework` 的 plaintext header（`case_id`、`bundle_hash`、KDF 參數、AEAD nonce、`key_check`、`meta` = stream manifest + task spec）目前**不被 AEAD 認證**——`crypto.rs` 的 seal/open 用空 AAD，`bundle.rs` 只把 payload 餵進 cipher；header 也不在 `bundle_hash` 內，且 `open_case` 不重算 `bundle_hash`。任何人都能改動 `.case` 的明文 header（含 task prompt、manifest、`case_id`、`bundle_hash`）而不被偵測，submission↔case 的 binding（`verifyBinding`）也因此**不是密碼學保證**，只是在比對沒人重算的宣告值。本變更把整個 header 綁進 AEAD 的 AAD，讓任何 header 竄改在 open 時當場失敗。

## What Changes

- 把整個 plaintext header 的 canonical bytes 當作 XChaCha20-Poly1305 的 **AAD**（additional authenticated data）餵進 seal/open 路徑（`crates/fcb/src/crypto.rs` 與 `crates/fcb/src/bundle.rs`）。AAD 在 open 前即可從明文 header 取得，不影響「無密碼讀 header」的能力。
- **BREAKING**：AEAD tag 會改變 → 既有 `.case` / `.casework` bundle 不再相容；golden vectors（`vectors/frozen_case.hex`、`vectors/frozen_work.hex`）重生，`crates/fcb/tests/vectors.rs` 的 frozen 常數同步更新。
- **BREAKING**：`min_reader` 由 1 提升為 2，讓 pre-AAD 的舊 reader 對新 bundle graceful refuse（`UnsupportedVersion`），而非誤讀。
- 不支援回讀舊（非 AAD）bundle——v0.1 採乾淨斷裂。
- 新增測試：改動 header 任一 byte（manifest / task / `case_id` 等）→ open 回 `Corrupt`；正常 round-trip 仍通過。

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `fcb-container-format`: 「Plaintext header」與「Passphrase-based cryptography」要求新增——plaintext header 受 AEAD AAD 認證；任何 header 竄改使 open 失敗為 `Corrupt`；`min_reader` 提升以對舊 reader graceful refuse。

## Impact

- Affected specs: fcb-container-format
- Affected code:
  - Modified: crates/fcb/src/crypto.rs, crates/fcb/src/bundle.rs, crates/fcb/src/container.rs, crates/fcb/tests/vectors.rs, vectors/frozen_case.hex, vectors/frozen_work.hex
  - New: (none)
  - Removed: (none)
