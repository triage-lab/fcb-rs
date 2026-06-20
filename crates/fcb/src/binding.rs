//! Case identity binding and local work isolation (pure logic).
//!
//! Evidence is re-decrypted every session and never persisted; only the
//! student's *work* is stored, keyed by `case_id` so different cases never
//! mix. `bundle_hash` binds work to a specific evidence version, so reopening a
//! re-issued challenge can be detected. The physical IndexedDB store is out of
//! scope here — this module provides the key derivation and verification logic
//! the store (and the teacher review platform) build on.

use sha2::{Digest, Sha256};

/// Content hash identifying a specific bundle/evidence version.
/// Format: `sha256:<hex>`.
pub fn compute_bundle_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(7 + digest.len() * 2);
    s.push_str("sha256:");
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Result of checking a submission against a challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingCheck {
    /// Same case and same evidence version.
    Match,
    /// Different challenge entirely.
    CaseMismatch,
    /// Same case id, but a different evidence version (re-issued bundle).
    EvidenceVersionMismatch,
}

/// Verify that work bound to (`work_case_id`, `work_bundle_hash`) corresponds to
/// a challenge (`case_id`, `case_bundle_hash`).
pub fn verify_binding(
    work_case_id: &str,
    work_bundle_hash: &str,
    case_id: &str,
    case_bundle_hash: &str,
) -> BindingCheck {
    if work_case_id != case_id {
        BindingCheck::CaseMismatch
    } else if work_bundle_hash != case_bundle_hash {
        BindingCheck::EvidenceVersionMismatch
    } else {
        BindingCheck::Match
    }
}

/// Local-storage partition key for a case's work. Distinct `case_id`s yield
/// distinct keys, so work for different cases is isolated.
pub fn work_key(case_id: &str) -> String {
    format!("fcb:work:{case_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_hash_is_deterministic_and_content_addressed() {
        let a = compute_bundle_hash(b"evidence-v1");
        let b = compute_bundle_hash(b"evidence-v1");
        let c = compute_bundle_hash(b"evidence-v2");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("sha256:"));
        assert_eq!(a.len(), 7 + 64);
    }

    #[test]
    fn binding_match_and_mismatches() {
        assert_eq!(
            verify_binding("case-1", "h1", "case-1", "h1"),
            BindingCheck::Match
        );
        assert_eq!(
            verify_binding("case-1", "h1", "case-2", "h1"),
            BindingCheck::CaseMismatch
        );
        // Same case, re-issued evidence -> version mismatch (the防呆 warning).
        assert_eq!(
            verify_binding("case-1", "h1", "case-1", "h2"),
            BindingCheck::EvidenceVersionMismatch
        );
    }

    #[test]
    fn different_cases_have_distinct_work_keys() {
        assert_ne!(work_key("case-1"), work_key("case-2"));
        assert_eq!(work_key("case-1"), work_key("case-1"));
    }
}
