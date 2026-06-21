//! WASM / JS bindings for the FCB codec.
//!
//! The browser workbench (and any other JS consumer) only sees the thin
//! `#[wasm_bindgen]` surface in [`wasm_api`]; everything below it is the already
//! byte-stable `fcb` codec. The bridge is split into a **native-testable core**
//! (this module) that returns plain Rust values, and a wasm-only marshaling
//! layer that serializes those values to JS via `serde-wasm-bindgen` and maps
//! [`FcbError`] onto a discriminable JS error. This keeps the meaningful logic
//! (stream decoding, error-kind mapping, binding) under `cargo test` without a
//! wasm runtime.

use ciborium::value::Value;
use serde::Serialize;

use fcb::binding::{self, BindingCheck};
use fcb::bundle::open_bytes;
use fcb::case::CasePayload;
use fcb::container::{peek_header, BundleKind};
use fcb::error::FcbError;
use fcb::evidence;
use fcb::submission::{self, Submission};
use fcb::task::{self, TaskSpec};

// ---------------------------------------------------------------------------
// JS-facing value shapes (serde -> JS objects via serde-wasm-bindgen).
// ---------------------------------------------------------------------------

/// One manifest entry surfaced before unlock (from the plaintext header).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestView {
    pub id: String,
    #[serde(rename = "type")]
    pub stream_type: String,
    pub records: u64,
    /// Whether the codec ships a built-in handler for this `type`.
    pub is_builtin: bool,
}

/// Header info readable without a passphrase.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PeekInfo {
    /// `"case"` or `"casework"`.
    pub kind: String,
    pub container_version: u16,
    pub header_schema_ver: u16,
    pub min_reader: u16,
    pub case_id: String,
    pub bundle_hash: String,
    /// Parsed stream manifest (empty for a `.casework`).
    pub streams: Vec<ManifestView>,
    /// Parsed task spec, when present.
    pub task: Option<TaskSpec>,
}

/// One decoded stream (manifest type joined with payload records).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StreamView {
    pub id: String,
    #[serde(rename = "type")]
    pub stream_type: String,
    pub is_builtin: bool,
    /// Records as decoded from the payload, byte-faithful (a `raw` field on a
    /// `fcb.syslog.v1` record survives verbatim).
    pub records: Vec<Value>,
}

/// A fully opened `.case`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CaseView {
    pub case_id: String,
    pub bundle_hash: String,
    pub task: Option<TaskSpec>,
    pub streams: Vec<StreamView>,
}

// The `.case` payload envelope `{ streams: [StreamData] }` is the crate's public
// `fcb::case::CasePayload` — the bridge decodes through that single shared type.

// ---------------------------------------------------------------------------
// Native-testable core.
// ---------------------------------------------------------------------------

/// KIND byte -> label. `peek_header` / `open_bytes` have already validated that
/// the byte is a known KIND (1 or 2) before this is reached.
fn kind_label(kind: BundleKind) -> &'static str {
    match kind {
        BundleKind::Case => "case",
        BundleKind::Work => "casework",
    }
}

/// Stable discriminator for each codec error, so JS callers can tell a wrong
/// passphrase from a corrupt bundle. Mirrors the `fcb-wasm-bridge` spec table.
pub fn error_kind(e: &FcbError) -> &'static str {
    match e {
        FcbError::BadMagic => "bad-magic",
        FcbError::UnsupportedVersion { .. } => "unsupported-version",
        FcbError::Malformed(_) => "malformed",
        FcbError::WrongPassphrase => "wrong-passphrase",
        FcbError::Corrupt => "corrupt",
    }
}

/// Stable label for a binding check result.
pub fn binding_label(check: BindingCheck) -> &'static str {
    match check {
        BindingCheck::Match => "match",
        BindingCheck::CaseMismatch => "case-mismatch",
        BindingCheck::EvidenceVersionMismatch => "evidence-version-mismatch",
    }
}

