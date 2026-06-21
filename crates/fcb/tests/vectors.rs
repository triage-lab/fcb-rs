//! Protocol test vectors + round-trip suite.
//!
//! `FROZEN_*_HEX` are byte-exact baselines produced with a fixed salt/nonce.
//! They serve two jobs:
//!   1. Cross-project compatibility: any FCB implementation must decode them.
//!   2. Format-regression guard: rebuilding them must reproduce the same bytes,
//!      so an accidental layout change fails this test instead of silently
//!      breaking interop.
//!
//! Random-salt round-trips cover end-to-end behavior of the public API.

use ciborium::value::Value;
use fcb::binding::{verify_binding, BindingCheck};
use fcb::bundle::{self, BundleParams};
use fcb::case::CasePayload;
use fcb::container::{write_container, AeadParams, BundleKind, Header, KdfParams};
use fcb::evidence::{decode_streams, manifest_from_meta, StreamData, StreamManifest};
use fcb::submission::{open_submission, pack_submission, Student, Submission};
use fcb::task::{task_from_meta, ReportMode, TaskSpec, TaskStep};
use fcb::{cbor, compress, crypto};
use serde::{Deserialize, Serialize};

const PASS: &str = "lab-pass";
const SALT: [u8; 16] = [
    0x53, 0x41, 0x4c, 0x54, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
];
const NONCE: [u8; 24] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
];

const FROZEN_CASE_HEX: &str = "89464342010100dc010000a8716865616465725f736368656d615f766572016a6d696e5f7265616465720167636173655f69646f61636d652d69722d323032362d30336b62756e646c655f686173686f7368613235363a6465616462656566636b6466a564616c676f686172676f6e3269646473616c749018531841184c18540102030405060708090a0b0c666d5f636f7374182066745f636f73740166705f636f7374016461656164a264616c676f71786368616368613230706f6c7931333035656e6f6e63659818000102030405060708090a0b0c0d0e0f1011121314151617696b65795f636865636b9820182f183218cc1218e5185d18780b18d118ca18a618bf18bb0b182d18f9184b187118db18da18de184c184a18770618ee18d618d7188918c7188518fa646d657461a26773747265616d7382a362696462733064747970656d6663622e7379736c6f672e7631677265636f72647302a362696462733164747970656b61636d652e6564722e7631677265636f72647301647461736ba36b7265706f72745f6d6f64656573746570736c696e737472756374696f6e7375496e7665737469676174652074686520686f73742e65737465707381a36269646271316670726f6d70746a736f757263652049503f6b616e737765725f74797065626970d9f8b055c0e997faa186caa996a8fb71e404713f056473e25a6e4a60166a1093693f25fed946e04aa8e1a2daf98821f7de7dc9af29db4d40b21c3f2fcc033d357eeb39cf0122b45f5b5f11c672741cf471ee532a30023cee6c0753";
const FROZEN_WORK_HEX: &str = "8946434202010025010000a8716865616465725f736368656d615f766572016a6d696e5f7265616465720167636173655f69646f61636d652d69722d323032362d30336b62756e646c655f686173686f7368613235363a6465616462656566636b6466a564616c676f686172676f6e3269646473616c749018531841184c18540102030405060708090a0b0c666d5f636f7374182066745f636f73740166705f636f7374016461656164a264616c676f71786368616368613230706f6c7931333035656e6f6e63659818000102030405060708090a0b0c0d0e0f1011121314151617696b65795f636865636b9820182f183218cc1218e5185d18780b18d118ca18a618bf18bb0b182d18f9184b187118db18da18de184c184a18770618ee18d618d7188918c7188518fa646d657461a0d9f8b055c0e9b7f9a176cca994a8eb64e5044f3b1289b0e35e6f057a542050c638667aaa99afe65ab0f1ffdbc39634b61423dd464febd0660289a532b8a25740924c4c17076de5bfaf05521b11d97f7a4d26c156a1d744f13e1f991f3041c88641e9b85279a5bac3938b2f77d5ac5567e1d80b376a9070";

/// Frozen canonical `bundle_hash` over the fixed case payload below:
/// `sha256(canonical plaintext payload bytes)`. Pins the producer/consumer
/// content-address contract.
const FROZEN_CASE_BUNDLE_HASH: &str =
    "sha256:376d586b42b0e800a6e78fea8bfb9a68cb569d033cc324b7b9b1800fc508eccf";

/// The fixed `.case` payload streams shared by the canonical-hash and pack_case
/// tests (and identical to what `build_case` carries).
fn fixed_case_streams() -> Vec<StreamData> {
    vec![
        StreamData {
            id: "s0".into(),
            records: vec![Value::Text("evt1".into()), Value::Text("evt2".into())],
        },
        StreamData {
            id: "s1".into(),
            records: vec![Value::Integer(7.into())],
        },
    ]
}

