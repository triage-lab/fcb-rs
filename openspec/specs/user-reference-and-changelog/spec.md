# user-reference-and-changelog Specification

## Purpose

TBD - created by archiving change 'user-reference-and-changelog'. Update Purpose after archive.

## Requirements

### Requirement: Project keeps a changelog

The repository SHALL contain a `CHANGELOG.md` in a recognizable Keep-a-Changelog style. It SHALL record the extraction from the originating browser-arena repository and the changes introduced in this batch (the codec gap closures and the documentation/licensing work), grouped under an `Unreleased` (or versioned) heading.

#### Scenario: a reader finds the project history

- **WHEN** a user opens `CHANGELOG.md`
- **THEN** it SHALL list, under dated or `Unreleased` headings, the extraction event and this batch's notable changes grouped by kind (for example Added / Changed)


<!-- @trace
source: user-reference-and-changelog
updated: 2026-06-21
code:
  - docs/README.md
  - crates/fcb/src/bundle.rs
  - docs/fcb-integration-guide.md
  - CHANGELOG.md
  - docs/fcb-cookbook.md
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/lib.rs
  - README.md
-->

---
### Requirement: Cookbook provides task-oriented recipes

The repository SHALL contain a `docs/fcb-cookbook.md` with self-contained recipes for common consumer tasks (for example: open and verify a submission's binding, detect an evidence-version mismatch after a reissue, decode a specific stream type, validate against a golden vector, distinguish a wrong passphrase from a corrupt bundle). Each recipe SHALL state its goal and show the calls involved, and SHALL cross-link the integration guide and the reference for depth.

#### Scenario: a consumer solves a concrete task from a recipe

- **WHEN** a developer needs to perform a common task and opens the cookbook
- **THEN** it SHALL provide a recipe naming the goal and the API calls involved
- **AND** it SHALL link to the integration guide and/or the protocol reference for further detail


<!-- @trace
source: user-reference-and-changelog
updated: 2026-06-21
code:
  - docs/README.md
  - crates/fcb/src/bundle.rs
  - docs/fcb-integration-guide.md
  - CHANGELOG.md
  - docs/fcb-cookbook.md
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/lib.rs
  - README.md
-->

---
### Requirement: Public API rustdoc builds warning-free with a runnable example

The crate's public API documentation SHALL build with no rustdoc warnings under `RUSTDOCFLAGS="-D warnings"`, with broken intra-doc links resolved. The `fcb` crate SHALL include at least one runnable crate-level doctest demonstrating the end-to-end pack/open path through the public API.

#### Scenario: rustdoc builds clean and the example runs

- **WHEN** `cargo doc --workspace --no-deps` runs with rustdoc warnings denied
- **THEN** it SHALL succeed with no broken intra-doc link or other rustdoc warning
- **AND** `cargo test --doc` SHALL execute and pass the crate-level example


<!-- @trace
source: user-reference-and-changelog
updated: 2026-06-21
code:
  - docs/README.md
  - crates/fcb/src/bundle.rs
  - docs/fcb-integration-guide.md
  - CHANGELOG.md
  - docs/fcb-cookbook.md
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/lib.rs
  - README.md
-->

---
### Requirement: Reference layer is cross-linked and consistent

The documentation index SHALL present a consistent reference layer: the root `README.md` and `docs/README.md` SHALL list the cookbook and the `CHANGELOG.md` alongside the existing protocol docs and the integration guide, and the cross-links between these documents SHALL resolve to existing files.

#### Scenario: the documentation set is navigable without dead links

- **WHEN** a reader navigates from the README doc index
- **THEN** the cookbook and changelog SHALL be discoverable from the index
- **AND** every cross-link among the user-facing docs SHALL point to an existing file

<!-- @trace
source: user-reference-and-changelog
updated: 2026-06-21
code:
  - docs/README.md
  - crates/fcb/src/bundle.rs
  - docs/fcb-integration-guide.md
  - CHANGELOG.md
  - docs/fcb-cookbook.md
  - crates/fcb-wasm/src/lib.rs
  - crates/fcb/src/lib.rs
  - README.md
-->