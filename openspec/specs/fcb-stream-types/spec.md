# fcb-stream-types Specification

## Purpose

TBD - created by archiving change 'fcb-stream-types'. Update Purpose after archive.

## Requirements

### Requirement: fcb.syslog.v1 record schema

The `fcb.syslog.v1` stream type SHALL represent each record as a CBOR map. Each record MUST contain the REQUIRED fields `ts`, `host`, and `msg`. The fields `raw`, `app`, `pid`, `severity`, `facility`, `msgid`, `sd`, and `format` are OPTIONAL. Field types and semantics SHALL be:

- `ts` — text. An RFC 3339 timestamp normalized to UTC with a trailing `Z`, millisecond precision. It represents the originator-reported event time.
- `host` — text. The source host (hostname, FQDN, or IP) as captured.
- `msg` — text. The parsed, human-readable message body.
- `raw` — text. The original unparsed log line exactly as captured (see "Raw line is the authoritative source").
- `app` — text. The originating application or program (RFC 5424 APP-NAME, RFC 3164 TAG, or equivalent).
- `pid` — unsigned integer. The originating process identifier.
- `severity` — unsigned integer in the range 0 to 7 (0 = Emergency, 7 = Debug).
- `facility` — unsigned integer in the range 0 to 23.
- `msgid` — text. The message-type identifier (RFC 5424 MSGID).
- `sd` — CBOR map keyed by SD-ID; each value is a CBOR map of parameter name to string value (RFC 5424 STRUCTURED-DATA, grouped per element).
- `format` — text, one of `rfc3164`, `rfc5424`, or `other`. The source wire format the record was derived from.

`severity` and `facility` SHALL be stored as numeric codes; human-readable names SHALL NOT be stored and SHALL be derived by the consumer from the numeric code. The producer SHALL normalize `ts` to UTC; when the source format lacks a year or timezone, the producer SHALL infer them and SHALL preserve the original line in `raw`.

#### Scenario: RFC 5424 source record

- **WHEN** the encoder packages a record captured from an RFC 5424 source
- **THEN** the record SHALL contain `ts`, `host`, `msg`, the numeric `severity` and `facility`, `format` set to `rfc5424`, and `raw` holding the original line
- **AND** structured data SHALL appear under `sd` grouped by SD-ID

##### Example: sshd auth failure (RFC 5424)

- **GIVEN** the line `<34>1 2026-03-14T08:21:33.512Z mymachine.example.com su - ID47 [ex@32473 iut="3"] 'su root' failed`
- **WHEN** it is encoded as an `fcb.syslog.v1` record
- **THEN** the record SHALL equal:

| field | value |
| ----- | ----- |
| ts | `2026-03-14T08:21:33.512Z` |
| host | `mymachine.example.com` |
| app | `su` |
| msgid | `ID47` |
| severity | `2` |
| facility | `4` |
| sd | `{"ex@32473":{"iut":"3"}}` |
| format | `rfc5424` |
| raw | the original line, verbatim |

#### Scenario: RFC 3164 source record with inferred year and timezone

- **WHEN** the encoder packages a record captured from an RFC 3164 source whose timestamp has no year or timezone
- **THEN** the producer SHALL infer the year and timezone to produce a UTC `ts`, set `format` to `rfc3164`, and store the original line verbatim in `raw`

##### Example: legacy su failure (RFC 3164)

- **GIVEN** the line `<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8` captured in year 2026 from a UTC source
- **WHEN** it is encoded as an `fcb.syslog.v1` record
- **THEN** `ts` SHALL be `2026-10-11T22:14:15Z`, `host` SHALL be `mymachine`, `app` SHALL be `su`, `severity` SHALL be `2`, `facility` SHALL be `4`, `format` SHALL be `rfc3164`, and `raw` SHALL hold the original line

#### Scenario: minimal record

- **WHEN** only the required fields are available
- **THEN** a record containing exactly `ts`, `host`, and `msg` SHALL be a valid `fcb.syslog.v1` record


<!-- @trace
source: fcb-stream-types
updated: 2026-06-20
code:
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - .spectra.yaml
  - docs/README.md
tests:
  - crates/fcb/tests/stream_types.rs
-->

---
### Requirement: Raw line is the authoritative source

When the `raw` field is present, it SHALL hold the original log line exactly as captured and SHALL be the lossless source of truth for the record. All parsed fields SHALL be treated as best-effort derivations and SHALL NOT be the sole representation of the event; a consumer SHALL be able to re-derive parsed fields from `raw`.

#### Scenario: re-deriving fields from raw

- **WHEN** a consumer encounters a parsed field it does not support or trust
- **THEN** the consumer SHALL be able to recover the information by re-parsing `raw`
- **AND** no information present on the original line SHALL be lost while `raw` is retained


<!-- @trace
source: fcb-stream-types
updated: 2026-06-20
code:
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - .spectra.yaml
  - docs/README.md
tests:
  - crates/fcb/tests/stream_types.rs
-->

---
### Requirement: Additive schema evolution within a type version

A typed stream record schema SHALL evolve additively within a given type version. New fields SHALL be introduced as OPTIONAL only. A consumer SHALL ignore record fields it does not recognize and SHALL NOT fail when an OPTIONAL field is absent. A producer that lacks a value for an OPTIONAL field SHALL omit that field.

#### Scenario: forward compatibility with an added field

- **WHEN** a producer adds a new OPTIONAL field to an `fcb.syslog.v1` record and a consumer that predates the field reads that record
- **THEN** the consumer SHALL ignore the unrecognized field and SHALL process the remaining fields normally

#### Scenario: backward compatibility with a missing field

- **WHEN** a consumer that recognizes a newer OPTIONAL field reads a record that omits it
- **THEN** the consumer SHALL treat the field as absent and SHALL NOT fail


<!-- @trace
source: fcb-stream-types
updated: 2026-06-20
code:
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - .spectra.yaml
  - docs/README.md
tests:
  - crates/fcb/tests/stream_types.rs
-->

---
### Requirement: Breaking changes bump the stream type version

A change that alters the type or meaning of an existing field, or removes a REQUIRED field, SHALL be published as a new stream type version (for example `fcb.syslog.v2`) rather than altered in place within an existing version. A reader that has no handler for a given stream type or version SHALL fall back to the generic table or timeline view and SHALL NOT treat the unknown type or version as fatal, consistent with the fcb-evidence-model unknown-type behavior.

#### Scenario: reader without a handler for a newer version

- **WHEN** a reader without an `fcb.syslog.v2` handler opens a bundle containing an `fcb.syslog.v2` stream
- **THEN** the reader SHALL surface the stream through the generic table or timeline fallback
- **AND** the reader SHALL NOT abort or error on the unknown version

<!-- @trace
source: fcb-stream-types
updated: 2026-06-20
code:
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - .spectra.yaml
  - docs/README.md
tests:
  - crates/fcb/tests/stream_types.rs
-->