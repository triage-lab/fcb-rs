## MODIFIED Requirements

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