/// Read the plaintext header without a passphrase. Surfaces kind, versions, the
/// binding identity, the parsed manifest, and the task spec.
pub fn peek(bytes: &[u8]) -> Result<PeekInfo, FcbError> {
    let header = peek_header(bytes)?;
    // peek_header guarantees a validated KIND at [4] and >= 11 bytes total.
    let kind = match bytes[4] {
        1 => "case",
        _ => "casework",
    }
    .to_string();
    let container_version = u16::from_le_bytes([bytes[5], bytes[6]]);
    let manifest = evidence::manifest_from_meta(&header.meta)?;
    let streams = manifest
        .into_iter()
        .map(|m| ManifestView {
            is_builtin: evidence::is_builtin_type(&m.stream_type),
            id: m.id,
            stream_type: m.stream_type,
            records: m.records,
        })
        .collect();
    let task = task::task_from_meta(&header.meta)?;
    Ok(PeekInfo {
        kind,
        container_version,
        header_schema_ver: header.header_schema_ver,
        min_reader: header.min_reader,
        case_id: header.case_id,
        bundle_hash: header.bundle_hash,
        streams,
        task,
    })
}

/// Decrypt a `.case` and decode its streams. Rejects a bundle whose KIND is not
/// `case` (do not silently treat a `.casework` as a `.case`).
pub fn open_case(bytes: &[u8], passphrase: &str) -> Result<CaseView, FcbError> {
    let (kind, header, payload) = open_bytes(bytes, passphrase)?;
    if kind != BundleKind::Case {
        return Err(FcbError::Malformed(format!(
            "not a .case (KIND is {})",
            kind_label(kind)
        )));
    }
    let manifest = evidence::manifest_from_meta(&header.meta)?;
    let case_payload: CasePayload = fcb::cbor::decode(&payload)?;
    let decoded = evidence::decode_streams(&manifest, &case_payload.streams)?;
    let streams = decoded
        .into_iter()
        .map(|d| StreamView {
            id: d.id,
            stream_type: d.stream_type,
            is_builtin: d.is_builtin,
            records: d.records,
        })
        .collect();
    let task = task::task_from_meta(&header.meta)?;
    Ok(CaseView {
        case_id: header.case_id,
        bundle_hash: header.bundle_hash,
        task,
        streams,
    })
}

/// Open a `.casework` submission. Rejects a non-work KIND (delegated to the codec).
pub fn open_work(bytes: &[u8], passphrase: &str) -> Result<Submission, FcbError> {
    submission::open_submission(bytes, passphrase)
}

/// Pack a submission into a sealed `.casework` bundle.
pub fn pack_work(work: &Submission, passphrase: &str) -> Result<Vec<u8>, FcbError> {
    submission::pack_submission(work, passphrase)
}

/// `sha256:<hex>` content hash over the supplied bytes.
pub fn bundle_hash(bytes: &[u8]) -> String {
    binding::compute_bundle_hash(bytes)
}

/// Three-state binding check label between a submission and a case.
pub fn verify_binding(
    work_case_id: &str,
    work_bundle_hash: &str,
    case_id: &str,
    case_bundle_hash: &str,
) -> &'static str {
    binding_label(binding::verify_binding(
        work_case_id,
        work_bundle_hash,
        case_id,
        case_bundle_hash,
    ))
}

/// Local-storage partition key for a case's work.
pub fn work_key(case_id: &str) -> String {
    binding::work_key(case_id)
}

