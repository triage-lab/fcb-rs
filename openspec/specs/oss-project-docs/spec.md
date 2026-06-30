# oss-project-docs Specification

## Purpose

TBD - created by archiving change 'oss-project-docs'. Update Purpose after archive.

## Requirements

### Requirement: Root README orients new users

The repository root SHALL contain a `README.md` that explains what FCB and the `fcb-rs` crate are, maps the repository layout (`crates/fcb`, `crates/fcb-wasm`, `docs/`, `openspec/`), and provides a quickstart for both the Rust and the WASM/JS consumption paths. It SHALL state the build and test commands, surface the CI status, declare the dual license, and link to the authoritative protocol docs under `docs/` rather than duplicating their content.

#### Scenario: a newcomer can get oriented and build

- **WHEN** a developer opens the repository root `README.md`
- **THEN** it SHALL describe the project and repository layout, show a Rust and a WASM quickstart, list the build/test commands, show CI status, state the license, and link to `docs/`


<!-- @trace
source: oss-project-docs
updated: 2026-06-21
code:
  - crates/fcb-wasm/LICENSE
  - CONTRIBUTING.md
  - SECURITY.md
  - crates/fcb-wasm/Cargo.toml
  - crates/fcb/LICENSE
  - LICENSE
  - CODE_OF_CONDUCT.md
  - crates/fcb/Cargo.toml
  - README.md
-->

---
### Requirement: License file is present

The repository SHALL provide a root `LICENSE` file containing the canonical Educational Community License, Version 2.0 (ECL-2.0) text, matching the `license = "ECL-2.0"` declared in the crate manifests, so the SPDX expression is backed by the actual license text. ECL-2.0 is chosen as an education-oriented license appropriate to the project's teaching context. Every publishable crate (`crates/fcb`, `crates/fcb-wasm`) SHALL ship a `LICENSE` file in its packaged artifact, and that file SHALL carry a complete copyright attribution identical to the root `LICENSE`. The packaged LICENSE files SHALL NOT contain the unfilled ECL-2.0 appendix placeholders `[yyyy]` or `[name of copyright owner]`.

#### Scenario: license text backs the SPDX expression

- **WHEN** a consumer checks the project license
- **THEN** the root SHALL contain a `LICENSE` file holding the ECL-2.0 text
- **AND** it SHALL correspond to the `ECL-2.0` SPDX expression in the crate manifests

#### Scenario: packaged crate license carries complete attribution

- **WHEN** a publishable crate is packaged for crates.io
- **THEN** the `LICENSE` file inside the package SHALL contain a filled copyright line identical to the root `LICENSE`
- **AND** it SHALL NOT contain the literal placeholders `[yyyy]` or `[name of copyright owner]`

---
### Requirement: Contributing guide states the workflow and invariants

The repository SHALL contain a `CONTRIBUTING.md` that describes the Spectra spec-driven workflow, the required quality gate (format, lint with warnings denied, the workspace test suite, and the WASM build), and the rule that existing byte-stable golden vectors MUST NOT be broken.

#### Scenario: a contributor learns the required checks

- **WHEN** a contributor reads `CONTRIBUTING.md`
- **THEN** it SHALL describe the spec-driven change workflow and the quality-gate commands
- **AND** it SHALL state that existing byte-stable golden vectors MUST stay green


<!-- @trace
source: oss-project-docs
updated: 2026-06-21
code:
  - crates/fcb-wasm/LICENSE
  - CONTRIBUTING.md
  - SECURITY.md
  - crates/fcb-wasm/Cargo.toml
  - crates/fcb/LICENSE
  - LICENSE
  - CODE_OF_CONDUCT.md
  - crates/fcb/Cargo.toml
  - README.md
-->

---
### Requirement: Code of conduct is published

The repository SHALL contain a `CODE_OF_CONDUCT.md` establishing community behavior expectations and an enforcement contact.

#### Scenario: community standards are discoverable

- **WHEN** a participant reads `CODE_OF_CONDUCT.md`
- **THEN** it SHALL state the expected behavior and how to report violations


<!-- @trace
source: oss-project-docs
updated: 2026-06-21
code:
  - crates/fcb-wasm/LICENSE
  - CONTRIBUTING.md
  - SECURITY.md
  - crates/fcb-wasm/Cargo.toml
  - crates/fcb/LICENSE
  - LICENSE
  - CODE_OF_CONDUCT.md
  - crates/fcb/Cargo.toml
  - README.md
-->

---
### Requirement: Security policy describes vulnerability reporting

As a cryptographic project, the repository SHALL contain a `SECURITY.md` that describes a private vulnerability reporting channel, which versions are supported, and the expected disclosure handling. It SHALL NOT instruct reporters to disclose vulnerabilities through public issues.

#### Scenario: a reporter finds a private channel

- **WHEN** someone discovers a potential vulnerability and reads `SECURITY.md`
- **THEN** it SHALL provide a private reporting channel and the supported-version and disclosure expectations
- **AND** it SHALL NOT direct the reporter to a public issue tracker for the report

