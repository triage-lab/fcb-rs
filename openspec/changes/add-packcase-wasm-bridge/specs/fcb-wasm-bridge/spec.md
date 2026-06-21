## ADDED Requirements

### Requirement: Case Authoring (Pack Case)

The WASM bridge SHALL expose an operation that packs a sealed `.case` bundle from a case object — a `case_id`, a non-empty stream manifest, an optional task spec, and a payload of streams whose records are CBOR-representable values — together with a passphrase, returning the bundle bytes. The packed bytes SHALL be byte-identical to those the native producer yields for the same logical input. The operation SHALL reject a case whose manifest contains no streams with an error whose kind indicates a malformed condition. Each manifest entry SHALL carry its stream type under the key `type`, and the task report mode SHALL be encoded in lowercase (`steps` or `freeform`). For integer record values within the JavaScript safe-integer range, the canonical payload and the resulting `bundle_hash` SHALL NOT depend on the JavaScript number representation; integer values outside that range MUST be supplied as `BigInt` to preserve their integer encoding.

#### Scenario: Pack and reopen a case

- **WHEN** a caller passes a valid case object and the correct passphrase to the pack operation, then passes the returned bytes and the same passphrase to the open operation
- **THEN** the open operation returns the same `case_id`, the same streams and records (including the lossless `raw` field of `fcb.syslog.v1` records), and the same task spec
- **AND** the `bundle_hash` is stable across repeated packs of the same input

#### Scenario: JS-authored case hashes identically to native

- **WHEN** a case is authored through the bridge whose integer record values are within the JavaScript safe-integer range
- **THEN** its canonical payload bytes and `bundle_hash` equal those produced by the native producer for the same logical records

##### Example: integer record preserves the frozen bundle hash

- **GIVEN** a payload with stream `s0` records `["evt1","evt2"]` and stream `s1` records `[7]`, where `7` is supplied as a plain JS number
- **WHEN** the case is packed through the bridge and reopened
- **THEN** the reported `bundle_hash` is `sha256:376d586b42b0e800a6e78fea8bfb9a68cb569d033cc324b7b9b1800fc508eccf`, identical to the native producer's frozen case bundle hash

#### Scenario: Reject a case with no streams

- **WHEN** the pack operation receives a case whose manifest contains no streams
- **THEN** the bridge raises an error whose kind indicates a malformed condition rather than returning bytes
