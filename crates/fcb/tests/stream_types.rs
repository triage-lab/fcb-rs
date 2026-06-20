//! Contract test: freeze the `fcb.syslog.v1` record schema.
//!
//! This is a schema-freezing round-trip, not a new codec feature. The codec
//! already round-trips arbitrary CBOR records, so packing three concrete
//! `fcb.syslog.v1` records (the spec's worked examples) and reading them back
//! byte-faithfully proves the schema's field set, key names, and value types
//! survive the full pack/open path. If the schema ever drifts, the
//! `assert_eq!` on the decoded records breaks here instead of silently.

use ciborium::value::Value;
use fcb::bundle::{self, BundleParams};
use fcb::case::CasePayload;
use fcb::cbor;
use fcb::container::BundleKind;
use fcb::evidence::{decode_streams, manifest_to_meta, StreamData, StreamManifest};

/// Helper: build a CBOR map record from string-keyed entries.
fn record(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(k, v)| (Value::Text(k.to_string()), v))
            .collect(),
    )
}

/// The RFC 5424 worked example (sshd auth failure) from the spec.
fn rfc5424_record() -> Value {
    // sd = {"ex@32473":{"iut":"3"}} — nested map keyed by SD-ID.
    let sd = Value::Map(vec![(
        Value::Text("ex@32473".into()),
        Value::Map(vec![(Value::Text("iut".into()), Value::Text("3".into()))]),
    )]);
    record(vec![
        ("ts", Value::Text("2026-03-14T08:21:33.512Z".into())),
        ("host", Value::Text("mymachine.example.com".into())),
        ("app", Value::Text("su".into())),
        ("msgid", Value::Text("ID47".into())),
        ("severity", Value::Integer(2.into())),
        ("facility", Value::Integer(4.into())),
        ("sd", sd),
        ("format", Value::Text("rfc5424".into())),
        ("msg", Value::Text("'su root' failed".into())),
        (
            "raw",
            Value::Text(
                "<34>1 2026-03-14T08:21:33.512Z mymachine.example.com su - ID47 [ex@32473 iut=\"3\"] 'su root' failed"
                    .into(),
            ),
        ),
    ])
}

/// The RFC 3164 worked example (legacy su failure) from the spec.
fn rfc3164_record() -> Value {
    record(vec![
        ("ts", Value::Text("2026-10-11T22:14:15Z".into())),
        ("host", Value::Text("mymachine".into())),
        ("app", Value::Text("su".into())),
        ("severity", Value::Integer(2.into())),
        ("facility", Value::Integer(4.into())),
        ("format", Value::Text("rfc3164".into())),
        (
            "msg",
            Value::Text("'su root' failed for lonvick on /dev/pts/8".into()),
        ),
        (
            "raw",
            Value::Text(
                "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8"
                    .into(),
            ),
        ),
    ])
}

/// The minimal record: only the REQUIRED fields ts/host/msg.
fn minimal_record() -> Value {
    record(vec![
        ("ts", Value::Text("2026-01-01T00:00:00Z".into())),
        ("host", Value::Text("h1".into())),
        ("msg", Value::Text("hello".into())),
    ])
}

/// The `fcb.netflow.v1` worked example: an HTTPS TCP flow with all nine
/// REQUIRED fields plus the OPTIONAL `tcp_flags` and `app`. The values here are
/// valid (port 443, proto 6), but the codec accepts any uint — range/IANA
/// constraints are spec-level, not enforced by the encoder/decoder.
fn netflow_tcp_record() -> Value {
    record(vec![
        ("ts_start", Value::Text("2026-03-14T08:20:00.000Z".into())),
        ("ts_end", Value::Text("2026-03-14T08:20:03.500Z".into())),
        ("src_ip", Value::Text("10.0.0.5".into())),
        ("dst_ip", Value::Text("203.0.113.10".into())),
        ("src_port", Value::Integer(49512.into())),
        ("dst_port", Value::Integer(443.into())),
        ("proto", Value::Integer(6.into())),
        ("bytes", Value::Integer(18452.into())),
        ("packets", Value::Integer(24.into())),
        ("tcp_flags", Value::Integer(26.into())),
        ("app", Value::Text("tls".into())),
    ])
}

/// The `fcb.netflow.v1` minimal example: a UDP (DNS) flow with only the nine
/// REQUIRED fields (no `tcp_flags`/`app`).
fn netflow_udp_record() -> Value {
    record(vec![
        ("ts_start", Value::Text("2026-03-14T08:19:58.000Z".into())),
        ("ts_end", Value::Text("2026-03-14T08:19:58.040Z".into())),
        ("src_ip", Value::Text("10.0.0.5".into())),
        ("dst_ip", Value::Text("10.0.0.1".into())),
        ("src_port", Value::Integer(53124.into())),
        ("dst_port", Value::Integer(53.into())),
        ("proto", Value::Integer(17.into())),
        ("bytes", Value::Integer(168.into())),
        ("packets", Value::Integer(2.into())),
    ])
}

