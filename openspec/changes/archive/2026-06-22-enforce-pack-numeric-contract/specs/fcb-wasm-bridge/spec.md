## ADDED Requirements

### Requirement: Deterministic numeric encoding at the pack boundary

The WASM bridge pack operations — packing a `.case` and packing a `.casework` submission — SHALL enforce that record values produce a deterministic CBOR encoding matching the native producer. An integer-valued number supplied as a plain JavaScript number whose magnitude lies outside the JavaScript safe-integer range SHALL be rejected with a discriminable error whose `kind` indicates a malformed condition, rather than silently producing a divergent canonical payload or bundle_hash. Integer values within the safe-integer range and genuine non-integer floating-point values SHALL be accepted and SHALL encode deterministically. Integer values outside the safe-integer range supplied as a `BigInt` SHALL be accepted and SHALL encode as CBOR integers without loss. This requirement SHALL apply equally to the case payload records and to the submission record fields.

#### Scenario: Safe-range integer encodes identically to native

- **WHEN** a case or submission is packed through the bridge whose integer record values are within the JavaScript safe-integer range
- **THEN** its canonical payload bytes and bundle_hash equal those the native producer yields for the same logical records

#### Scenario: Genuine float is accepted and deterministic

- **WHEN** a record value is a non-integer floating-point number such as 3.14
- **THEN** the pack operation SHALL accept it and the resulting canonical payload SHALL be byte-stable across repeated packs of the same input

#### Scenario: Out-of-range integer as a plain number is rejected

- **WHEN** a record value is an integer-valued plain JavaScript number whose magnitude is outside the JavaScript safe-integer range
- **THEN** the pack operation SHALL raise an error whose kind indicates a malformed condition and SHALL NOT return bytes

#### Scenario: Out-of-range integer as BigInt is preserved

- **WHEN** an out-of-safe-range integer record value is supplied as a BigInt
- **THEN** the pack operation SHALL encode it as a CBOR integer without loss and the native reader SHALL decode the same integer value

##### Example: numeric boundary matrix

| Record value (as authored) | Pack result | Rationale |
| -------------------------- | ----------- | --------- |
| `7` (plain number) | accepted, CBOR integer, hash matches native | safe-range integer |
| `9007199254740991` = 2^53-1 (plain number) | accepted, CBOR integer | largest safe integer |
| `3.14` (plain number) | accepted, CBOR float, byte-stable | genuine non-integer float |
| `9007199254740992` = 2^53 (plain number) | rejected, malformed | integer-valued, outside safe range |
| `9007199254740993n` = (2^53+1) as BigInt | accepted, CBOR integer, lossless | explicit BigInt for out-of-range integer |
