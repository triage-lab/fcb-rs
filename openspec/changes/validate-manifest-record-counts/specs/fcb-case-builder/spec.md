## ADDED Requirements

### Requirement: Pack rejects manifest inconsistent with payload

`pack_case` SHALL reject, with a Malformed error and without producing a bundle, any case whose stream manifest is inconsistent with the payload. The set of stream ids declared by the manifest SHALL equal the set of stream ids present in the payload — with no missing id, no extra id, and no duplicate id on either side — and for each stream id the number of records carried by the payload SHALL equal the record count declared by that stream's manifest entry. For a case whose manifest and payload are consistent, `pack_case` SHALL produce byte-for-byte identical output to packing without this requirement; this requirement adds rejection of inconsistent cases only and changes nothing for consistent ones.

#### Scenario: Declared record count differs from payload

- **WHEN** a caller packs a case whose manifest declares a record count for a stream that differs from the number of records the payload carries for that stream
- **THEN** `pack_case` SHALL return a Malformed error and SHALL NOT produce a bundle

#### Scenario: Stream id set differs between manifest and payload

- **WHEN** a caller packs a case whose payload carries a stream id not declared in the manifest, or omits a stream id the manifest declares, or repeats a stream id on either side
- **THEN** `pack_case` SHALL return a Malformed error

#### Scenario: Consistent case packs unchanged

- **WHEN** a caller packs a case whose manifest stream id set and per-stream record counts all agree with the payload
- **THEN** `pack_case` SHALL succeed and produce byte-for-byte the same bundle as before this requirement
