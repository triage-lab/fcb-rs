# fcb-case-builder Specification

## Purpose

TBD - created by archiving change 'add-pack-case-and-bundle-hash'. Update Purpose after archive.

## Requirements

### Requirement: Authoritative case payload envelope

The crate SHALL expose a public `CasePayload` type that models the `.case` plaintext payload as `{ streams: [StreamData] }`, and SHALL provide a single canonical serialization entry point that produces the payload's plaintext bytes. The canonical serialization MUST be byte-identical to the CBOR encoding of the `{ streams }` envelope used by the existing frozen case vector, so that producers and consumers agree on one authoritative byte sequence.

#### Scenario: Canonical serialization is the single source of payload bytes

- **WHEN** a caller builds a `CasePayload` from a list of streams and requests its canonical bytes
- **THEN** the returned bytes are the CBOR encoding of `{ streams: [StreamData] }`
- **AND** decoding those bytes back yields a `CasePayload` whose streams equal the input

#### Scenario: Canonical bytes preserve the frozen case vector

- **WHEN** the plaintext payload of the existing frozen case vector is reconstructed through the public canonical serialization
- **THEN** the resulting bytes are byte-identical to the payload originally used to build that frozen vector
- **AND** the full sealed `FROZEN_CASE_HEX` vector remains byte-stable


<!-- @trace
source: add-pack-case-and-bundle-hash
updated: 2026-06-21
code:
  - crates/fcb/src/case.rs
  - docs/fcb-reference.md
  - crates/fcb/src/submission.rs
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/crypto.rs
  - crates/fcb/src/lib.rs
  - docs/README.md
  - crates/fcb/src/compress.rs
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - crates/fcb/src/evidence.rs
  - crates/fcb/src/task.rs
  - crates/fcb/src/container.rs
tests:
  - crates/fcb/tests/stream_types.rs
  - crates/fcb/tests/vectors.rs
-->

---
### Requirement: Frozen canonical bundle hash

The crate SHALL define and freeze the canonical `bundle_hash` for a case as the SHA-256 of the canonical plaintext payload bytes, formatted as `sha256:<lowercase-hex>`. The crate SHALL expose a helper that computes this canonical hash directly from a `CasePayload`, so the definition is enforced by code rather than left to convention.

#### Scenario: Canonical hash is content-addressed over plaintext payload

- **WHEN** the canonical bundle hash is computed for a `CasePayload`
- **THEN** the result equals `compute_bundle_hash` applied to that payload's canonical bytes
- **AND** the result is stable across repeated calls and independent of any random salt or nonce

##### Example: frozen hash for a fixed payload

- **GIVEN** a `CasePayload` with two streams of fixed, known records
- **WHEN** the canonical bundle hash is computed
- **THEN** it equals a single frozen `sha256:` value pinned by a regression test


<!-- @trace
source: add-pack-case-and-bundle-hash
updated: 2026-06-21
code:
  - crates/fcb/src/case.rs
  - docs/fcb-reference.md
  - crates/fcb/src/submission.rs
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/crypto.rs
  - crates/fcb/src/lib.rs
  - docs/README.md
  - crates/fcb/src/compress.rs
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - crates/fcb/src/evidence.rs
  - crates/fcb/src/task.rs
  - crates/fcb/src/container.rs
tests:
  - crates/fcb/tests/stream_types.rs
  - crates/fcb/tests/vectors.rs
-->

---
### Requirement: Case bundle production

The crate SHALL provide a `pack_case` function that seals an evidence case into a `KIND=Case` bundle, mirroring the existing `pack_submission` shape. The function SHALL set the bundle's header `bundle_hash` to the canonical hash of the supplied payload, SHALL embed the stream manifest and any task specification into the plaintext header meta, and SHALL carry the canonical payload bytes as the encrypted body.

#### Scenario: Packed case round-trips through open

