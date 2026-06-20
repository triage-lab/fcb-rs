# fcb-evidence-model Specification

## Purpose

TBD - created by archiving change 'fcb-protocols'. Update Purpose after archive.

## Requirements

### Requirement: Self-describing typed streams

A bundle's decrypted payload SHALL consist of zero or more typed streams. The header `meta.streams[]` manifest SHALL declare, for each stream, an `id`, a `type` carrying an embedded schema version, and a record count. Built-in types SHALL NOT be a closed enumeration; the model SHALL permit arbitrary namespaced types.

#### Scenario: Manifest describes streams

- **WHEN** a reader parses the header of a decrypted bundle
- **THEN** it SHALL obtain, for each stream, the `id`, the versioned `type`, and the record count without parsing the stream body


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
### Requirement: Namespaced stream type identifiers

A stream `type` SHALL be a namespaced identifier ending in a version segment. Built-in types SHALL use the `fcb.` namespace; third-party types SHALL use a distinct namespace to avoid collision.

#### Scenario: Third-party type coexists with built-in

- **WHEN** a bundle contains both an `fcb.syslog.v1` stream and an `acme.edr.v1` stream
- **THEN** the reader SHALL treat both as first-class streams distinguished only by their `type` identifier

##### Example: mixed manifest

- **GIVEN** a manifest with streams of type `fcb.syslog.v1`, `fcb.netflow.v1`, and `acme.edr.v1`
- **WHEN** the reader lists the streams
- **THEN** all three SHALL appear, each retaining its declared type


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
### Requirement: Graceful degradation for unknown types

An unknown stream `type` SHALL NOT be treated as an error. A consumer that has no registered handler for a stream type SHALL fall back to a generic representation rather than failing the whole bundle.

#### Scenario: Unknown type falls back

- **WHEN** a consumer loads a stream whose `type` has no registered handler
- **THEN** the consumer SHALL render the stream with a generic fallback and SHALL continue loading the remaining streams

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