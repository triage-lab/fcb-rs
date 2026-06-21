//! WASM / JS bindings for the FCB codec.
//!
//! The browser workbench (and any other JS consumer) only sees the thin
//! `#[wasm_bindgen]` surface in the wasm-only `wasm_api` module; everything below it is the already
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
use fcb::case::{pack_case as fcb_pack_case, CaseInput, CasePayload};
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
    // Verify the declared content address against the actual payload. The header
    // is now AEAD-authenticated, but AAD only guarantees the bundle_hash field
    // was not tampered — not that the producer's value matches the payload. For
    // a `.case`, bundle_hash IS the hash of the canonical payload, so recompute
    // and compare; a mismatch is a malformed/forged bundle -> Corrupt. (A
    // `.casework` header bundle_hash is a binding reference to its case, not the
    // hash of the submission payload, so open_work does not do this.)
    if binding::compute_bundle_hash(&payload) != header.bundle_hash {
        return Err(FcbError::Corrupt);
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

/// The largest IEEE-754 double that is an exact ("safe") integer: `2^53 - 1`.
/// Beyond this magnitude a JS `number` cannot faithfully carry an integer, so
/// serde-wasm-bindgen encodes such a value as a CBOR float — diverging from a
/// native producer that would use a CBOR integer, which would change the
/// canonical payload and the `bundle_hash`. Out-of-range integers MUST instead
/// be supplied as a `BigInt`, which serde-wasm-bindgen encodes as a CBOR integer.
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Enforce the deterministic-encoding contract on a record value: reject any
/// integer-valued float whose magnitude exceeds the JS safe-integer range
/// (a number that should have been a `BigInt`). Genuine fractional floats,
/// safe-range integers, and BigInt-sourced integers (which arrive as CBOR
/// integers, not floats) all pass. Recurses into arrays, maps, and tags.
fn check_numeric_determinism(value: &Value) -> Result<(), FcbError> {
    match value {
        Value::Float(f) if f.is_finite() && f.fract() == 0.0 && f.abs() > MAX_SAFE_INTEGER => {
            Err(FcbError::Malformed(format!(
                "record holds an integer-valued number {f} outside the JS safe-integer range; \
                 supply it as a BigInt to preserve its integer encoding"
            )))
        }
        Value::Array(items) => items.iter().try_for_each(check_numeric_determinism),
        Value::Map(entries) => entries.iter().try_for_each(|(k, v)| {
            check_numeric_determinism(k)?;
            check_numeric_determinism(v)
        }),
        Value::Tag(_, inner) => check_numeric_determinism(inner),
        _ => Ok(()),
    }
}

/// Pack a submission into a sealed `.casework` bundle. Enforces the pack-boundary
/// numeric-determinism contract over the submission's record fields.
pub fn pack_work(work: &Submission, passphrase: &str) -> Result<Vec<u8>, FcbError> {
    for v in work
        .notes
        .iter()
        .chain(std::iter::once(&work.report))
        .chain(work.activity.iter())
    {
        check_numeric_determinism(v)?;
    }
    submission::pack_submission(work, passphrase)
}

/// Pack an evidence case into a sealed `.case` bundle (mirror of `pack_work`).
/// Enforces the pack-boundary numeric-determinism contract over every record.
pub fn pack_case(input: &CaseInput, passphrase: &str) -> Result<Vec<u8>, FcbError> {
    for stream in &input.payload.streams {
        for record in &stream.records {
            check_numeric_determinism(record)?;
        }
    }
    fcb_pack_case(input, passphrase)
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
    use fcb::case::CaseInput;

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

    /// Pack a case (a JS object) into a sealed `.case` bundle.
    #[wasm_bindgen(js_name = packCase)]
    pub fn pack_case(input: JsValue, passphrase: &str) -> Result<Vec<u8>, JsValue> {
        let case_input: CaseInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        crate::pack_case(&case_input, passphrase).map_err(to_js_error)
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
        let case_payload = CasePayload {
            streams: vec![StreamData {
                id: "s0".into(),
                records: vec![syslog_record()],
            }],
        };
        // bundle_hash must equal the canonical payload's hash — open_case now
        // verifies the content address.
        let bundle_hash = fcb::case::case_bundle_hash(&case_payload).unwrap();
        let payload = fcb::cbor::encode(&case_payload).unwrap();
        let mut params = BundleParams::new(BundleKind::Case, "acme-ir-2026-03", bundle_hash, meta);
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

    fn sample_case_input() -> CaseInput {
        CaseInput {
            case_id: "acme-ir-2026-03".into(),
            manifest: vec![StreamManifest {
                id: "s0".into(),
                stream_type: "fcb.syslog.v1".into(),
                records: 1,
            }],
            task: Some(sample_task()),
            payload: CasePayload {
                streams: vec![StreamData {
                    id: "s0".into(),
                    records: vec![syslog_record()],
                }],
            },
        }
    }

    #[test]
    fn open_case_rejects_bundle_hash_mismatch() {
        // A .case whose header bundle_hash does not match its canonical payload
        // must fail as Corrupt at open — even though the header is now
        // AEAD-authenticated and the passphrase is correct — because open_case
        // recomputes and verifies the content address.
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
        let payload = fcb::cbor::encode(&CasePayload {
            streams: vec![StreamData {
                id: "s0".into(),
                records: vec![syslog_record()],
            }],
        })
        .unwrap();
        // Deliberately wrong content address (does not hash the payload above).
        let mut params =
            BundleParams::new(BundleKind::Case, "acme-ir-2026-03", "sha256:deadbeef", meta);
        params.m_cost = 32;
        params.t_cost = 1;
        params.p_cost = 1;
        let bytes = pack_bytes(&params, &payload, PASS).unwrap();
        assert_eq!(open_case(&bytes, PASS).unwrap_err(), FcbError::Corrupt);
        // A correctly-hashed case still opens fine.
        assert!(open_case(&build_case(), PASS).is_ok());
    }

    #[test]
    fn check_numeric_determinism_boundary() {
        use ciborium::value::Value;
        // Safe-range integer and genuine float pass.
        assert!(super::check_numeric_determinism(&Value::Integer(7.into())).is_ok());
        assert!(super::check_numeric_determinism(&Value::Float(1.5)).is_ok());
        // Integer-valued float inside the safe range passes (2^53 - 1 boundary).
        assert!(super::check_numeric_determinism(&Value::Float(9_007_199_254_740_991.0)).is_ok());
        // 2^53 (integer-valued float, out of safe range) is rejected.
        assert!(matches!(
            super::check_numeric_determinism(&Value::Float(9_007_199_254_740_992.0)),
            Err(FcbError::Malformed(_))
        ));
        // Nested inside an array/map is also caught.
        let nested = Value::Array(vec![Value::Map(vec![(
            Value::Text("k".into()),
            Value::Float(9_007_199_254_740_992.0),
        )])]);
        assert!(matches!(
            super::check_numeric_determinism(&nested),
            Err(FcbError::Malformed(_))
        ));
        // NaN / Inf are genuine (non-integer-valued) floats -> allowed.
        assert!(super::check_numeric_determinism(&Value::Float(f64::NAN)).is_ok());
        assert!(super::check_numeric_determinism(&Value::Float(f64::INFINITY)).is_ok());
    }

    #[test]
    fn pack_case_rejects_out_of_safe_range_integer() {
        // A record carrying an integer-valued float beyond 2^53 is rejected at
        // the pack boundary (it would diverge from a native integer author).
        let input = CaseInput {
            case_id: "c".into(),
            manifest: vec![StreamManifest {
                id: "s0".into(),
                stream_type: "fcb.json.v1".into(),
                records: 1,
            }],
            task: None,
            payload: CasePayload {
                streams: vec![StreamData {
                    id: "s0".into(),
                    records: vec![Value::Float(9_007_199_254_740_992.0)],
                }],
            },
        };
        assert!(matches!(pack_case(&input, PASS), Err(FcbError::Malformed(_))));
        // A safe-range integer record packs fine.
        let ok = CaseInput {
            payload: CasePayload {
                streams: vec![StreamData {
                    id: "s0".into(),
                    records: vec![Value::Integer(7.into())],
                }],
            },
            ..input
        };
        assert!(pack_case(&ok, PASS).is_ok());
    }

    #[test]
    fn pack_work_rejects_out_of_safe_range_integer() {
        let mut work = sample_submission();
        work.activity = vec![Value::Float(9_007_199_254_740_992.0)];
        assert!(matches!(pack_work(&work, PASS), Err(FcbError::Malformed(_))));
    }

    #[test]
    fn case_round_trips_through_bridge() {
        let input = sample_case_input();
        let bytes = pack_case(&input, PASS).unwrap();
        let view = open_case(&bytes, PASS).unwrap();
        assert_eq!(view.case_id, "acme-ir-2026-03");
        assert_eq!(view.streams.len(), 1);
        assert_eq!(view.streams[0].stream_type, "fcb.syslog.v1");
        // Byte-faithful records, including the syslog `raw` field.
        assert_eq!(view.streams[0].records, vec![syslog_record()]);
        assert_eq!(view.task.unwrap().report_mode, ReportMode::Steps);
        // Canonical, deterministic content hash — equals hashing the payload directly.
        assert_eq!(
            view.bundle_hash,
            fcb::case::case_bundle_hash(&input.payload).unwrap()
        );
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

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::wasm_bindgen_test;

    const PASS: &str = "lab-pass";
    // Mirrors crates/fcb/tests/vectors.rs FROZEN_CASE_BUNDLE_HASH: the canonical
    // hash of the fixed case payload (s0: ["evt1","evt2"], s1: [7]). Independent
    // of the random salt/nonce a sealed bundle uses.
    const FROZEN_CASE_BUNDLE_HASH: &str =
        "sha256:376d586b42b0e800a6e78fea8bfb9a68cb569d033cc324b7b9b1800fc508eccf";

    // A JS-authored case crosses the real serde-wasm-bindgen boundary: the
    // integer `7` is a plain JS number. If it deserialized to a CBOR float
    // instead of an integer, the canonical payload — and this hash — would differ.
    #[wasm_bindgen_test]
    fn js_authored_case_hashes_identically_to_native() {
        let case = js_sys::JSON::parse(
            r#"{
                "case_id": "acme-ir-2026-03",
                "manifest": [
                    { "id": "s0", "type": "fcb.syslog.v1", "records": 2 },
                    { "id": "s1", "type": "fcb.syslog.v1", "records": 1 }
                ],
                "payload": { "streams": [
                    { "id": "s0", "records": ["evt1", "evt2"] },
                    { "id": "s1", "records": [7] }
                ] }
            }"#,
        )
        .unwrap();

        let bytes = crate::wasm_api::pack_case(case, PASS).unwrap();
        // peekHeader reads bundle_hash from the plaintext header — no decode needed.
        let info = crate::wasm_api::peek_header(&bytes).unwrap();
        let bundle_hash = js_sys::Reflect::get(&info, &JsValue::from_str("bundle_hash"))
            .unwrap()
            .as_string()
            .unwrap();
        assert_eq!(bundle_hash, FROZEN_CASE_BUNDLE_HASH);
    }

    /// Build a single-stream `.case` carrying one `record` value and return its
    /// bundle_hash, or the error `kind` if packing failed.
    fn pack_hash_for(record: JsValue) -> Result<String, String> {
        use js_sys::{Array, Object, Reflect};
        let me = Object::new();
        Reflect::set(&me, &"id".into(), &"s0".into()).unwrap();
        Reflect::set(&me, &"type".into(), &"fcb.json.v1".into()).unwrap();
        Reflect::set(&me, &"records".into(), &JsValue::from(1u32)).unwrap();
        let manifest = Array::new();
        manifest.push(&me);

        let records = Array::new();
        records.push(&record);
        let stream = Object::new();
        Reflect::set(&stream, &"id".into(), &"s0".into()).unwrap();
        Reflect::set(&stream, &"records".into(), &records).unwrap();
        let streams = Array::new();
        streams.push(&stream);
        let payload = Object::new();
        Reflect::set(&payload, &"streams".into(), &streams).unwrap();

        let case = Object::new();
        Reflect::set(&case, &"case_id".into(), &"c".into()).unwrap();
        Reflect::set(&case, &"manifest".into(), &manifest).unwrap();
        Reflect::set(&case, &"payload".into(), &payload).unwrap();

        match crate::wasm_api::pack_case(case.into(), PASS) {
            Ok(bytes) => {
                let info = crate::wasm_api::peek_header(&bytes).unwrap();
                Ok(Reflect::get(&info, &"bundle_hash".into())
                    .unwrap()
                    .as_string()
                    .unwrap())
            }
            Err(e) => Err(Reflect::get(&e, &"kind".into())
                .ok()
                .and_then(|k| k.as_string())
                .unwrap_or_else(|| "unknown".into())),
        }
    }

    // Characterization (verified at the real serde-wasm-bindgen boundary): a
    // plain JS number at/above 2^53 encodes as a CBOR float — a distinct
    // canonical encoding from a BigInt, which encodes as a CBOR integer. So the
    // pack boundary rejects integer-valued out-of-safe-range numbers and
    // requires a BigInt for them; safe-range integers and genuine floats pass.
    #[wasm_bindgen_test]
    fn numeric_boundary_contract() {
        use js_sys::BigInt;
        // Safe-range integers pack fine (7 and 2^53 - 1).
        assert!(pack_hash_for(JsValue::from_f64(7.0)).is_ok());
        assert!(pack_hash_for(JsValue::from_f64(9007199254740991.0)).is_ok());
        // A genuine (fractional) float packs fine.
        assert!(pack_hash_for(JsValue::from_f64(1.5)).is_ok());
        // A plain JS number at 2^53 (integer-valued, out of safe range) is rejected.
        assert_eq!(
            pack_hash_for(JsValue::from_f64(9007199254740992.0)),
            Err("malformed".to_string())
        );
        // The same out-of-range integer supplied as a BigInt packs fine...
        let big = pack_hash_for(JsValue::from(BigInt::from(9007199254740993i64)));
        assert!(big.is_ok());
        // ...and a different BigInt integer yields a different hash (lossless).
        let big2 = pack_hash_for(JsValue::from(BigInt::from(9007199254740994i64)));
        assert!(big2.is_ok());
        assert_ne!(big, big2);
    }
}
