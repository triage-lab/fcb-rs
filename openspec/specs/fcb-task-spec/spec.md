# fcb-task-spec Specification

## Purpose

TBD - created by archiving change 'fcb-protocols'. Update Purpose after archive.

## Requirements

### Requirement: Embedded task definition

A challenge bundle SHALL embed a task definition in `meta.task`. The task definition SHALL declare a `report_mode` of either `steps` or `freeform`. When `report_mode` is `steps`, the definition SHALL include an ordered list of steps, each with an `id`, a `prompt`, and an `answer_type`.

#### Scenario: Steps mode defines ordered prompts

- **WHEN** an authoring tool sets `report_mode` to `steps`
- **THEN** the task definition SHALL contain an ordered list of steps, each with an `id`, a `prompt`, and an `answer_type`

#### Scenario: Freeform mode defines a single report

- **WHEN** an authoring tool sets `report_mode` to `freeform`
- **THEN** the task definition SHALL describe a single open report and SHALL NOT require a step list


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
### Requirement: Answer-safety invariant

The student-facing build of a challenge bundle SHALL NOT contain any answer, answer key, grading rubric, or step solution. Because the student client decrypts the entire bundle, any such data would be exposed to the student. Correct answers SHALL exist only outside the student build.

#### Scenario: Student build excludes answers

- **WHEN** an authoring tool produces the student build of a challenge bundle
- **THEN** the task definition SHALL contain prompts and structure only
- **AND** it SHALL NOT contain any field carrying a correct answer, rubric, or solution

##### Example: steps task in student build

- **GIVEN** a step `q1` with prompt "source IP of initial intrusion?" and `answer_type` "ip"
- **WHEN** the student build is produced
- **THEN** the step SHALL include `id`, `prompt`, and `answer_type`
- **AND** the step SHALL NOT include any answer or expected-value field

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