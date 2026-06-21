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

---
### Requirement: Passphrase-based Open and Stream Decoding

The WASM bridge SHALL expose an operation that, given bundle bytes and a passphrase, derives the key, decrypts and decompresses the payload, and returns the decoded streams (each stream's `id` and its records) for `.case` bundles. The returned record values SHALL preserve the codec's byte-faithful semantics, including the lossless `raw` field of `fcb.syslog.v1` records when present.

#### Scenario: Open and decode a case

- **WHEN** a caller passes valid `.case` bytes together with the correct passphrase
- **THEN** the bridge returns decoded streams whose records correspond to the records that were packed into the bundle

#### Scenario: Preserve lossless raw on decode

- **WHEN** a decoded `fcb.syslog.v1` record carried a `raw` field at pack time
- **THEN** the value surfaced through the bridge retains that `raw` field verbatim

---
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

---
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

---
### Requirement: Case Authoring (Pack Case)

The WASM bridge SHALL expose an operation that packs a sealed `.case` bundle from a case object — a `case_id`, a non-empty stream manifest, an optional task spec, and a payload of streams whose records are CBOR-representable values — together with a passphrase, returning the bundle bytes. The canonical payload and the resulting `bundle_hash` SHALL be identical to those the native producer yields for the same logical input; the sealed bundle bytes differ between packs because each seal mixes in a random salt and nonce. The operation SHALL reject a case whose manifest contains no streams with an error whose kind indicates a malformed condition. Each manifest entry SHALL carry its stream type under the key `type`, and the task report mode SHALL be encoded in lowercase (`steps` or `freeform`). For integer record values within the JavaScript safe-integer range, the canonical payload and the resulting `bundle_hash` SHALL NOT depend on the JavaScript number representation; integer values outside that range MUST be supplied as `BigInt` to preserve their integer encoding.

#### Scenario: Pack and reopen a case

- **WHEN** a caller passes a valid case object and the correct passphrase to the pack operation, then passes the returned bytes and the same passphrase to the open operation
- **THEN** the open operation returns the same `case_id`, the same streams and records (including the lossless `raw` field of `fcb.syslog.v1` records), and the same task spec
- **AND** the `bundle_hash` is stable across repeated packs of the same input

#### Scenario: JS-authored case hashes identically to native

- **WHEN** a case is authored through the bridge whose integer record values are within the JavaScript safe-integer range
- **THEN** its canonical payload bytes and `bundle_hash` equal those produced by the native producer for the same logical records

##### Example: integer record preserves the frozen bundle hash

- **GIVEN** a payload with stream `s0` records `["evt1","evt2"]` and stream `s1` records `[7]`, where `7` is supplied as a plain JS number
- **WHEN** the case is packed through the bridge and reopened
- **THEN** the reported `bundle_hash` is `sha256:376d586b42b0e800a6e78fea8bfb9a68cb569d033cc324b7b9b1800fc508eccf`, identical to the native producer's frozen case bundle hash

#### Scenario: Reject a case with no streams

- **WHEN** the pack operation receives a case whose manifest contains no streams
- **THEN** the bridge raises an error whose kind indicates a malformed condition rather than returning bytes

<!-- @trace
source: add-packcase-wasm-bridge
updated: 2026-06-21
code:
  - .understand-anything/intermediate/arch-input.json
  - .understand-anything/intermediate/review.json
  - .understand-anything/intermediate/batch-0.json
  - .understand-anything/.trash-1782036076/tmp/tour.json
  - .understand-anything/intermediate/assembled-graph.json
  - .understand-anything/intermediate/batch-3.json
  - .understand-anything/meta.json
  - .understand-anything/intermediate/batch-1.json
  - .understand-anything/.trash-1782036076/tmp/ua-inline-validate.cjs
  - .understand-anything/.understandignore
  - .understand-anything/intermediate/fingerprint-input.json
  - .understand-anything/.trash-1782036076/tmp/layers.json
  - .understand-anything/intermediate/batch-2.json
  - .understand-anything/knowledge-graph.json
-->

---
### Requirement: Deterministic numeric encoding at the pack boundary

The WASM bridge pack operations — packing a `.case` and packing a `.casework` submission — SHALL enforce that record values produce a deterministic CBOR encoding matching the native producer. An integer-valued number supplied as a plain JavaScript number whose magnitude lies outside the JavaScript safe-integer range SHALL be rejected with a discriminable error whose `kind` indicates a malformed condition, rather than silently producing a divergent canonical payload or bundle_hash. Integer values within the safe-integer range and genuine non-integer floating-point values SHALL be accepted and SHALL encode deterministically. Integer values outside the safe-integer range supplied as a `BigInt` SHALL be accepted and SHALL encode as CBOR integers without loss. This requirement SHALL apply equally to the case payload records and to the submission record fields.

#### Scenario: Safe-range integer encodes identically to native

- **WHEN** a case or submission is packed through the bridge whose integer record values are within the JavaScript safe-integer range
- **THEN** its canonical payload bytes and bundle_hash equal those the native producer yields for the same logical records

#### Scenario: Genuine float is accepted and deterministic

- **WHEN** a record value is a non-integer floating-point number such as 3.14
- **THEN** the pack operation SHALL accept it and the resulting canonical payload SHALL be byte-stable across repeated packs of the same input

#### Scenario: Out-of-range integer as a plain number is rejected

- **WHEN** a record value is an integer-valued plain JavaScript number whose magnitude is outside the JavaScript safe-integer range
- **THEN** the pack operation SHALL raise an error whose kind indicates a malformed condition and SHALL NOT return bytes

#### Scenario: Out-of-range integer as BigInt is preserved

- **WHEN** an out-of-safe-range integer record value is supplied as a BigInt
- **THEN** the pack operation SHALL encode it as a CBOR integer without loss and the native reader SHALL decode the same integer value

##### Example: numeric boundary matrix

| Record value (as authored) | Pack result | Rationale |
| -------------------------- | ----------- | --------- |
| `7` (plain number) | accepted, CBOR integer, hash matches native | safe-range integer |
| `9007199254740991` = 2^53-1 (plain number) | accepted, CBOR integer | largest safe integer |
| `3.14` (plain number) | accepted, CBOR float, byte-stable | genuine non-integer float |
| `9007199254740992` = 2^53 (plain number) | rejected, malformed | integer-valued, outside safe range |
| `9007199254740993n` = (2^53+1) as BigInt | accepted, CBOR integer, lossless | explicit BigInt for out-of-range integer |
