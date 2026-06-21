# fcb-submission Specification

## Purpose

TBD - created by archiving change 'fcb-protocols'. Update Purpose after archive.

## Requirements

### Requirement: Submission bundle format

A submission SHALL be encoded as an FCB container with `KIND=work`. Its decrypted payload SHALL contain the student's notes, report, activity log, and evidence references, together with `case_id`, `bundle_hash`, a student identity, and an export timestamp.

#### Scenario: Submission packaged as a work bundle

- **WHEN** the platform exports a student's work
- **THEN** it SHALL produce an FCB container whose KIND is work
- **AND** the payload SHALL include notes, report, activity, evidence references, `case_id`, `bundle_hash`, and an export timestamp


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
### Requirement: Case binding via case_id and bundle_hash

A submission SHALL embed the `case_id` and `bundle_hash` of the challenge it was produced from. A reviewing consumer SHALL be able to verify, using these fields, that a submission corresponds to a specific case and a specific evidence version.

#### Scenario: Reviewer verifies correspondence

- **WHEN** a reviewing platform opens a submission
- **THEN** it SHALL read the `case_id` and `bundle_hash` and SHALL be able to confirm whether they match a given challenge


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
### Requirement: Local work isolation by case

The platform SHALL key locally persisted student work by `case_id` so that work for different cases never mixes. When persisted work exists for a `case_id` but the opened challenge's `bundle_hash` differs from the stored one, the platform SHALL warn rather than silently attach the work to a different evidence version.

#### Scenario: Different cases stay isolated

- **WHEN** the student opens a second challenge with a different `case_id`
- **THEN** the platform SHALL load only the work stored under that `case_id`

#### Scenario: Evidence version mismatch warns

- **WHEN** persisted work exists for a `case_id` but the opened bundle's `bundle_hash` differs from the stored value
- **THEN** the platform SHALL surface a warning before associating the existing work with the opened bundle

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
### Requirement: Submission has a byte-stable golden vector

The test suite SHALL pin the on-disk bytes of a real seven-field `Submission` (`case_id`, `bundle_hash`, `student`, `notes`, `report`, `activity`, `exported_at`) with a frozen golden vector, built deterministically with a fixed salt and nonce. Rebuilding the vector from the same inputs SHALL reproduce the same bytes exactly, so an accidental change to the submission payload layout fails the test instead of silently breaking cross-implementation interoperability. The golden vector SHALL decode back to a `Submission` whose seven fields equal the inputs.

#### Scenario: rebuilding the submission vector reproduces the frozen bytes

- **WHEN** the frozen submission vector is rebuilt from its fixed inputs (fixed salt, nonce, and the seven-field `Submission`)
- **THEN** the produced bytes SHALL be byte-identical to the frozen vector
- **AND** any change to the submission payload layout SHALL fail this byte-stability test

#### Scenario: the frozen submission vector decodes to its seven fields

- **WHEN** the frozen submission vector is opened as a `.casework` with the correct passphrase
- **THEN** it SHALL decode to a `Submission` whose `case_id`, `bundle_hash`, `student`, `notes`, `report`, `activity`, and `exported_at` equal the inputs that produced the vector

<!-- @trace
source: freeze-submission-golden-vector
updated: 2026-06-21
code:
  - docs/fcb-reference.md
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
tests:
  - crates/fcb/tests/vectors.rs
-->