// ---------------------------------------------------------------------------
// WASM marshaling layer (compiles only for wasm32).
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    use fcb::error::FcbError;
    use fcb::submission::Submission;

    /// Serialize a value to a JS object (maps become plain objects, not `Map`).
    fn to_js(v: &impl serde::Serialize) -> Result<JsValue, JsValue> {
        let s = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        v.serialize(&s)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Build a JS `Error` carrying a stable `kind` discriminator.
    fn to_js_error(e: FcbError) -> JsValue {
        let err = js_sys::Error::new(&e.to_string());
        let _ = js_sys::Reflect::set(
            &err,
            &JsValue::from_str("kind"),
            &JsValue::from_str(crate::error_kind(&e)),
        );
        err.into()
    }

    /// Read the plaintext header without a passphrase.
    #[wasm_bindgen(js_name = peekHeader)]
    pub fn peek_header(bytes: &[u8]) -> Result<JsValue, JsValue> {
        let info = crate::peek(bytes).map_err(to_js_error)?;
        to_js(&info)
    }

    /// Decrypt a `.case` and decode its streams.
    #[wasm_bindgen(js_name = openCase)]
    pub fn open_case(bytes: &[u8], passphrase: &str) -> Result<JsValue, JsValue> {
        let view = crate::open_case(bytes, passphrase).map_err(to_js_error)?;
        to_js(&view)
    }

    /// Open a `.casework` submission.
    #[wasm_bindgen(js_name = openSubmission)]
    pub fn open_submission(bytes: &[u8], passphrase: &str) -> Result<JsValue, JsValue> {
        let work = crate::open_work(bytes, passphrase).map_err(to_js_error)?;
        to_js(&work)
    }

    /// Pack a submission (a JS object) into a sealed `.casework` bundle.
    #[wasm_bindgen(js_name = packSubmission)]
    pub fn pack_submission(submission: JsValue, passphrase: &str) -> Result<Vec<u8>, JsValue> {
        let work: Submission = serde_wasm_bindgen::from_value(submission)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        crate::pack_work(&work, passphrase).map_err(to_js_error)
    }

    /// `sha256:<hex>` over the supplied bytes.
    #[wasm_bindgen(js_name = computeBundleHash)]
    pub fn compute_bundle_hash(bytes: &[u8]) -> String {
        crate::bundle_hash(bytes)
    }

    /// Three-state binding check label.
    #[wasm_bindgen(js_name = verifyBinding)]
    pub fn verify_binding(
        work_case_id: &str,
        work_bundle_hash: &str,
        case_id: &str,
        case_bundle_hash: &str,
    ) -> String {
        crate::verify_binding(work_case_id, work_bundle_hash, case_id, case_bundle_hash).to_string()
    }

    /// Local-storage partition key for a case's work.
    #[wasm_bindgen(js_name = workKey)]
    pub fn work_key(case_id: &str) -> String {
        crate::work_key(case_id)
    }
}

