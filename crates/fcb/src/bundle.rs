//! Top-level codec: ties container framing, crypto, and compression into
//! `pack`/`open` for whole bundles. This is the single place that generates a
//! fresh salt + nonce, derives the key, and assembles the plaintext header.

use ciborium::value::Value;

use crate::compress;
use crate::container::{
    read_container, write_container, AeadParams, BundleKind, Header, KdfParams,
};
use crate::crypto::{self, NONCE_LEN};
use crate::error::{FcbError, Result};

/// Default Argon2id cost (memory in KiB, iterations, parallelism). Tuned for an
/// interactive in-browser unlock; stored per-bundle so it can evolve.
pub const DEFAULT_M_COST: u32 = 19456;
pub const DEFAULT_T_COST: u32 = 2;
pub const DEFAULT_P_COST: u32 = 1;
const SALT_LEN: usize = 16;

fn random_bytes(n: usize) -> Result<Vec<u8>> {
    let mut b = vec![0u8; n];
    getrandom::getrandom(&mut b).map_err(|e| FcbError::Malformed(format!("rng failure: {e}")))?;
    Ok(b)
}

/// Inputs for packing a bundle. `meta` is the opaque header metadata (stream
/// manifest and/or task spec, built by the evidence/task layers).
#[derive(Debug, Clone)]
pub struct BundleParams {
    pub kind: BundleKind,
    pub case_id: String,
    pub bundle_hash: String,
    pub meta: Value,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl BundleParams {
    /// Build params with the default Argon2id cost.
    pub fn new(
        kind: BundleKind,
        case_id: impl Into<String>,
        bundle_hash: impl Into<String>,
        meta: Value,
    ) -> Self {
        BundleParams {
            kind,
            case_id: case_id.into(),
            bundle_hash: bundle_hash.into(),
            meta,
            m_cost: DEFAULT_M_COST,
            t_cost: DEFAULT_T_COST,
            p_cost: DEFAULT_P_COST,
        }
    }
}

/// Pack payload bytes into a sealed FCB container (compress-then-encrypt) with
/// a fresh random salt and nonce.
pub fn pack_bytes(params: &BundleParams, payload: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let kdf = KdfParams {
        algo: "argon2id".into(),
        salt: random_bytes(SALT_LEN)?,
        m_cost: params.m_cost,
        t_cost: params.t_cost,
        p_cost: params.p_cost,
    };
    let nonce = random_bytes(NONCE_LEN)?;
    let key = crypto::derive_key(passphrase, &kdf)?;
    let key_check = crypto::key_check_value(&key);
    let ciphertext = compress::pack_payload(&key, &nonce, payload)?;

    let header = Header {
        header_schema_ver: 1,
        min_reader: 1,
        case_id: params.case_id.clone(),
        bundle_hash: params.bundle_hash.clone(),
        kdf,
        aead: AeadParams {
            algo: "xchacha20poly1305".into(),
            nonce,
        },
        key_check,
        meta: params.meta.clone(),
    };
    write_container(params.kind, &header, &ciphertext)
}

/// Open a sealed FCB container to its kind, plaintext header, and decrypted +
/// decompressed payload bytes.
pub fn open_bytes(bytes: &[u8], passphrase: &str) -> Result<(BundleKind, Header, Vec<u8>)> {
    let container = read_container(bytes)?;
    let key = crypto::derive_key(passphrase, &container.header.kdf)?;
    let payload = compress::unpack_payload(
        &key,
        &container.header.key_check,
        &container.header.aead.nonce,
        &container.payload,
    )?;
    Ok((container.kind, container.header, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FcbError;

    fn fast_params(kind: BundleKind) -> BundleParams {
        // Low Argon2 cost keeps the test fast; real bundles use the defaults.
        let mut p = BundleParams::new(kind, "case-1", "hash-1", Value::Map(vec![]));
        p.m_cost = 32;
        p.t_cost = 1;
        p.p_cost = 1;
        p
    }

    #[test]
    fn pack_open_round_trips() {
        let params = fast_params(BundleKind::Case);
        let bytes = pack_bytes(&params, b"some evidence payload", "hunter2").unwrap();
        let (kind, header, payload) = open_bytes(&bytes, "hunter2").unwrap();
        assert_eq!(kind, BundleKind::Case);
        assert_eq!(header.case_id, "case-1");
        assert_eq!(payload, b"some evidence payload");
    }

    #[test]
    fn wrong_passphrase_rejected_at_bundle_level() {
        let params = fast_params(BundleKind::Case);
        let bytes = pack_bytes(&params, b"x", "hunter2").unwrap();
        assert_eq!(
            open_bytes(&bytes, "nope").unwrap_err(),
            FcbError::WrongPassphrase
        );
    }
}
