//! Top-level codec: ties container framing, crypto, and compression into
//! `pack`/`open` for whole bundles. This is the single place that generates a
//! fresh salt + nonce, derives the key, and assembles the plaintext header.

use ciborium::value::Value;

use crate::compress;
use crate::container::{
    encode_prefix, read_container, AeadParams, BundleKind, Header, KdfParams,
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

    let header = Header {
        header_schema_ver: 1,
        // v2: the plaintext header is now AEAD-authenticated (bound as AAD); a
        // pre-AAD v1 reader must refuse rather than misopen.
        min_reader: 2,
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
    // Bind the whole plaintext prefix (magic, KIND, version, hdr_len, header) as
    // AEAD additional authenticated data, then append the sealed payload to it.
    let prefix = encode_prefix(params.kind, &header)?;
    let ciphertext = compress::pack_payload(&key, &header.aead.nonce, payload, &prefix)?;
    let mut out = prefix;
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Open a sealed FCB container to its kind, plaintext header, and decrypted +
/// decompressed payload bytes. The plaintext prefix preceding the payload is the
/// AEAD additional authenticated data, so any header tamper fails as `Corrupt`.
pub fn open_bytes(bytes: &[u8], passphrase: &str) -> Result<(BundleKind, Header, Vec<u8>)> {
    let container = read_container(bytes)?;
    // The AAD is exactly the bytes preceding the payload, i.e. everything the
    // pack path bound via encode_prefix. read_container hands back the payload
    // as the tail slice, so the prefix length is the remainder.
    let prefix_len = bytes.len() - container.payload.len();
    let aad = &bytes[..prefix_len];
    let key = crypto::derive_key(passphrase, &container.header.kdf)?;
    let payload = compress::unpack_payload(
        &key,
        &container.header.key_check,
        &container.header.aead.nonce,
        &container.payload,
        aad,
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
    fn pack_generates_fresh_salt_and_nonce() {
        // Salt and nonce are random per pack: same params + payload + passphrase
        // must still produce different sealed bytes. Nonce reuse would be a
        // confidentiality break, so this guards against an accidental switch to
        // fixed randomness.
        let params = fast_params(BundleKind::Case);
        let a = pack_bytes(&params, b"same payload", "pw").unwrap();
        let b = pack_bytes(&params, b"same payload", "pw").unwrap();
        assert_ne!(a, b, "salt/nonce must be fresh per pack");
        assert_eq!(open_bytes(&a, "pw").unwrap().2, b"same payload");
        assert_eq!(open_bytes(&b, "pw").unwrap().2, b"same payload");
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

    #[test]
    fn header_tamper_is_corrupt() {
        // The plaintext header is bound as AAD. Flip a byte of a header field
        // value (the case_id string) — the header still parses and the
        // passphrase is right, but the AAD no longer matches the sealed tag, so
        // the open fails as Corrupt rather than returning tampered data.
        let params = fast_params(BundleKind::Case);
        let good = pack_bytes(&params, b"some evidence payload", "pw").unwrap();
        // Control: the untampered bundle opens cleanly. Without this, a Corrupt
        // below could merely mean AAD binding is broken for *every* bundle; the
        // control proves the failure is specifically due to the header byte.
        assert!(open_bytes(&good, "pw").is_ok());

        let mut bytes = good.clone();
        let needle = b"case-1";
        let pos = bytes
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("case_id appears in the plaintext header");
        bytes[pos] ^= 0x01;
        assert_eq!(open_bytes(&bytes, "pw").unwrap_err(), FcbError::Corrupt);
    }

    #[test]
    fn packs_declare_min_reader_2() {
        // New bundles require an AAD-aware (v2) reader.
        let params = fast_params(BundleKind::Case);
        let bytes = pack_bytes(&params, b"x", "pw").unwrap();
        let (_kind, header, _payload) = open_bytes(&bytes, "pw").unwrap();
        assert_eq!(header.min_reader, 2);
    }
}