// ---------------------------------------------------------------------------
// Native tests — exercise the core against real packed bundles + the spec
// error-kind table, with no wasm runtime.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fcb::bundle::{pack_bytes, BundleParams};
    use fcb::container::BundleKind;
    use fcb::evidence::{StreamData, StreamManifest};
    use fcb::submission::Student;
    use fcb::task::{ReportMode, TaskStep};

    const PASS: &str = "lab-pass";

    fn t(s: &str) -> Value {
        Value::Text(s.into())
    }

    fn syslog_record() -> Value {
        // A syslog record whose `raw` field must survive decoding verbatim.
        Value::Map(vec![
            (t("ts"), t("2026-03-14T08:21:33.512Z")),
            (t("host"), t("mymachine.example.com")),
            (t("msg"), t("'su root' failed")),
            (t("raw"), t("<34>1 2026-03-14T08:21:33.512Z mymachine.example.com su - ID47 - 'su root' failed")),
        ])
    }

    fn sample_task() -> TaskSpec {
        TaskSpec {
            report_mode: ReportMode::Steps,
            instructions: "Investigate the host.".into(),
            steps: vec![TaskStep {
                id: "q1".into(),
                prompt: "source IP?".into(),
                answer_type: "ip".into(),
            }],
        }
    }

    /// Build a real `.case` with one syslog stream and a task spec.
    fn build_case() -> Vec<u8> {
        #[derive(Serialize)]
        struct CaseMeta {
            streams: Vec<StreamManifest>,
            task: TaskSpec,
        }
        let manifest = vec![StreamManifest {
            id: "s0".into(),
            stream_type: "fcb.syslog.v1".into(),
            records: 1,
        }];
        let meta = fcb::cbor::to_value(&CaseMeta {
            streams: manifest,
            task: sample_task(),
        })
        .unwrap();
        // The encrypted body uses the crate's public envelope type.
        let payload = fcb::cbor::encode(&CasePayload {
            streams: vec![StreamData {
                id: "s0".into(),
                records: vec![syslog_record()],
            }],
        })
        .unwrap();
        let mut params =
            BundleParams::new(BundleKind::Case, "acme-ir-2026-03", "sha256:deadbeef", meta);
        params.m_cost = 32; // fast Argon2 for tests
        params.t_cost = 1;
        params.p_cost = 1;
        pack_bytes(&params, &payload, PASS).unwrap()
    }

    fn sample_submission() -> Submission {
        Submission {
            case_id: "acme-ir-2026-03".into(),
            bundle_hash: "sha256:deadbeef".into(),
            student: Student {
                id: "s1".into(),
                name: "Lin".into(),
            },
            notes: vec![t("pinned line 42")],
            report: t("freeform body"),
            activity: vec![t("search: failed login")],
            exported_at: "2026-06-21T10:00:00Z".into(),
        }
    }

    #[test]
    fn error_kind_matches_spec_table() {
        assert_eq!(error_kind(&FcbError::BadMagic), "bad-magic");
        assert_eq!(
            error_kind(&FcbError::UnsupportedVersion {
                min_reader: 2,
                supported: 1
            }),
            "unsupported-version"
        );
        assert_eq!(error_kind(&FcbError::Malformed("x".into())), "malformed");
        assert_eq!(error_kind(&FcbError::WrongPassphrase), "wrong-passphrase");
        assert_eq!(error_kind(&FcbError::Corrupt), "corrupt");
    }

    #[test]
    fn peek_reads_header_without_passphrase() {
        let bytes = build_case();
        let info = peek(&bytes).unwrap();
        assert_eq!(info.kind, "case");
        assert_eq!(info.container_version, 1);
        assert_eq!(info.case_id, "acme-ir-2026-03");
        assert_eq!(info.streams.len(), 1);
        assert_eq!(info.streams[0].stream_type, "fcb.syslog.v1");
        assert!(info.streams[0].is_builtin);
        assert!(info.task.is_some());
    }

    #[test]
    fn peek_rejects_non_fcb() {
        assert_eq!(
            error_kind(&peek(b"\x00ELF not fcb").unwrap_err()),
            "bad-magic"
        );
    }

    #[test]
    fn open_case_decodes_streams_and_preserves_raw() {
        let bytes = build_case();
        let view = open_case(&bytes, PASS).unwrap();
        assert_eq!(view.case_id, "acme-ir-2026-03");
        assert_eq!(view.streams.len(), 1);
        assert_eq!(view.streams[0].stream_type, "fcb.syslog.v1");
        // Byte-faithful: the record (including its `raw` field) round-trips verbatim.
        assert_eq!(view.streams[0].records, vec![syslog_record()]);
        // Task survives.
        assert_eq!(view.task.unwrap().report_mode, ReportMode::Steps);
    }

    #[test]
    fn open_case_wrong_passphrase_maps_to_kind() {
        let bytes = build_case();
        let err = open_case(&bytes, "nope").unwrap_err();
        assert_eq!(error_kind(&err), "wrong-passphrase");
    }

    #[test]
    fn open_case_rejects_a_casework() {
        let work = pack_work(&sample_submission(), PASS).unwrap();
        assert!(matches!(
            open_case(&work, PASS),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn submission_round_trips_through_bridge() {
        let work = sample_submission();
        let bytes = pack_work(&work, PASS).unwrap();
        assert_eq!(open_work(&bytes, PASS).unwrap(), work);
    }

    #[test]
    fn open_work_rejects_a_case() {
        let case = build_case();
        assert!(matches!(
            open_work(&case, PASS),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn bundle_hash_and_binding_and_work_key() {
        let h = bundle_hash(b"evidence-v1");
        assert!(h.starts_with("sha256:"));
        assert_eq!(h.len(), 7 + 64);
        assert_eq!(verify_binding("c", "h", "c", "h"), "match");
        assert_eq!(verify_binding("c", "h", "c2", "h"), "case-mismatch");
        assert_eq!(
            verify_binding("c", "h", "c", "h2"),
            "evidence-version-mismatch"
        );
        assert_ne!(work_key("c1"), work_key("c2"));
    }
}