#[derive(Serialize)]
struct CaseMeta {
    streams: Vec<StreamManifest>,
    task: TaskSpec,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct WorkPayload {
    case_id: String,
    bundle_hash: String,
    report: Value,
}

/// Deterministic build (fixed salt/nonce) — must match the frozen vectors.
fn build(kind: BundleKind, meta: Value, payload: Vec<u8>) -> Vec<u8> {
    let kdf = KdfParams {
        algo: "argon2id".into(),
        salt: SALT.to_vec(),
        m_cost: 32,
        t_cost: 1,
        p_cost: 1,
    };
    let key = crypto::derive_key(PASS, &kdf).unwrap();
    let kcv = crypto::key_check_value(&key);
    let ct = compress::pack_payload(&key, &NONCE, &payload).unwrap();
    let header = Header {
        header_schema_ver: 1,
        min_reader: 1,
        case_id: "acme-ir-2026-03".into(),
        bundle_hash: "sha256:deadbeef".into(),
        kdf,
        aead: AeadParams {
            algo: "xchacha20poly1305".into(),
            nonce: NONCE.to_vec(),
        },
        key_check: kcv,
        meta,
    };
    write_container(kind, &header, &ct).unwrap()
}

fn build_case() -> Vec<u8> {
    let manifest = vec![
        StreamManifest {
            id: "s0".into(),
            stream_type: "fcb.syslog.v1".into(),
            records: 2,
        },
        StreamManifest {
            id: "s1".into(),
            stream_type: "acme.edr.v1".into(),
            records: 1,
        },
    ];
    let task = TaskSpec {
        report_mode: ReportMode::Steps,
        instructions: "Investigate the host.".into(),
        steps: vec![TaskStep {
            id: "q1".into(),
            prompt: "source IP?".into(),
            answer_type: "ip".into(),
        }],
    };
    let meta = cbor::to_value(&CaseMeta {
        streams: manifest,
        task,
    })
    .unwrap();
    let payload = cbor::encode(&CasePayload {
        streams: vec![
            StreamData {
                id: "s0".into(),
                records: vec![Value::Text("evt1".into()), Value::Text("evt2".into())],
            },
            StreamData {
                id: "s1".into(),
                records: vec![Value::Integer(7.into())],
            },
        ],
    })
    .unwrap();
    build(BundleKind::Case, meta, payload)
}

fn build_work() -> Vec<u8> {
    let payload = cbor::encode(&WorkPayload {
        case_id: "acme-ir-2026-03".into(),
        bundle_hash: "sha256:deadbeef".into(),
        report: Value::Text("freeform report".into()),
    })
    .unwrap();
    build(BundleKind::Work, Value::Map(vec![]), payload)
}

#[test]
fn case_vector_is_byte_stable() {
    assert_eq!(
        hex::encode(build_case()),
        FROZEN_CASE_HEX,
        "case format drifted"
    );
}

#[test]
fn work_vector_is_byte_stable() {
    assert_eq!(
        hex::encode(build_work()),
        FROZEN_WORK_HEX,
        "casework format drifted"
    );
}

#[test]
fn frozen_case_vector_decodes_to_expected_structure() {
    let bytes = hex::decode(FROZEN_CASE_HEX).unwrap();
    let (kind, header, payload) = bundle::open_bytes(&bytes, PASS).unwrap();
    assert_eq!(kind, BundleKind::Case);
    assert_eq!(header.case_id, "acme-ir-2026-03");
    assert_eq!(header.bundle_hash, "sha256:deadbeef");

    // Manifest + task are readable from the plaintext meta.
    let manifest = manifest_from_meta(&header.meta).unwrap();
    assert_eq!(manifest.len(), 2);
    let task = task_from_meta(&header.meta).unwrap().unwrap();
    assert_eq!(task.report_mode, ReportMode::Steps);
    assert_eq!(task.steps[0].id, "q1");

    // Payload streams join with the manifest; unknown type still surfaces.
    let case: CasePayload = cbor::decode(&payload).unwrap();
    let streams = decode_streams(&manifest, &case.streams).unwrap();
    assert_eq!(streams[0].stream_type, "fcb.syslog.v1");
    assert!(streams[0].is_builtin);
    assert_eq!(streams[0].records.len(), 2);
    assert_eq!(streams[1].stream_type, "acme.edr.v1");
    assert!(!streams[1].is_builtin);
}

#[test]
fn frozen_work_vector_decodes_to_expected_structure() {
    let bytes = hex::decode(FROZEN_WORK_HEX).unwrap();
    let (kind, header, payload) = bundle::open_bytes(&bytes, PASS).unwrap();
    assert_eq!(kind, BundleKind::Work);
    let work: WorkPayload = cbor::decode(&payload).unwrap();
    assert_eq!(work.case_id, "acme-ir-2026-03");
    // Binding check confirms the work matches the case + evidence version.
    assert_eq!(
        verify_binding(
            &work.case_id,
            &work.bundle_hash,
            &header.case_id,
            &header.bundle_hash
        ),
        BindingCheck::Match
    );
}

#[test]
fn wrong_passphrase_on_vector_is_rejected() {
    let bytes = hex::decode(FROZEN_CASE_HEX).unwrap();
    assert_eq!(
        bundle::open_bytes(&bytes, "wrong").unwrap_err(),
        fcb::FcbError::WrongPassphrase
    );
}

#[test]
fn tampered_vector_is_corrupt() {
    let mut bytes = hex::decode(FROZEN_CASE_HEX).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01; // flip a payload byte
    assert_eq!(
        bundle::open_bytes(&bytes, PASS).unwrap_err(),
        fcb::FcbError::Corrupt
    );
}

#[test]
fn submission_random_round_trip() {
    // End-to-end through the public API with random salt/nonce.
    let work = Submission {
        case_id: "acme-ir-2026-03".into(),
        bundle_hash: "sha256:deadbeef".into(),
        student: Student {
            id: "s1".into(),
            name: "Lin".into(),
        },
        notes: vec![Value::Text("pinned line 42".into())],
        report: Value::Text("body".into()),
        activity: vec![Value::Text("search".into())],
        exported_at: "2026-06-20T10:00:00Z".into(),
    };
    let bytes = pack_submission(&work, "pw").unwrap();
    assert_eq!(open_submission(&bytes, "pw").unwrap(), work);
}

#[test]
fn case_random_round_trip() {
    let mut params = BundleParams::new(BundleKind::Case, "c", "h", Value::Map(vec![]));
    params.m_cost = 32;
    params.t_cost = 1;
    params.p_cost = 1;
    let bytes = bundle::pack_bytes(&params, b"evidence", "pw").unwrap();
    let (kind, _h, payload) = bundle::open_bytes(&bytes, "pw").unwrap();
    assert_eq!(kind, BundleKind::Case);
    assert_eq!(payload, b"evidence");
}

#[test]
fn case_canonical_bundle_hash_is_frozen() {
    // Canonical bundle_hash = sha256(canonical plaintext payload bytes), frozen
    // so producers and consumers agree on one content address, independent of
    // any random salt/nonce.
    let payload = fcb::case::CasePayload {
        streams: fixed_case_streams(),
    };
    assert_eq!(
        fcb::case::case_bundle_hash(&payload).unwrap(),
        FROZEN_CASE_BUNDLE_HASH,
        "canonical bundle_hash drifted"
    );
    // It equals compute_bundle_hash applied to the canonical bytes directly.
    let canonical = payload.to_canonical_bytes().unwrap();
    assert_eq!(
        fcb::case::case_bundle_hash(&payload).unwrap(),
        fcb::binding::compute_bundle_hash(&canonical)
    );
}

#[test]
fn pack_case_round_trips_and_binds_hash() {
    let manifest = vec![
        StreamManifest {
            id: "s0".into(),
            stream_type: "fcb.syslog.v1".into(),
            records: 2,
        },
        StreamManifest {
            id: "s1".into(),
            stream_type: "acme.edr.v1".into(),
            records: 1,
        },
    ];
    let task = TaskSpec {
        report_mode: ReportMode::Steps,
        instructions: "Investigate the host.".into(),
        steps: vec![TaskStep {
            id: "q1".into(),
            prompt: "source IP?".into(),
            answer_type: "ip".into(),
        }],
    };
    let payload = fcb::case::CasePayload {
        streams: fixed_case_streams(),
    };
    let input = fcb::case::CaseInput {
        case_id: "acme-ir-2026-03".into(),
        manifest: manifest.clone(),
        task: Some(task.clone()),
        payload: payload.clone(),
    };
    let bytes = fcb::case::pack_case(&input, PASS).unwrap();

    let (kind, header, body) = bundle::open_bytes(&bytes, PASS).unwrap();
    assert_eq!(kind, BundleKind::Case);
    assert_eq!(header.case_id, "acme-ir-2026-03");
    // The header bundle_hash is bound to the canonical payload.
    assert_eq!(
        header.bundle_hash,
        fcb::case::case_bundle_hash(&payload).unwrap()
    );

    // Manifest + task are readable from the plaintext meta.
    let read_manifest = manifest_from_meta(&header.meta).unwrap();
    assert_eq!(read_manifest, manifest);
    assert_eq!(task_from_meta(&header.meta).unwrap(), Some(task));

    // Streams decode and join; the built-in flag reflects the manifest type.
    let back: fcb::case::CasePayload = cbor::decode(&body).unwrap();
    let streams = decode_streams(&read_manifest, &back.streams).unwrap();
    assert_eq!(streams[0].stream_type, "fcb.syslog.v1");
    assert!(streams[0].is_builtin);
    assert_eq!(streams[0].records.len(), 2);
    assert_eq!(streams[1].stream_type, "acme.edr.v1");
    assert!(!streams[1].is_builtin);
}
