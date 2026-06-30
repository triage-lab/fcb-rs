## MODIFIED Requirements

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

## ADDED Requirements

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
