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

---
### Requirement: fcb.netflow.v1 record schema

The `fcb.netflow.v1` stream type SHALL represent each record as a CBOR map describing one network flow. Each record MUST contain the REQUIRED fields `ts_start`, `ts_end`, `src_ip`, `dst_ip`, `src_port`, `dst_port`, `proto`, `bytes`, and `packets`. The fields `tcp_flags` and `app` are OPTIONAL. Field types and semantics SHALL be:

- `ts_start` — text. An RFC 3339 timestamp normalized to UTC with a trailing `Z`, marking the first observed packet of the flow.
- `ts_end` — text. An RFC 3339 timestamp normalized to UTC with a trailing `Z`, marking the last observed packet of the flow. `ts_end` SHALL be greater than or equal to `ts_start`.
- `src_ip` — text. The source address (IPv4 or IPv6) as captured.
- `dst_ip` — text. The destination address (IPv4 or IPv6) as captured.
- `src_port` — unsigned integer in the range 0 to 65535. For transport protocols without ports, the producer SHALL use `0`.
- `dst_port` — unsigned integer in the range 0 to 65535. For transport protocols without ports, the producer SHALL use `0`.
- `proto` — unsigned integer. The IANA transport protocol number (for example 6 = TCP, 17 = UDP, 1 = ICMP).
- `bytes` — unsigned integer. The total number of bytes observed in the flow.
- `packets` — unsigned integer. The total number of packets observed in the flow.
- `tcp_flags` — unsigned integer. The cumulative bitwise-OR of TCP flags observed across the flow (for example `0x02` = SYN). Present only for TCP flows.
- `app` — text. An optional application or layer-7 protocol label (for example `tls`, `dns`).

The range constraints on `src_port`, `dst_port`, and the IANA meaning of `proto` are specification-level constraints; the codec SHALL NOT enforce them (it accepts any unsigned integer), consistent with the `fcb.syslog.v1` treatment of `severity` and `facility`.

#### Scenario: TCP flow record

- **WHEN** the encoder packages a captured TCP flow with its byte and packet counts
- **THEN** the record SHALL contain the nine REQUIRED fields with `proto` set to `6`
- **AND** `tcp_flags` SHALL carry the cumulative TCP flags when available

##### Example: HTTPS flow with cumulative flags

- **GIVEN** a TCP flow from `10.0.0.5:49512` to `203.0.113.10:443` observed from `2026-03-14T08:20:00.000Z` to `2026-03-14T08:20:03.500Z`, carrying 18452 bytes in 24 packets with cumulative flags `0x1a`
- **WHEN** it is encoded as an `fcb.netflow.v1` record
- **THEN** the record SHALL equal:

| field | value |
| ----- | ----- |
| ts_start | `2026-03-14T08:20:00.000Z` |
| ts_end | `2026-03-14T08:20:03.500Z` |
| src_ip | `10.0.0.5` |
| dst_ip | `203.0.113.10` |
| src_port | `49512` |
| dst_port | `443` |
| proto | `6` |
| bytes | `18452` |
| packets | `24` |
| tcp_flags | `26` |
| app | `tls` |

#### Scenario: UDP flow without optional fields

- **WHEN** the encoder packages a UDP flow and no TCP flags or application label apply
- **THEN** a record containing exactly the nine REQUIRED fields with `proto` set to `17` SHALL be a valid `fcb.netflow.v1` record
- **AND** `tcp_flags` and `app` SHALL be omitted

##### Example: DNS query flow (UDP, minimal)

- **GIVEN** a UDP flow from `10.0.0.5:53124` to `10.0.0.1:53` observed from `2026-03-14T08:19:58.000Z` to `2026-03-14T08:19:58.040Z`, carrying 168 bytes in 2 packets
- **WHEN** it is encoded as an `fcb.netflow.v1` record
- **THEN** the record SHALL contain only `ts_start`, `ts_end`, `src_ip`, `dst_ip`, `src_port` (`53124`), `dst_port` (`53`), `proto` (`17`), `bytes` (`168`), and `packets` (`2`)


<!-- @trace
source: define-netflow-json-stream-schemas
updated: 2026-06-21
code:
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - docs/README.md
  - docs/fcb-reference.md
tests:
  - crates/fcb/tests/stream_types.rs
-->

---
### Requirement: fcb.json.v1 record schema

The `fcb.json.v1` stream type SHALL represent each record as an arbitrary CBOR map — a universal object container for data that has no dedicated stream type. The schema SHALL NOT require any specific keys. Map keys SHALL be text and each value SHALL be permitted to be any CBOR value (text, integer, float, boolean, null, array, or nested map). A consumer SHALL preserve every key and value byte-faithfully and SHALL NOT drop, reorder, or coerce unrecognized content.

#### Scenario: arbitrary object round-trips byte-faithfully

- **WHEN** a record that is an arbitrary CBOR map (including nested maps and arrays) is encoded as an `fcb.json.v1` record and later decoded
- **THEN** the decoded record SHALL equal the original map exactly, with all keys, values, types, and nesting preserved

##### Example: nested alert object

- **GIVEN** the object `{ "kind": "alert", "score": 0.91, "tags": ["beacon", "c2"], "meta": { "asn": 64512 } }`
- **WHEN** it is encoded as an `fcb.json.v1` record and decoded
- **THEN** the decoded record SHALL equal that object exactly, preserving the nested `meta` map, the `tags` array, the float `score`, and the integer `asn`

#### Scenario: minimal object

- **WHEN** a record is a single-entry CBOR map such as `{ "k": "v" }`
- **THEN** it SHALL be a valid `fcb.json.v1` record

<!-- @trace
source: define-netflow-json-stream-schemas
updated: 2026-06-21
code:
  - docs/fcb-data-model.md
  - docs/fcb-wire-format.md
  - docs/README.md
  - docs/fcb-reference.md
tests:
  - crates/fcb/tests/stream_types.rs
-->