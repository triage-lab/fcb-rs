## ADDED Requirements

### Requirement: Authenticated plaintext header

The plaintext header SHALL be cryptographically authenticated by binding its bytes — together with the framing prefix that precedes it (the magic, KIND, container_version, and hdr_len) — as the AEAD additional authenticated data (AAD) used to seal the payload. A reader SHALL reconstruct the identical AAD from the bytes it reads before decryption, so that any modification to a header field (including case_id, bundle_hash, KDF parameters, AEAD nonce, key_check, or the meta object carrying the stream manifest and task spec) or to the framing prefix causes AEAD verification to fail. The header SHALL remain readable without the passphrase.

#### Scenario: Header authenticated without breaking passphrase-free read

- **WHEN** a reader opens a bundle before supplying a passphrase
- **THEN** it SHALL still read the plaintext header fields
- **AND** when the passphrase is later supplied, decryption SHALL verify the header bytes as AAD

#### Scenario: Header field tamper detected

- **WHEN** an attacker modifies any header field (for example the task prompt inside the meta object) without knowing the passphrase
- **THEN** a subsequent open with the correct passphrase SHALL fail with a Corrupt error and SHALL NOT return any decoded data

### Requirement: Verified content address on case open

When opening a case bundle, a reader SHALL recompute the bundle_hash from the decrypted canonical payload and SHALL compare it to the bundle_hash declared in the header; a mismatch SHALL fail with a Corrupt error. This verification SHALL apply to case bundles only — a submission bundle's header bundle_hash is a binding reference to its case, not the hash of the submission payload, and SHALL NOT be recomputed from the submission payload.

#### Scenario: Case declared hash matches payload

- **WHEN** a reader opens a valid case bundle with the correct passphrase
- **THEN** the recomputed bundle_hash SHALL equal the header bundle_hash and the open SHALL succeed

#### Scenario: Case declared hash does not match payload

- **WHEN** a case bundle's header declares a bundle_hash that does not equal the hash of its canonical payload
- **THEN** the reader SHALL fail with a Corrupt error and SHALL NOT return any decoded data

## MODIFIED Requirements

### Requirement: Passphrase-based cryptography

The key SHALL be derived from the passphrase using Argon2id with the salt and parameters stored in the header. The payload SHALL be sealed with an AEAD (XChaCha20-Poly1305) that provides both confidentiality and integrity, and the entire plaintext header together with its framing prefix (magic, KIND, container_version, hdr_len) SHALL be bound as the AEAD additional authenticated data (AAD). A wrong passphrase, a tampered ciphertext, and a tampered header SHALL each cause a distinct, non-silent failure rather than yielding partially-decoded data.

#### Scenario: Wrong passphrase

- **WHEN** a reader supplies an incorrect passphrase
- **THEN** AEAD verification SHALL fail and the reader SHALL return a WrongPassphrase error

#### Scenario: Tampered ciphertext

- **WHEN** any byte of the encrypted payload is modified
- **THEN** AEAD verification SHALL fail and the reader SHALL return a Corrupt error

#### Scenario: Tampered header

- **WHEN** any byte of the plaintext header or its framing prefix (magic, KIND, container_version, hdr_len) is modified
- **THEN** AEAD verification SHALL fail and the reader SHALL return a Corrupt error
