//! `.casework` — the student submission bundle.
//!
//! Same container family as `.case` (KIND=work), but the payload holds the
//! student's work: notes, report, activity log, evidence references, plus the
//! `case_id`/`bundle_hash` that bind the work to a specific challenge and
//! evidence version (see `crate::binding`).

use ciborium::value::Value;
use serde::{Deserialize, Serialize};

use crate::bundle::{self, BundleParams};
use crate::cbor;
use crate::container::BundleKind;
use crate::error::{FcbError, Result};

/// Minimal student identity carried in a submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Student {
    pub id: String,
    pub name: String,
}

/// The student's work product. Record-level shapes (notes/report/activity) stay
/// opaque CBOR here; the workbench owns their schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Submission {
    /// Challenge this work belongs to.
    pub case_id: String,
    /// Evidence version this work was produced against.
    pub bundle_hash: String,
    pub student: Student,
    /// Annotations and evidence references.
    pub notes: Vec<Value>,
    /// Step answers or freeform report body.
    pub report: Value,
    /// Investigation action log.
    pub activity: Vec<Value>,
    /// ISO-8601 export timestamp (stamped by the caller).
    pub exported_at: String,
}

/// Pack a submission into a sealed `.casework` bundle (KIND=work).
pub fn pack_submission(work: &Submission, passphrase: &str) -> Result<Vec<u8>> {
    let payload = cbor::encode(work)?;
    let params = BundleParams::new(
        BundleKind::Work,
        work.case_id.clone(),
        work.bundle_hash.clone(),
        Value::Map(vec![]),
    );
    bundle::pack_bytes(&params, &payload, passphrase)
}

/// Open a `.casework` bundle. Rejects a bundle whose KIND is not `work`.
pub fn open_submission(bytes: &[u8], passphrase: &str) -> Result<Submission> {
    let (kind, _header, payload) = bundle::open_bytes(bytes, passphrase)?;
    if kind != BundleKind::Work {
        return Err(FcbError::Malformed("not a .casework (KIND != work)".into()));
    }
    cbor::decode(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Submission {
        Submission {
            case_id: "acme-ir-2026-03".into(),
            bundle_hash: "sha256:9f2c".into(),
            student: Student {
                id: "s1234567".into(),
                name: "Lin".into(),
            },
            notes: vec![Value::Text("pinned auth.log line 42".into())],
            report: Value::Text("freeform report body".into()),
            activity: vec![Value::Text("search: failed login".into())],
            exported_at: "2026-06-20T10:00:00Z".into(),
        }
    }

    #[test]
    fn submission_round_trips() {
        let work = sample();
        let bytes = pack_submission(&work, "passw0rd").unwrap();
        let back = open_submission(&bytes, "passw0rd").unwrap();
        assert_eq!(back, work);
    }

    #[test]
    fn opening_a_case_as_submission_is_rejected() {
        // Pack a KIND=case bundle, then try to read it as a submission.
        let params = BundleParams::new(BundleKind::Case, "c", "h", Value::Map(vec![]));
        let mut params = params;
        params.m_cost = 32;
        params.t_cost = 1;
        params.p_cost = 1;
        let case_bytes = bundle::pack_bytes(&params, b"evidence", "passw0rd").unwrap();
        assert!(matches!(
            open_submission(&case_bytes, "passw0rd"),
            Err(FcbError::Malformed(_))
        ));
    }
}
