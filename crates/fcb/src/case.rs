//! `.case` builder — the authoritative producer interface for evidence bundles.
//!
//! The teacher/authoring side of the codec, mirroring
//! [`crate::submission::pack_submission`] on the student side. A case carries N
//! typed evidence streams plus an optional embedded (answer-free) task. This
//! module owns the single canonical serialization of the `{ streams }` payload
//! envelope and freezes the canonical `bundle_hash` as the SHA-256 of those
//! plaintext payload bytes — so producers and consumers agree on one byte
//! sequence and one content address, independent of the random salt/nonce a
//! sealed bundle uses.

use serde::{Deserialize, Serialize};

use crate::binding;
use crate::bundle::{self, BundleParams};
use crate::cbor;
use crate::container::BundleKind;
use crate::error::{FcbError, Result};
use crate::evidence::{StreamData, StreamManifest};
use crate::task::TaskSpec;

/// The `.case` plaintext payload envelope: the typed stream records carried in
/// the encrypted body. This is the single authoritative envelope shared by the
/// codec, its tests, and the WASM bridge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CasePayload {
    #[serde(default)]
    pub streams: Vec<StreamData>,
}

impl CasePayload {
    /// The canonical plaintext bytes of this payload — the single serialization
    /// entry point both producers and consumers MUST use. Equivalent to a CBOR
    /// encoding of the `{ streams }` envelope.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>> {
        cbor::encode(self)
    }
}

/// The canonical `bundle_hash` for a case: `sha256:<hex>` over the canonical
/// plaintext payload bytes. Independent of the random salt/nonce a sealed
/// bundle carries, so the same evidence always yields the same content address.
pub fn case_bundle_hash(payload: &CasePayload) -> Result<String> {
    Ok(binding::compute_bundle_hash(&payload.to_canonical_bytes()?))
}

/// Inputs for building a `.case` bundle. The manifest declares each stream's
/// namespaced `type` and record count (which [`StreamData`] does not carry);
/// the payload carries the records. The optional task embeds the answer-free
/// assignment in the plaintext header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseInput {
    pub case_id: String,
    pub manifest: Vec<StreamManifest>,
    pub task: Option<TaskSpec>,
    pub payload: CasePayload,
}

/// The plaintext header `meta` map for a case: `{ streams, task? }`. Readable
/// without a passphrase by [`crate::evidence::manifest_from_meta`] and
/// [`crate::task::task_from_meta`]. `task` is omitted when absent.
#[derive(Serialize)]
struct CaseMeta<'a> {
    streams: &'a [StreamManifest],
    #[serde(skip_serializing_if = "Option::is_none")]
    task: Option<&'a TaskSpec>,
}

/// Producer invariant: the manifest's declared stream id set and per-stream
/// record counts MUST agree with the payload exactly. A producer that seals a
/// `.case` whose manifest disagrees with the payload would mint a bundle whose
/// inconsistency only surfaces (partially) at open time, so reject it loudly at
/// pack time instead. A single pass over the payload builds an id -> count map
/// (rejecting a repeated payload id), then each manifest entry is consumed from
/// it — which also catches a missing payload stream and a duplicate manifest id
/// — and any unconsumed payload stream is one the manifest never declared.
fn check_manifest_matches_payload(
    manifest: &[StreamManifest],
    payload: &CasePayload,
) -> Result<()> {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::with_capacity(payload.streams.len());
    for stream in &payload.streams {
        if counts
            .insert(stream.id.as_str(), stream.records.len())
            .is_some()
        {
            return Err(FcbError::Malformed(format!(
                "payload declares stream id {:?} more than once",
                stream.id
            )));
        }
    }
    for entry in manifest {
        match counts.remove(entry.id.as_str()) {
            None => {
                return Err(FcbError::Malformed(format!(
                    "manifest declares stream {:?} with no matching payload stream",
                    entry.id
                )))
            }
            Some(actual) if actual as u64 != entry.records => {
                return Err(FcbError::Malformed(format!(
                    "manifest declares {} records for stream {:?} but payload carries {}",
                    entry.records, entry.id, actual
                )))
            }
            Some(_) => {}
        }
    }
    if let Some(extra) = counts.keys().next() {
        return Err(FcbError::Malformed(format!(
            "payload carries stream {:?} not declared in the manifest",
            extra
        )));
    }
    Ok(())
}