<!-- @trace
source: oss-project-docs
updated: 2026-06-21
code:
  - crates/fcb-wasm/LICENSE
  - CONTRIBUTING.md
  - SECURITY.md
  - crates/fcb-wasm/Cargo.toml
  - crates/fcb/LICENSE
  - LICENSE
  - CODE_OF_CONDUCT.md
  - crates/fcb/Cargo.toml
  - README.md
-->

---
### Requirement: Security policy documents the cryptographic trust boundaries

The `SECURITY.md` SHALL document the container's cryptographic trust boundaries: what the AEAD authenticates — the encrypted payload together with the entire plaintext header and its framing prefix, bound as additional authenticated data — and what remains a deliberate by-design boundary rather than a cryptographic guarantee. The documented by-design boundaries SHALL include at least: the `bundle_hash` is a content address over the plaintext payload and can act as a confirmation oracle for low-entropy payloads; the submission-to-case binding identity is sensitive to any re-pack of the case payload; and the stream manifest's declared record counts are advisory metadata not enforced against the payload. The descriptions SHALL be consistent with the implemented codec behavior and SHALL NOT describe the plaintext header as unauthenticated.

#### Scenario: a reader learns what is and is not cryptographically guaranteed

- **WHEN** a reader opens `SECURITY.md` to understand the threat model
- **THEN** it SHALL state that the plaintext header is authenticated as AEAD additional authenticated data
- **AND** it SHALL list the remaining by-design boundaries, including the bundle_hash confirmation oracle, the re-pack sensitivity of the binding, and the advisory nature of manifest record counts

---
### Requirement: CI enforces the documented quality gate

The continuous integration workflow SHALL enforce the same quality gate that `CONTRIBUTING.md` and `README.md` document as required before merge: a formatting check that fails on unformatted code, a clippy lint pass over the workspace that treats warnings as errors, and the workspace test suite. The workflow SHALL additionally protect the WebAssembly distribution path on which downstream consumers depend: it SHALL build the `fcb-wasm` bridge with a pinned `wasm-pack` version in release mode for both the `web` and the `nodejs` target, SHALL run the `fcb-wasm` `wasm-bindgen` test suite under a wasm runtime, and SHALL build the workspace under a pinned toolchain matching the crates' declared minimum supported Rust version. Any gate failure SHALL fail the workflow, so unformatted, warning-laden, behaviorally-regressed, or MSRV-violating code cannot merge green.

#### Scenario: unformatted or warning-laden code fails CI

- **WHEN** a commit that fails the formatting check or produces a clippy warning is pushed or opened as a pull request
- **THEN** the CI workflow SHALL fail on the corresponding gate rather than report success

#### Scenario: the wasm bridge is built reproducibly for both targets

- **WHEN** the CI workflow runs
- **THEN** it SHALL build the `fcb-wasm` bridge with a pinned `wasm-pack` version in release mode for the `web` target and for the `nodejs` target

#### Scenario: the wasm boundary behavior is tested in CI

- **WHEN** the CI workflow runs
- **THEN** it SHALL execute the `fcb-wasm` `wasm-bindgen` test suite under a wasm runtime
- **AND** a failure of any wasm boundary test SHALL fail the workflow

#### Scenario: the declared MSRV is enforced

- **WHEN** the CI workflow runs
- **THEN** it SHALL build the workspace under a pinned toolchain matching the crates' declared `rust-version`
- **AND** a build failure under that toolchain SHALL fail the workflow

---
### Requirement: Project publishes citation metadata

The repository SHALL contain a root `CITATION.cff` file in Citation File Format that lets GitHub and CFF-aware tools render a citation for the project. It SHALL declare the project title, the software type, the authors, the repository URL, the `ECL-2.0` license, and a released version and date consistent with the crate manifests and `CHANGELOG.md`.

#### Scenario: a citation can be rendered from the metadata

- **WHEN** a user views the repository on GitHub or runs a CFF-aware tool
- **THEN** a citation SHALL be produced from `CITATION.cff` carrying the project title, authors, version, and the `ECL-2.0` license

---
### Requirement: Publishable crate manifests carry crates.io metadata

Each publishable crate manifest (`crates/fcb/Cargo.toml`, `crates/fcb-wasm/Cargo.toml`) SHALL declare the metadata required for a clean, discoverable crates.io publication: `keywords`, `categories`, `documentation`, and a minimum supported Rust version via `rust-version`. Any intra-workspace path dependency that is itself a publishable crate SHALL also declare a `version` requirement, so the depending crate is packageable; a path-only dependency SHALL NOT be its sole form. The declared `categories` SHALL be valid crates.io category slugs, and `keywords` SHALL satisfy the crates.io constraints of at most five entries, each at most twenty characters.

#### Scenario: each publishable crate packages without error

- **WHEN** `cargo package` runs against a publishable crate in the workspace
- **THEN** the manifest SHALL provide every field crates.io requires for the package step to succeed
- **AND** no dependency SHALL be rejected for missing a version requirement

#### Scenario: the crate is discoverable and toolchain-honest on crates.io

- **WHEN** the published crate is rendered on crates.io
- **THEN** it SHALL surface its `keywords` and `categories`
- **AND** it SHALL declare a `rust-version` so consumers on older toolchains receive a clear minimum rather than a cryptic build failure
