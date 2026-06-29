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

---
### Requirement: Reference and changelog reflect the authenticated-container model

The `CHANGELOG.md` SHALL record the breaking trust-model hardening as distinct entries: the plaintext header becoming authenticated via AEAD additional authenticated data, the `min_reader` increment that makes pre-authentication readers refuse the new bundles, and the pack-boundary numeric contract that rejects out-of-safe-range integer numbers supplied as plain JavaScript numbers. The cookbook and reference layer SHALL document the case-open content-address verification — the recomputation of `bundle_hash` from the decrypted payload — and its confirmation-oracle caveat for low-entropy payloads, and SHALL document that the submission-to-case binding is invalidated by any re-pack of the case payload. All such prose SHALL follow the documentation language standard.

#### Scenario: the changelog records the breaking trust-model change

- **WHEN** a reader opens `CHANGELOG.md` after this change ships
- **THEN** under an Unreleased or versioned heading it SHALL list the header authentication, the min_reader increment, and the pack-boundary numeric contract as breaking changes

#### Scenario: a consumer finds the content-address and binding caveats in the cookbook

- **WHEN** a developer consults the cookbook for verification and binding behavior
- **THEN** it SHALL describe the case-open bundle_hash recomputation and its confirmation-oracle caveat
- **AND** it SHALL state that re-packing a case payload invalidates prior submission bindings

---
### Requirement: Protocol reference docs are accurate and symbol-anchored

The protocol reference documents under `docs/` (`docs/fcb-reference.md`, `docs/fcb-wire-format.md`, `docs/fcb-data-model.md`) SHALL stay consistent with the implemented codec and SHALL reference source code by symbol name (function, type, or constant) rather than by source line number, so the documentation does not drift when code is refactored. Precise values SHALL be anchored to named constants and the frozen golden vectors. These documents SHALL NOT state a license other than the `ECL-2.0` declared in the crate manifests, SHALL NOT describe `pack_case` or `CasePayload` as absent, SHALL describe the capability set consistently with the actual `openspec/specs/` directories, and SHALL scope the manifest/payload superset gap to the reader path because the producer path (`pack_case`) enforces it.

#### Scenario: reference docs survive a code refactor without drifting

- **WHEN** a developer consults the protocol reference docs after a refactor that moves source lines
- **THEN** the docs SHALL still resolve to the named symbols and frozen golden vectors they cite
- **AND** they SHALL state the `ECL-2.0` license, present `pack_case` and `CasePayload` as available, and describe the capability set matching `openspec/specs/`

#### Scenario: reference docs carry no source line-number anchors

- **WHEN** the protocol reference docs reference implementation code
- **THEN** they SHALL identify it by function, type, or constant name rather than by a source line number