/// Pack an evidence case into a sealed `.case` bundle (KIND=case).
///
/// The header `bundle_hash` is set to the canonical hash of the payload, the
/// manifest and any task go into the plaintext header meta, and the canonical
/// payload bytes form the encrypted body. Uses the default Argon2id cost, like
/// [`crate::submission::pack_submission`].
pub fn pack_case(input: &CaseInput, passphrase: &str) -> Result<Vec<u8>> {
    // A case with no declared streams carries no evidence — reject it loudly
    // rather than seal an empty `.case` that fails only when a student opens it.
    if input.manifest.is_empty() {
        return Err(FcbError::Malformed("case has no streams".into()));
    }
    // The manifest (in the plaintext header) and the payload streams must agree
    // on the stream id set and per-stream record counts before we seal anything.
    check_manifest_matches_payload(&input.manifest, &input.payload)?;
    let payload_bytes = input.payload.to_canonical_bytes()?;
    let bundle_hash = binding::compute_bundle_hash(&payload_bytes);
    let meta = cbor::to_value(&CaseMeta {
        streams: &input.manifest,
        task: input.task.as_ref(),
    })?;
    let params = BundleParams::new(BundleKind::Case, input.case_id.clone(), bundle_hash, meta);
    bundle::pack_bytes(&params, &payload_bytes, passphrase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value;

    fn payload() -> CasePayload {
        CasePayload {
            streams: vec![
                StreamData {
                    id: "s0".into(),
                    records: vec![Value::Text("evt1".into())],
                },
                StreamData {
                    id: "s1".into(),
                    records: vec![Value::Integer(7.into())],
                },
            ],
        }
    }

    #[test]
    fn canonical_bytes_round_trip() {
        let p = payload();
        let bytes = p.to_canonical_bytes().unwrap();
        // The single serialization entry point decodes back to the same payload.
        let back: CasePayload = cbor::decode(&bytes).unwrap();
        assert_eq!(back, p);
        // ...and is deterministic across calls.
        assert_eq!(p.to_canonical_bytes().unwrap(), bytes);
    }

    #[test]
    fn bundle_hash_is_content_addressed() {
        let h = case_bundle_hash(&payload()).unwrap();
        assert!(h.starts_with("sha256:"));
        assert_eq!(h.len(), 7 + 64);
        // Same content -> same hash; it equals hashing the canonical bytes.
        assert_eq!(h, case_bundle_hash(&payload()).unwrap());
        assert_eq!(
            h,
            binding::compute_bundle_hash(&payload().to_canonical_bytes().unwrap())
        );
        // Different content -> different hash.
        let other = CasePayload { streams: vec![] };
        assert_ne!(h, case_bundle_hash(&other).unwrap());
    }

    #[test]
    fn pack_case_rejects_empty_manifest() {
        // An evidence-free case is rejected at production time (before any KDF).
        let input = CaseInput {
            case_id: "c".into(),
            manifest: vec![],
            task: None,
            payload: CasePayload { streams: vec![] },
        };
        assert!(matches!(
            pack_case(&input, "pw"),
            Err(FcbError::Malformed(_))
        ));
    }

    fn manifest(entries: &[(&str, u64)]) -> Vec<StreamManifest> {
        entries
            .iter()
            .map(|(id, n)| StreamManifest {
                id: (*id).into(),
                stream_type: "fcb.json.v1".into(),
                records: *n,
            })
            .collect()
    }

    fn case_with(manifest: Vec<StreamManifest>, payload: CasePayload) -> CaseInput {
        CaseInput {
            case_id: "c".into(),
            manifest,
            task: None,
            payload,
        }
    }

    #[test]
    fn pack_case_accepts_consistent_manifest() {
        // payload() carries s0 (1 record) and s1 (1 record); a manifest that
        // declares exactly those ids and counts packs fine.
        let input = case_with(manifest(&[("s0", 1), ("s1", 1)]), payload());
        assert!(pack_case(&input, "pw").is_ok());
    }

    #[test]
    fn pack_case_rejects_record_count_mismatch() {
        // Manifest claims s0 has 2 records, but the payload carries 1.
        let input = case_with(manifest(&[("s0", 2), ("s1", 1)]), payload());
        assert!(matches!(
            pack_case(&input, "pw"),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn pack_case_rejects_extra_payload_stream() {
        // Manifest declares only s0; payload carries s0 and s1.
        let input = case_with(manifest(&[("s0", 1)]), payload());
        assert!(matches!(
            pack_case(&input, "pw"),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn pack_case_rejects_missing_payload_stream() {
        // Manifest declares s0, s1, s2; payload only carries s0 and s1.
        let input = case_with(manifest(&[("s0", 1), ("s1", 1), ("s2", 1)]), payload());
        assert!(matches!(
            pack_case(&input, "pw"),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn pack_case_rejects_duplicate_payload_id() {
        let dup = CasePayload {
            streams: vec![
                StreamData {
                    id: "s0".into(),
                    records: vec![Value::Text("a".into())],
                },
                StreamData {
                    id: "s0".into(),
                    records: vec![Value::Text("b".into())],
                },
            ],
        };
        let input = case_with(manifest(&[("s0", 1)]), dup);
        assert!(matches!(
            pack_case(&input, "pw"),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn pack_case_rejects_duplicate_manifest_id() {
        // Manifest repeats s0; payload has a single s0.
        let single = CasePayload {
            streams: vec![StreamData {
                id: "s0".into(),
                records: vec![Value::Text("a".into())],
            }],
        };
        let input = case_with(manifest(&[("s0", 1), ("s0", 1)]), single);
        assert!(matches!(
            pack_case(&input, "pw"),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn pack_case_accepts_zero_record_stream() {
        // A stream declared with 0 records and an empty payload records vec is
        // internally consistent, so the invariant accepts it (whether an empty
        // stream is meaningful is a separate concern, not this check's job).
        let p = CasePayload {
            streams: vec![StreamData {
                id: "s0".into(),
                records: vec![],
            }],
        };
        let input = case_with(manifest(&[("s0", 0)]), p);
        assert!(pack_case(&input, "pw").is_ok());
    }

    #[test]
    fn pack_case_rejects_mismatch_in_non_first_stream() {
        // The check is position-independent: a count mismatch in the LAST
        // declared stream is caught just like one in the first.
        let p = CasePayload {
            streams: vec![
                StreamData {
                    id: "s0".into(),
                    records: vec![Value::Text("a".into())],
                },
                StreamData {
                    id: "s1".into(),
                    records: vec![Value::Text("b".into())],
                },
                StreamData {
                    id: "s2".into(),
                    records: vec![Value::Text("c".into())],
                },
            ],
        };
        // Manifest declares s2 has 2 records, but the payload carries 1.
        let input = case_with(manifest(&[("s0", 1), ("s1", 1), ("s2", 2)]), p);
        assert!(matches!(
            pack_case(&input, "pw"),
            Err(FcbError::Malformed(_))
        ));
    }
}
