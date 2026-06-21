## MODIFIED Requirements

### Requirement: Integration guide covers the WASM/JS consumption path

The integration guide SHALL show how to build the WASM package with `wasm-pack` and how a JS consumer calls the bridge surface (`peekHeader`, `openCase`, `openSubmission`, `packSubmission`, `packCase`, `computeBundleHash`, `verifyBinding`, `workKey`), including a thin adapter aligned with the `crates/fcb-wasm` style. For `packCase`, the guide SHALL document the JS case object shape and its two footguns: each manifest entry uses the key `type` (not `stream_type`), and integer record values MUST stay within the JavaScript safe-integer range — values beyond it MUST be passed as `BigInt` — so the authored bundle hashes identically to a native-authored case.

#### Scenario: a JS consumer can wire up the bridge from the guide

- **WHEN** a JS developer follows the guide
- **THEN** it SHALL show the `wasm-pack build` command and example calls to the bridge functions including `packCase`
- **AND** it SHALL present a small adapter that wraps those calls

#### Scenario: a JS author packs a case with the correct object shape

- **WHEN** a JS developer authors a `.case` through `packCase` following the guide
- **THEN** the guide SHALL show a case object whose manifest entries use the key `type` and whose `report_mode` is lowercase
- **AND** it SHALL warn that integer record values beyond the JavaScript safe-integer range MUST be passed as `BigInt` to keep the bundle hash identical to the native producer

<!-- @trace
source: user-integration-guide
updated: 2026-06-21
code:
  - README.md
  - docs/fcb-integration-guide.md
  - docs/README.md
-->