/// The `fcb.json.v1` worked example: an arbitrary nested object with a float,
/// an array, and a nested map.
fn json_nested_record() -> Value {
    record(vec![
        ("kind", Value::Text("alert".into())),
        ("score", Value::Float(0.91)),
        (
            "tags",
            Value::Array(vec![Value::Text("beacon".into()), Value::Text("c2".into())]),
        ),
        (
            "meta",
            Value::Map(vec![(
                Value::Text("asn".into()),
                Value::Integer(64512.into()),
            )]),
        ),
    ])
}

/// The `fcb.json.v1` minimal example: a single-entry map.
fn json_minimal_record() -> Value {
    record(vec![("k", Value::Text("v".into()))])
}

/// Pack one typed stream through the public codec and return the decoded
/// payload + the manifest used. Low Argon2 cost keeps it fast.
fn round_trip(stream_type: &str, records: Vec<Value>) -> (CasePayload, StreamManifest) {
    let count = records.len() as u64;
    let manifest = StreamManifest {
        id: "s0".into(),
        stream_type: stream_type.into(),
        records: count,
    };
    let meta = manifest_to_meta(std::slice::from_ref(&manifest)).unwrap();

    let payload = cbor::encode(&CasePayload {
        streams: vec![StreamData {
            id: "s0".into(),
            records,
        }],
    })
    .unwrap();

    let mut params = BundleParams::new(BundleKind::Case, "case-syslog", "sha256:test", meta);
    params.m_cost = 32;
    params.t_cost = 1;
    params.p_cost = 1;
    let bytes = bundle::pack_bytes(&params, &payload, "pw").unwrap();

    let (kind, _header, opened) = bundle::open_bytes(&bytes, "pw").unwrap();
    assert_eq!(kind, BundleKind::Case);
    let decoded: CasePayload = cbor::decode(&opened).unwrap();
    (decoded, manifest)
}

#[test]
fn syslog_v1_records_round_trip_byte_faithfully() {
    let originals = vec![rfc5424_record(), rfc3164_record(), minimal_record()];
    let (decoded, manifest) = round_trip("fcb.syslog.v1", originals.clone());

    // Every field of every record survives the full pack/open path verbatim.
    assert_eq!(
        decoded.streams[0].records, originals,
        "fcb.syslog.v1 record schema drifted under round-trip"
    );

    // The manifest joins back to a built-in stream of the frozen type.
    let streams = decode_streams(&manifest_vec(manifest), &decoded.streams).unwrap();
    assert_eq!(streams[0].stream_type, "fcb.syslog.v1");
    assert!(streams[0].is_builtin);
}

#[test]
fn syslog_v1_minimal_record_round_trips() {
    let originals = vec![minimal_record()];
    let (decoded, manifest) = round_trip("fcb.syslog.v1", originals.clone());

    assert_eq!(
        decoded.streams[0].records, originals,
        "minimal fcb.syslog.v1 record (ts/host/msg only) drifted under round-trip"
    );

    let streams = decode_streams(&manifest_vec(manifest), &decoded.streams).unwrap();
    assert_eq!(streams[0].stream_type, "fcb.syslog.v1");
    assert!(streams[0].is_builtin);
}

#[test]
fn netflow_v1_records_round_trip_byte_faithfully() {
    let originals = vec![netflow_tcp_record(), netflow_udp_record()];
    let (decoded, manifest) = round_trip("fcb.netflow.v1", originals.clone());

    // The 5-tuple, counts, time range, and optional flags survive verbatim.
    assert_eq!(
        decoded.streams[0].records, originals,
        "fcb.netflow.v1 record schema drifted under round-trip"
    );

    let streams = decode_streams(&manifest_vec(manifest), &decoded.streams).unwrap();
    assert_eq!(streams[0].stream_type, "fcb.netflow.v1");
    assert!(streams[0].is_builtin);
}

#[test]
fn json_v1_records_round_trip_byte_faithfully() {
    let originals = vec![json_nested_record(), json_minimal_record()];
    let (decoded, manifest) = round_trip("fcb.json.v1", originals.clone());

    // Nested maps, arrays, floats, and ints all survive verbatim.
    assert_eq!(
        decoded.streams[0].records, originals,
        "fcb.json.v1 record schema drifted under round-trip"
    );

    let streams = decode_streams(&manifest_vec(manifest), &decoded.streams).unwrap();
    assert_eq!(streams[0].stream_type, "fcb.json.v1");
    assert!(streams[0].is_builtin);
}

/// Wrap a single manifest entry into the slice `decode_streams` expects.
fn manifest_vec(m: StreamManifest) -> Vec<StreamManifest> {
    vec![m]
}
