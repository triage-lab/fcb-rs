# fcb-wasm-bridge Specification

## Purpose

Defines the WASM / JS interface contract over the FCB codec: how the browser workbench and other JavaScript consumers invoke the same Rust codec through wasm-bindgen. This capability is implemented by the `crates/fcb-wasm` crate (extract-fcb-rs Phase 2); the contract below is the target it is built against.

## Requirements

### Requirement: Passphrase-free Header Inspection

The WASM bridge SHALL expose an operation that reads an FCB bundle's plaintext header without requiring a passphrase, returning at minimum the bundle kind, container version, `case_id`, `bundle_hash`, `min_reader`, and the `meta` (stream manifest plus task spec) when present. The operation SHALL reject inputs whose first four bytes are not the FCB magic, and SHALL signal an unsupported-version condition when `min_reader` exceeds the reader version.

#### Scenario: Inspect header before unlock

- **WHEN** a caller passes the bytes of a valid `.case` bundle and no passphrase
- **THEN** the bridge returns the `case_id`, `min_reader`, and the stream manifest read from the plaintext header

#### Scenario: Reject non-FCB input

- **WHEN** a caller passes bytes whose first four bytes are not the FCB magic
- **THEN** the bridge raises an error whose kind identifies a bad-magic / malformed-container condition

### Requirement: Passphrase-based Open and Stream Decoding

The WASM bridge SHALL expose an operation that, given bundle bytes and a passphrase, derives the key, decrypts and decompresses the payload, and returns the decoded streams (each stream's `id` and its records) for `.case` bundles. The returned record values SHALL preserve the codec's byte-faithful semantics, including the lossless `raw` field of `fcb.syslog.v1` records when present.

#### Scenario: Open and decode a case

- **WHEN** a caller passes valid `.case` bytes together with the correct passphrase
- **THEN** the bridge returns decoded streams whose records correspond to the records that were packed into the bundle

#### Scenario: Preserve lossless raw on decode

- **WHEN** a decoded `fcb.syslog.v1` record carried a `raw` field at pack time
- **THEN** the value surfaced through the bridge retains that `raw` field verbatim

### Requirement: Discriminable Error Mapping

The WASM bridge SHALL map each codec error variant to a JS-visible error carrying a stable `kind` discriminator, so that callers can distinguish a wrong passphrase from a corrupt or tampered bundle and from a malformed container.

#### Scenario: Wrong passphrase is distinct from corrupt

- **WHEN** the open operation is invoked with an incorrect passphrase on an otherwise intact bundle
- **THEN** the raised error carries a `kind` indicating wrong-passphrase, distinct from the corrupt `kind`

##### Example: Error variant to kind discriminator

| Codec error | JS error `kind` |
| ----------- | --------------- |
| BadMagic | `bad-magic` |
| UnsupportedVersion | `unsupported-version` |
| Malformed | `malformed` |
| WrongPassphrase | `wrong-passphrase` |
| Corrupt | `corrupt` |

### Requirement: Submission, Bundle Hash, and Binding Operations

The WASM bridge SHALL expose packing and opening of `.casework` submissions, computation of the bundle hash over supplied bytes, and binding verification between a submission's `case_id`/`bundle_hash` pair and a case's `case_id`/`bundle_hash` pair, returning a three-state binding result (match, case mismatch, evidence-version mismatch). Opening a submission SHALL reject bundles whose kind is not `.casework`.

#### Scenario: Verify binding match

- **WHEN** binding verification receives a submission and a case whose `case_id` and `bundle_hash` are identical
- **THEN** the result indicates a match

#### Scenario: Detect evidence-version mismatch

- **WHEN** the `case_id` values match but the `bundle_hash` values differ
- **THEN** the result indicates an evidence-version mismatch

#### Scenario: Reject non-submission bundle on open

- **WHEN** the submission-open operation receives bytes whose container kind is `.case` rather than `.casework`
- **THEN** the bridge raises an error rather than returning a submission
