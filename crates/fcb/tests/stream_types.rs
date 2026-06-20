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
use fcb::container::BundleKind;
use fcb::evidence::{decode_streams, manifest_to_meta, StreamData, StreamManifest};
use fcb::cbor;
use serde::{Deserialize, Serialize};

/// Mirror of vectors.rs: the encrypted payload is just the stream records.
#[derive(Serialize, Deserialize)]
struct CasePayload {
    streams: Vec<StreamData>,
}

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
                "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8".into(),
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

/// Pack one `fcb.syslog.v1` stream through the public codec and return the
/// decoded payload + the manifest used. Low Argon2 cost keeps it fast.
fn round_trip(records: Vec<Value>) -> (CasePayload, StreamManifest) {
    let count = records.len() as u64;
    let manifest = StreamManifest {
        id: "s0".into(),
        stream_type: "fcb.syslog.v1".into(),
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
    let (decoded, manifest) = round_trip(originals.clone());

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
    let (decoded, manifest) = round_trip(originals.clone());

    assert_eq!(
        decoded.streams[0].records, originals,
        "minimal fcb.syslog.v1 record (ts/host/msg only) drifted under round-trip"
    );

    let streams = decode_streams(&manifest_vec(manifest), &decoded.streams).unwrap();
    assert_eq!(streams[0].stream_type, "fcb.syslog.v1");
    assert!(streams[0].is_builtin);
}

/// Wrap a single manifest entry into the slice `decode_streams` expects.
fn manifest_vec(m: StreamManifest) -> Vec<StreamManifest> {
    vec![m]
}
