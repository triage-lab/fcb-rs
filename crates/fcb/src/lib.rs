//! FCB — Forensic Case Bundle codec.
//!
//! One Rust crate, two compile targets: native (for the authoring CLI and the
//! teacher review platform) and WASM (for the in-browser student workbench).
//! It owns the FCB container format, passphrase-based crypto, compression, the
//! self-describing evidence model, the embedded task spec, and submission
//! packaging.
//!
//! This module wires the pieces together; the on-the-wire contract lives in
//! `openspec/specs/fcb-*`.
//!
//! # Example
//!
//! Pack an evidence case and open it back through the public API:
//!
//! ```
//! use ciborium::value::Value;
//! use fcb::case::{pack_case, CaseInput, CasePayload};
//! use fcb::container::BundleKind;
//! use fcb::evidence::{StreamData, StreamManifest};
//!
//! let manifest = vec![StreamManifest {
//!     id: "s0".into(),
//!     stream_type: "example.note.v1".into(),
//!     records: 1,
//! }];
//! let payload = CasePayload {
//!     streams: vec![StreamData {
//!         id: "s0".into(),
//!         records: vec![Value::Text("an event".into())],
//!     }],
//! };
//! let input = CaseInput { case_id: "demo".into(), manifest, task: None, payload };
//!
//! // pack_case computes the canonical bundle_hash, builds the header meta,
//! // and seals with a fresh random salt/nonce. The literal "passphrase" here is
//! // illustration only — never hardcode a passphrase in real code.
//! let bytes = pack_case(&input, "passphrase")?;
//!
//! let (kind, header, _payload) = fcb::bundle::open_bytes(&bytes, "passphrase")?;
//! assert_eq!(kind, BundleKind::Case);
//! assert_eq!(header.case_id, "demo");
//! # Ok::<(), fcb::FcbError>(())
//! ```
//!
//! See [`docs/fcb-integration-guide.md`](https://github.com/triage-lab/fcb-rs/blob/main/docs/fcb-integration-guide.md)
//! for the full consumer guide.

pub mod binding;
pub mod bundle;
pub mod case;
pub mod cbor;
pub mod compress;
pub mod container;
pub mod crypto;
pub mod error;
pub mod evidence;
pub mod submission;
pub mod task;

pub use error::{FcbError, Result};

#[cfg(target_arch = "wasm32")]
mod wasm;
