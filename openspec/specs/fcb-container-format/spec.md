# fcb-container-format Specification

## Purpose

TBD - created by archiving change 'fcb-protocols'. Update Purpose after archive.

## Requirements

### Requirement: Container magic and bundle kind

The container SHALL begin with the 4-byte magic sequence `0x89 0x46 0x43 0x42` (byte `0x89` followed by ASCII "FCB"). The magic SHALL be constant across all versions and SHALL NOT encode a version number. A 1-byte KIND field SHALL immediately follow the magic and SHALL distinguish a challenge bundle (case) from a submission bundle (work).

#### Scenario: Reader identifies a valid bundle

- **WHEN** a reader opens a file whose first 4 bytes are `0x89 0x46 0x43 0x42`
- **THEN** the reader SHALL accept it as an FCB container and read the KIND byte to determine whether it is a case or a work bundle

#### Scenario: Reader rejects a non-FCB file

- **WHEN** a reader opens a file whose first 4 bytes are not `0x89 0x46 0x43 0x42`
- **THEN** the reader SHALL fail with a BadMagic error and SHALL NOT attempt decryption


<!-- @trace
source: fcb-protocols
updated: 2026-06-20
code:
  - crates/fcb/src/error.rs
  - tsconfig.json
  - vitest.config.ts
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/compress.rs
  - crates/fcb/src/container.rs
  - crates/fcb/src/crypto.rs
  - package.json
  - crates/fcb/src/binding.rs
  - src/contracts/index.ts
  - src/contracts/plugin.ts
  - crates/fcb/src/task.rs
  - src/contracts/evidence.ts
  - crates/fcb/src/lib.rs
  - crates/fcb/src/wasm.rs
  - crates/fcb/src/cbor.rs
  - crates/fcb/src/evidence.rs
  - src/contracts/query.ts
  - Cargo.toml
  - crates/fcb/src/submission.rs
  - pnpm-workspace.yaml
  - crates/fcb/Cargo.toml
tests:
  - crates/fcb/tests/vectors.rs
  - src/contracts/plugin.test.ts
  - src/contracts/query.test.ts
-->

---
### Requirement: Multi-level versioning and graceful refusal

The container SHALL carry a `container_version` (u16) field after KIND, and the plaintext header SHALL carry a `header_schema_ver` and a `min_reader` field. A reader SHALL dispatch its parse strategy by `container_version`. A reader whose supported version is lower than the bundle's `min_reader` SHALL refuse gracefully rather than misparse.

#### Scenario: Reader too old refuses gracefully

- **WHEN** a bundle declares a `min_reader` greater than the reader's supported version
- **THEN** the reader SHALL fail with an UnsupportedVersion error and SHALL NOT emit partial or corrupted data

#### Scenario: Parse strategy dispatched by version

- **WHEN** a reader encounters a known `container_version`
- **THEN** the reader SHALL select the parse path defined for that version


<!-- @trace
source: fcb-protocols
updated: 2026-06-20
code:
  - crates/fcb/src/error.rs
  - tsconfig.json
  - vitest.config.ts
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/compress.rs
  - crates/fcb/src/container.rs
  - crates/fcb/src/crypto.rs
  - package.json
  - crates/fcb/src/binding.rs
  - src/contracts/index.ts
  - src/contracts/plugin.ts
  - crates/fcb/src/task.rs
  - src/contracts/evidence.ts
  - crates/fcb/src/lib.rs
  - crates/fcb/src/wasm.rs
  - crates/fcb/src/cbor.rs
  - crates/fcb/src/evidence.rs
  - src/contracts/query.ts
  - Cargo.toml
  - crates/fcb/src/submission.rs
  - pnpm-workspace.yaml
  - crates/fcb/Cargo.toml
tests:
  - crates/fcb/tests/vectors.rs
  - src/contracts/plugin.test.ts
  - src/contracts/query.test.ts
-->

---
### Requirement: Plaintext header

The bytes following the fixed prefix SHALL be a length-prefixed (`hdr_len` u32) plaintext CBOR header. The header SHALL contain the KDF salt and parameters, the AEAD nonce, the `case_id`, the `bundle_hash`, the `header_schema_ver`, the `min_reader`, and a `meta` object. These fields SHALL be readable without the passphrase.

#### Scenario: Header read before key derivation

