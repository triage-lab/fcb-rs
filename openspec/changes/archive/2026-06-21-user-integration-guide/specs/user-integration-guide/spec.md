## ADDED Requirements

### Requirement: Integration guide covers the Rust consumption path

The integration guide SHALL show how to depend on `fcb` as a Cargo git dependency and SHALL provide runnable snippets for opening a `.case` (header inspection, decryption, and stream decoding), producing a `.case`, producing and opening a `.casework`, and verifying case binding, using the crate's public API.

#### Scenario: a Rust consumer can open and produce bundles from the guide

- **WHEN** a Rust developer follows the guide
- **THEN** it SHALL show a Cargo git dependency declaration and runnable snippets for opening a `.case`, producing a `.case`, and producing/opening a `.casework`
- **AND** each snippet SHALL use the crate's public API (for example `bundle::open_bytes`, `case::pack_case`, `submission::pack_submission`)

### Requirement: Integration guide covers the WASM/JS consumption path

The integration guide SHALL show how to build the WASM package with `wasm-pack` and how a JS consumer calls the bridge surface (`peekHeader`, `openCase`, `openSubmission`, `packSubmission`, `computeBundleHash`, `verifyBinding`, `workKey`), including a thin adapter aligned with the `crates/fcb-wasm` style.

#### Scenario: a JS consumer can wire up the bridge from the guide

- **WHEN** a JS developer follows the guide
- **THEN** it SHALL show the `wasm-pack build` command and example calls to the bridge functions
- **AND** it SHALL present a small adapter that wraps those calls

### Requirement: Integration guide documents error-kind handling

The integration guide SHALL map the `FcbError` variants to the bridge `error_kind` strings (`bad-magic`, `unsupported-version`, `malformed`, `wrong-passphrase`, `corrupt`) and SHALL give guidance on handling each, so consumers can distinguish a wrong passphrase from a corrupt or tampered bundle.

#### Scenario: a consumer distinguishes error kinds

- **WHEN** an open operation fails
- **THEN** the guide SHALL let the consumer map the failure to a stable error kind and react appropriately (for example, prompt for the passphrase on `wrong-passphrase` versus reject the file on `corrupt`)

### Requirement: Integration guide states the golden-vector contract

The integration guide SHALL explain how consumers — especially non-Rust reimplementations — use the frozen golden vectors in `crates/fcb/tests/vectors.rs` as a cross-implementation compatibility baseline.

#### Scenario: a reimplementer validates against the vectors

- **WHEN** someone reimplements the codec in another language
- **THEN** the guide SHALL direct them to decode and byte-compare against the frozen vectors as the compatibility check

### Requirement: Integration guide describes the end-to-end flow and cross-links authoritative docs

The integration guide SHALL describe the end-to-end lifecycle (teacher issues a `.case`, student opens it and produces a `.casework`, the platform verifies binding on intake) and SHALL cross-link the authoritative protocol docs under `docs/` rather than restating their content.

#### Scenario: a reader sees the whole lifecycle and where to go deeper

- **WHEN** a developer reads the guide end to end
- **THEN** it SHALL present the issue → solve → submit → verify lifecycle
- **AND** it SHALL link to `docs/fcb-wire-format.md`, `docs/fcb-data-model.md`, and `docs/fcb-reference.md` for authoritative detail
