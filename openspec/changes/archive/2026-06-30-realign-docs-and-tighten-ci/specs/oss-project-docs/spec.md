## ADDED Requirements

### Requirement: CI enforces the documented quality gate

The continuous integration workflow SHALL enforce the same quality gate that `CONTRIBUTING.md` and `README.md` document as required before merge: a formatting check that fails on unformatted code, a clippy lint pass over the workspace that treats warnings as errors, the workspace test suite, and a WebAssembly build of the `fcb-wasm` bridge crate. Any gate failure SHALL fail the workflow, so unformatted or warning-laden code cannot merge green.

#### Scenario: unformatted or warning-laden code fails CI

- **WHEN** a commit that fails the formatting check or produces a clippy warning is pushed or opened as a pull request
- **THEN** the CI workflow SHALL fail on the corresponding gate rather than report success

#### Scenario: the wasm bridge is built in CI

- **WHEN** the CI workflow runs
- **THEN** it SHALL produce a WebAssembly build of the `fcb-wasm` bridge crate in addition to running the workspace test suite

### Requirement: Project publishes citation metadata

The repository SHALL contain a root `CITATION.cff` file in Citation File Format that lets GitHub and CFF-aware tools render a citation for the project. It SHALL declare the project title, the software type, the authors, the repository URL, the `ECL-2.0` license, and a released version and date consistent with the crate manifests and `CHANGELOG.md`.

#### Scenario: a citation can be rendered from the metadata

- **WHEN** a user views the repository on GitHub or runs a CFF-aware tool
- **THEN** a citation SHALL be produced from `CITATION.cff` carrying the project title, authors, version, and the `ECL-2.0` license
