## ADDED Requirements

### Requirement: Root README orients new users

The repository root SHALL contain a `README.md` that explains what FCB and the `fcb-rs` crate are, maps the repository layout (`crates/fcb`, `crates/fcb-wasm`, `docs/`, `openspec/`), and provides a quickstart for both the Rust and the WASM/JS consumption paths. It SHALL state the build and test commands, surface the CI status, declare the dual license, and link to the authoritative protocol docs under `docs/` rather than duplicating their content.

#### Scenario: a newcomer can get oriented and build

- **WHEN** a developer opens the repository root `README.md`
- **THEN** it SHALL describe the project and repository layout, show a Rust and a WASM quickstart, list the build/test commands, show CI status, state the license, and link to `docs/`

### Requirement: License file is present

The repository SHALL provide a root `LICENSE` file containing the canonical Educational Community License, Version 2.0 (ECL-2.0) text, matching the `license = "ECL-2.0"` declared in the crate manifests, so the SPDX expression is backed by the actual license text. ECL-2.0 is chosen as an education-oriented license appropriate to the project's teaching context.

#### Scenario: license text backs the SPDX expression

- **WHEN** a consumer checks the project license
- **THEN** the root SHALL contain a `LICENSE` file holding the ECL-2.0 text
- **AND** it SHALL correspond to the `ECL-2.0` SPDX expression in the crate manifests

### Requirement: Contributing guide states the workflow and invariants

The repository SHALL contain a `CONTRIBUTING.md` that describes the Spectra spec-driven workflow, the required quality gate (format, lint with warnings denied, the workspace test suite, and the WASM build), and the rule that existing byte-stable golden vectors MUST NOT be broken.

#### Scenario: a contributor learns the required checks

- **WHEN** a contributor reads `CONTRIBUTING.md`
- **THEN** it SHALL describe the spec-driven change workflow and the quality-gate commands
- **AND** it SHALL state that existing byte-stable golden vectors MUST stay green

### Requirement: Code of conduct is published

The repository SHALL contain a `CODE_OF_CONDUCT.md` establishing community behavior expectations and an enforcement contact.

#### Scenario: community standards are discoverable

- **WHEN** a participant reads `CODE_OF_CONDUCT.md`
- **THEN** it SHALL state the expected behavior and how to report violations

### Requirement: Security policy describes vulnerability reporting

As a cryptographic project, the repository SHALL contain a `SECURITY.md` that describes a private vulnerability reporting channel, which versions are supported, and the expected disclosure handling. It SHALL NOT instruct reporters to disclose vulnerabilities through public issues.

#### Scenario: a reporter finds a private channel

- **WHEN** someone discovers a potential vulnerability and reads `SECURITY.md`
- **THEN** it SHALL provide a private reporting channel and the supported-version and disclosure expectations
- **AND** it SHALL NOT direct the reporter to a public issue tracker for the report
