## ADDED Requirements

### Requirement: Submission has a byte-stable golden vector

The test suite SHALL pin the on-disk bytes of a real seven-field `Submission` (`case_id`, `bundle_hash`, `student`, `notes`, `report`, `activity`, `exported_at`) with a frozen golden vector, built deterministically with a fixed salt and nonce. Rebuilding the vector from the same inputs SHALL reproduce the same bytes exactly, so an accidental change to the submission payload layout fails the test instead of silently breaking cross-implementation interoperability. The golden vector SHALL decode back to a `Submission` whose seven fields equal the inputs.

#### Scenario: rebuilding the submission vector reproduces the frozen bytes

- **WHEN** the frozen submission vector is rebuilt from its fixed inputs (fixed salt, nonce, and the seven-field `Submission`)
- **THEN** the produced bytes SHALL be byte-identical to the frozen vector
- **AND** any change to the submission payload layout SHALL fail this byte-stability test

#### Scenario: the frozen submission vector decodes to its seven fields

- **WHEN** the frozen submission vector is opened as a `.casework` with the correct passphrase
- **THEN** it SHALL decode to a `Submission` whose `case_id`, `bundle_hash`, `student`, `notes`, `report`, `activity`, and `exported_at` equal the inputs that produced the vector
