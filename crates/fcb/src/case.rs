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
#[derive(Debug, Clone)]
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
}
