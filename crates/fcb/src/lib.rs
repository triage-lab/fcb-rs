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