- **WHEN** a caller packs a case with a manifest, an optional task, and a payload, then opens the produced bytes with the same passphrase
- **THEN** the opened bundle is recognized as a case
- **AND** the stream manifest and task are readable from the plaintext header
- **AND** the decoded streams equal the input payload streams
- **AND** the header `bundle_hash` equals the canonical hash of the input payload

#### Scenario: Manifest supplies stream types

- **WHEN** a caller packs a case whose manifest declares a built-in type and a third-party type
- **THEN** opening and decoding the streams marks the built-in stream as having a built-in handler and the third-party stream as not built-in

##### Example: built-in and third-party streams

- **GIVEN** a manifest with `{id: "s0", type: "fcb.syslog.v1"}` and `{id: "s1", type: "acme.edr.v1"}`, and a payload carrying records for both ids
- **WHEN** the packed case is opened and its streams decoded
- **THEN** stream `s0` reports a built-in handler and stream `s1` does not, while both surface their records


<!-- @trace
source: add-pack-case-and-bundle-hash
updated: 2026-06-21
code:
  - crates/fcb/src/case.rs
  - docs/fcb-reference.md
  - crates/fcb/src/submission.rs
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/crypto.rs
  - crates/fcb/src/lib.rs
  - docs/README.md
  - crates/fcb/src/compress.rs
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - crates/fcb/src/evidence.rs
  - crates/fcb/src/task.rs
  - crates/fcb/src/container.rs
tests:
  - crates/fcb/tests/stream_types.rs
  - crates/fcb/tests/vectors.rs
-->

---
### Requirement: Shared envelope across producer and consumer

The WASM bridge SHALL consume the crate's public `CasePayload` type instead of defining its own duplicate, so there is exactly one envelope definition shared by the codec, its tests, and the bridge.

#### Scenario: Bridge decodes a packed case via the shared type

- **WHEN** a case packed through the public `pack_case` is opened by the bridge `open_case`
- **THEN** the bridge decodes its streams using the crate's public `CasePayload`
- **AND** the records surface byte-faithfully to the consumer

<!-- @trace
source: add-pack-case-and-bundle-hash
updated: 2026-06-21
code:
  - crates/fcb/src/case.rs
  - docs/fcb-reference.md
  - crates/fcb/src/submission.rs
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/bundle.rs
  - crates/fcb/src/crypto.rs
  - crates/fcb/src/lib.rs
  - docs/README.md
  - crates/fcb/src/compress.rs
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - crates/fcb/src/evidence.rs
  - crates/fcb/src/task.rs
  - crates/fcb/src/container.rs
tests:
  - crates/fcb/tests/stream_types.rs
  - crates/fcb/tests/vectors.rs
-->

---
### Requirement: Pack rejects manifest inconsistent with payload

`pack_case` SHALL reject, with a Malformed error and without producing a bundle, any case whose stream manifest is inconsistent with the payload. The set of stream ids declared by the manifest SHALL equal the set of stream ids present in the payload — with no missing id, no extra id, and no duplicate id on either side — and for each stream id the number of records carried by the payload SHALL equal the record count declared by that stream's manifest entry. For a case whose manifest and payload are consistent, `pack_case` SHALL produce byte-for-byte identical output to packing without this requirement; this requirement adds rejection of inconsistent cases only and changes nothing for consistent ones.

#### Scenario: Declared record count differs from payload

- **WHEN** a caller packs a case whose manifest declares a record count for a stream that differs from the number of records the payload carries for that stream
- **THEN** `pack_case` SHALL return a Malformed error and SHALL NOT produce a bundle

#### Scenario: Stream id set differs between manifest and payload

- **WHEN** a caller packs a case whose payload carries a stream id not declared in the manifest, or omits a stream id the manifest declares, or repeats a stream id on either side
- **THEN** `pack_case` SHALL return a Malformed error

#### Scenario: Consistent case packs unchanged

- **WHEN** a caller packs a case whose manifest stream id set and per-stream record counts all agree with the payload
- **THEN** `pack_case` SHALL succeed and produce byte-for-byte the same bundle as before this requirement
