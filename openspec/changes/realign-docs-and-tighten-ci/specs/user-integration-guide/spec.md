## ADDED Requirements

### Requirement: Integration guide documents Rust-side content-address verification and the pack invariant

The integration guide SHALL state that the `.case` content-address verification — recomputing `bundle_hash` from the decrypted payload — is performed by the WASM bridge but NOT by the core crate's plain open path, and SHALL direct Rust consumers to recompute the canonical bundle hash with `case_bundle_hash` and compare it against the header value. It SHALL also document that `pack_case` rejects, as a malformed error, any case whose stream manifest does not match the payload by stream-id set and per-stream record count.

#### Scenario: a Rust consumer verifies content addressing

- **WHEN** a Rust developer opens a `.case` through the core crate following the guide
- **THEN** the guide SHALL instruct them to recompute the canonical bundle hash with `case_bundle_hash` and compare it to the header `bundle_hash`, because the core open path does not do so automatically

#### Scenario: an author learns the manifest-must-match-payload rule

- **WHEN** a developer authors a `.case` whose manifest declares a stream-id set or record counts that differ from the payload
- **THEN** the guide SHALL document that `pack_case` rejects it as a malformed error

### Requirement: Integration guide documents kind-less bridge deserialization failures

The integration guide SHALL document that a malformed JS input to a bridge pack call (for example a manifest entry using the wrong field key) fails during deserialization and surfaces as a thrown error WITHOUT a stable error kind, distinct from a post-deserialization `FcbError` that carries a kind such as `malformed`. The error-kind guidance SHALL reflect this distinction so consumers do not expect a kind on deserialization failures.

#### Scenario: a wrong field key throws without an error kind

- **WHEN** a JS consumer calls a bridge pack function with a manifest entry that uses the wrong field key
- **THEN** the guide SHALL explain that the call throws a kind-less error during deserialization, so an error-kind lookup yields the consumer's unknown fallback rather than `malformed`
