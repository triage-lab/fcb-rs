//! Self-describing evidence model.
//!
//! A bundle carries N typed streams. The plaintext header's manifest declares
//! each stream's `id`, namespaced versioned `type`, and record count; the
//! encrypted payload carries the records. Built-in types are not a closed
//! enumeration — any namespaced type is first-class, and an unknown type is
//! never fatal (it is surfaced for the consumer to fall back on).

use ciborium::value::Value;
use serde::{Deserialize, Serialize};

use crate::cbor;
use crate::error::{FcbError, Result};

/// Built-in stream types shipped by this codec. New types do not need to be
/// listed here — this set only distinguishes "has a built-in handler" from
/// "needs a plugin or the generic fallback".
pub const BUILTIN_STREAM_TYPES: &[&str] = &["fcb.syslog.v1", "fcb.netflow.v1", "fcb.json.v1"];

/// Whether a stream type has a built-in handler.
pub fn is_builtin_type(stream_type: &str) -> bool {
    BUILTIN_STREAM_TYPES.contains(&stream_type)
}

/// One entry in the header manifest (`meta.streams[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamManifest {
    pub id: String,
    /// Namespaced, versioned type, e.g. `fcb.syslog.v1`.
    #[serde(rename = "type")]
    pub stream_type: String,
    pub records: u64,
}

/// One stream's records as carried in the (decrypted, decompressed) payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamData {
    pub id: String,
    pub records: Vec<Value>,
}

/// A stream surfaced to the consumer: manifest type + payload records, with a
/// flag indicating whether a built-in handler exists.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedStream {
    pub id: String,
    pub stream_type: String,
    pub records: Vec<Value>,
    /// `false` ⇒ no built-in handler; the consumer falls back to a generic
    /// table/timeline view (or a registered plugin).
    pub is_builtin: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StreamsMeta {
    #[serde(default)]
    streams: Vec<StreamManifest>,
}

/// Encode a stream manifest into the opaque `header.meta` value.
pub fn manifest_to_meta(manifest: &[StreamManifest]) -> Result<Value> {
    cbor::to_value(&StreamsMeta {
        streams: manifest.to_vec(),
    })
}

/// Read the stream manifest from an opaque `header.meta` value. Readable
/// before decryption, since the manifest lives in the plaintext header.
pub fn manifest_from_meta(meta: &Value) -> Result<Vec<StreamManifest>> {
    let m: StreamsMeta = cbor::from_value(meta)?;
    Ok(m.streams)
}

/// Join the manifest (types) with payload records (by `id`). Unknown types are
/// returned, never rejected; a manifest entry with no payload stream is a
/// structural error.
pub fn decode_streams(manifest: &[StreamManifest], payload: &[StreamData]) -> Result<Vec<DecodedStream>> {
    manifest
        .iter()
        .map(|m| {
            let data = payload
                .iter()
                .find(|s| s.id == m.id)
                .ok_or_else(|| FcbError::Malformed(format!("payload missing stream {}", m.id)))?;
            Ok(DecodedStream {
                id: m.id.clone(),
                stream_type: m.stream_type.clone(),
                records: data.records.clone(),
                is_builtin: is_builtin_type(&m.stream_type),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(n: i64) -> Value {
        Value::Integer(n.into())
    }

    fn mixed_manifest() -> Vec<StreamManifest> {
        vec![
            StreamManifest { id: "s0".into(), stream_type: "fcb.syslog.v1".into(), records: 2 },
            StreamManifest { id: "s1".into(), stream_type: "acme.edr.v1".into(), records: 1 },
        ]
    }

    #[test]
    fn manifest_round_trips_through_meta() {
        // The manifest is self-describing and readable from the plaintext meta.
        let manifest = mixed_manifest();
        let meta = manifest_to_meta(&manifest).unwrap();
        assert_eq!(manifest_from_meta(&meta).unwrap(), manifest);
    }

    #[test]
    fn builtin_and_third_party_types_both_listed() {
        let manifest = mixed_manifest();
        let payload = vec![
            StreamData { id: "s0".into(), records: vec![rec(1), rec(2)] },
            StreamData { id: "s1".into(), records: vec![rec(9)] },
        ];
        let decoded = decode_streams(&manifest, &payload).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].stream_type, "fcb.syslog.v1");
        assert!(decoded[0].is_builtin);
        // Third-party namespaced type is a first-class stream...
        assert_eq!(decoded[1].stream_type, "acme.edr.v1");
        // ...just without a built-in handler -> consumer falls back.
        assert!(!decoded[1].is_builtin);
    }

    #[test]
    fn unknown_type_does_not_abort_other_streams() {
        // An unknown type sits between two known ones; all must still decode.
        let manifest = vec![
            StreamManifest { id: "a".into(), stream_type: "fcb.syslog.v1".into(), records: 1 },
            StreamManifest { id: "b".into(), stream_type: "vendor.unknown.v3".into(), records: 1 },
            StreamManifest { id: "c".into(), stream_type: "fcb.json.v1".into(), records: 1 },
        ];
        let payload = vec![
            StreamData { id: "a".into(), records: vec![rec(1)] },
            StreamData { id: "b".into(), records: vec![rec(2)] },
            StreamData { id: "c".into(), records: vec![rec(3)] },
        ];
        let decoded = decode_streams(&manifest, &payload).unwrap();
        assert_eq!(decoded.len(), 3);
        assert!(!decoded[1].is_builtin);
        assert!(decoded[0].is_builtin && decoded[2].is_builtin);
    }
}
