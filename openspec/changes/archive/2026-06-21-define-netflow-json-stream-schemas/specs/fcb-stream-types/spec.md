## ADDED Requirements

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