- **WHEN** a reader opens a bundle before any passphrase is supplied
- **THEN** the reader SHALL be able to read the KDF salt and parameters, the AEAD nonce, and the `case_id` from the plaintext header


<!-- @trace
source: fcb-protocols
updated: 2026-06-20
code:
  - crates/fcb/src/error.rs
  - tsconfig.json
  - vitest.config.ts
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/compress.rs
  - crates/fcb/src/container.rs
  - crates/fcb/src/crypto.rs
  - package.json
  - crates/fcb/src/binding.rs
  - src/contracts/index.ts
  - src/contracts/plugin.ts
  - crates/fcb/src/task.rs
  - src/contracts/evidence.ts
  - crates/fcb/src/lib.rs
  - crates/fcb/src/wasm.rs
  - crates/fcb/src/cbor.rs
  - crates/fcb/src/evidence.rs
  - src/contracts/query.ts
  - Cargo.toml
  - crates/fcb/src/submission.rs
  - pnpm-workspace.yaml
  - crates/fcb/Cargo.toml
tests:
  - crates/fcb/tests/vectors.rs
  - src/contracts/plugin.test.ts
  - src/contracts/query.test.ts
-->

---
### Requirement: Compress-then-encrypt payload

The payload SHALL be produced by first compressing the serialized content with zstd and then encrypting the compressed bytes with the AEAD. A reader SHALL decrypt before decompressing. Encrypt-then-compress SHALL NOT be used.

#### Scenario: Payload pipeline order

- **WHEN** a writer produces the payload
- **THEN** it SHALL apply zstd compression first and AEAD encryption second
- **AND** a reader SHALL apply AEAD decryption first and zstd decompression second


<!-- @trace
source: fcb-protocols
updated: 2026-06-20
code:
  - crates/fcb/src/error.rs
  - tsconfig.json
  - vitest.config.ts
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/compress.rs
  - crates/fcb/src/container.rs
  - crates/fcb/src/crypto.rs
  - package.json
  - crates/fcb/src/binding.rs
  - src/contracts/index.ts
  - src/contracts/plugin.ts
  - crates/fcb/src/task.rs
  - src/contracts/evidence.ts
  - crates/fcb/src/lib.rs
  - crates/fcb/src/wasm.rs
  - crates/fcb/src/cbor.rs
  - crates/fcb/src/evidence.rs
  - src/contracts/query.ts
  - Cargo.toml
  - crates/fcb/src/submission.rs
  - pnpm-workspace.yaml
  - crates/fcb/Cargo.toml
tests:
  - crates/fcb/tests/vectors.rs
  - src/contracts/plugin.test.ts
  - src/contracts/query.test.ts
-->

---
### Requirement: Passphrase-based cryptography

The key SHALL be derived from the passphrase using Argon2id with the salt and parameters stored in the header. The payload SHALL be sealed with an AEAD (XChaCha20-Poly1305) that provides both confidentiality and integrity. A wrong passphrase and a tampered ciphertext SHALL each cause a distinct, non-silent failure rather than yielding partially-decoded data.

#### Scenario: Wrong passphrase

- **WHEN** a reader supplies an incorrect passphrase
- **THEN** AEAD verification SHALL fail and the reader SHALL return a WrongPassphrase error

#### Scenario: Tampered ciphertext

- **WHEN** any byte of the encrypted payload is modified
- **THEN** AEAD verification SHALL fail and the reader SHALL return a Corrupt error

<!-- @trace
source: fcb-protocols
updated: 2026-06-20
code:
  - crates/fcb/src/error.rs
  - tsconfig.json
  - vitest.config.ts
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/compress.rs
  - crates/fcb/src/container.rs
  - crates/fcb/src/crypto.rs
  - package.json
  - crates/fcb/src/binding.rs
  - src/contracts/index.ts
  - src/contracts/plugin.ts
  - crates/fcb/src/task.rs
  - src/contracts/evidence.ts
  - crates/fcb/src/lib.rs
  - crates/fcb/src/wasm.rs
  - crates/fcb/src/cbor.rs
  - crates/fcb/src/evidence.rs
  - src/contracts/query.ts
  - Cargo.toml
  - crates/fcb/src/submission.rs
  - pnpm-workspace.yaml
  - crates/fcb/Cargo.toml
tests:
  - crates/fcb/tests/vectors.rs
  - src/contracts/plugin.test.ts
  - src/contracts/query.test.ts
-->