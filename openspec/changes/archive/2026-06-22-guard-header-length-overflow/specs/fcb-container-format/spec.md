## ADDED Requirements

### Requirement: Overflow-safe length parsing

The container reader SHALL compute every header and payload slice bound that derives from a wire-supplied length or offset field using overflow-safe arithmetic. When such a computation would overflow the platform `usize`, or would address a range outside the input buffer, the reader SHALL reject the container with a Malformed error. The reader SHALL NOT panic on any supported target, including 32-bit targets such as `wasm32`, regardless of the length value supplied. Parsing a well-formed container SHALL produce a byte-for-byte identical result to parsing it without this requirement; this requirement adds rejection of out-of-range inputs only and changes no behavior for valid inputs.

#### Scenario: Header length that overflows the platform usize

- **WHEN** a reader parses a container whose declared header length, added to the header offset, would overflow the platform `usize`
- **THEN** the reader SHALL return a Malformed error and SHALL NOT panic

#### Scenario: Header length beyond the input buffer

- **WHEN** a reader parses a container whose declared header length addresses bytes beyond the end of the input buffer
- **THEN** the reader SHALL return a Malformed error

#### Scenario: Well-formed container is unaffected

- **WHEN** a reader parses a well-formed container
- **THEN** the reader SHALL return the same kind, version, header, and payload as it did before this requirement, with no change to the accepted byte sequence
