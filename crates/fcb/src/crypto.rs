//! Passphrase-based cryptography for FCB payloads.
//!
//! `passphrase --Argon2id--> key --XChaCha20-Poly1305--> sealed payload`.
//!
//! AEAD alone cannot say *why* it failed, but callers need to tell "wrong
//! password" from "tampered file". We store a non-secret **key-check value**
//! (KCV = domain-separated hash of the derived key) in the plaintext header.
//! On open: KCV mismatch ⇒ [`FcbError::WrongPassphrase`]; KCV match but AEAD
//! failure ⇒ [`FcbError::Corrupt`]. Argon2id remains the brute-force barrier,
//! so publishing a hash of the key does not meaningfully help an attacker.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use sha2::{Digest, Sha256};

use crate::container::KdfParams;
use crate::error::{FcbError, Result};

/// Derived symmetric key length (XChaCha20-Poly1305 key size).
pub const KEY_LEN: usize = 32;
/// XChaCha20-Poly1305 nonce length.
pub const NONCE_LEN: usize = 24;

const KCV_DOMAIN: &[u8] = b"FCB-key-check-v1";

/// Derive the symmetric key from a passphrase and the header's KDF params.
pub fn derive_key(passphrase: &str, kdf: &KdfParams) -> Result<[u8; KEY_LEN]> {
    if kdf.algo != "argon2id" {
        return Err(FcbError::Malformed(format!(
            "unsupported KDF: {}",
            kdf.algo
        )));
    }
    let params = Params::new(kdf.m_cost, kdf.t_cost, kdf.p_cost, Some(KEY_LEN))
        .map_err(|e| FcbError::Malformed(format!("bad argon2 params: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    argon
        .hash_password_into(passphrase.as_bytes(), &kdf.salt, &mut key)
        .map_err(|e| FcbError::Malformed(format!("argon2 failure: {e}")))?;
    Ok(key)
}

/// Compute the non-secret key-check value stored in the header.
pub fn key_check_value(key: &[u8; KEY_LEN]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(KCV_DOMAIN);
    h.update(key);
    h.finalize().to_vec()
}

fn cipher_for(key: &[u8; KEY_LEN]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(Key::from_slice(key))
}

fn nonce_from(nonce: &[u8]) -> Result<&XNonce> {
    if nonce.len() != NONCE_LEN {
        return Err(FcbError::Malformed(format!(
            "nonce must be {NONCE_LEN} bytes, got {}",
            nonce.len()
        )));
    }
    Ok(XNonce::from_slice(nonce))
}

/// Seal plaintext with the key and nonce. Used by the pack path.
pub fn seal(key: &[u8; KEY_LEN], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let nonce = nonce_from(nonce)?;
    cipher_for(key)
        .encrypt(nonce, plaintext)
        .map_err(|_| FcbError::Malformed("AEAD encryption failure".into()))
}

/// Raw AEAD open. Any verification failure maps to [`FcbError::Corrupt`];
/// prefer [`open_payload`] when a KCV is available to distinguish wrong keys.
pub fn open(key: &[u8; KEY_LEN], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let nonce = nonce_from(nonce)?;
    cipher_for(key)
        .decrypt(nonce, ciphertext)
        .map_err(|_| FcbError::Corrupt)
}

/// Open a payload, distinguishing a wrong passphrase from tampering using the
/// header's key-check value.
pub fn open_payload(
    key: &[u8; KEY_LEN],
    expected_kcv: &[u8],
    nonce: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    if !ct_eq(&key_check_value(key), expected_kcv) {
        return Err(FcbError::WrongPassphrase);
    }
    open(key, nonce, ciphertext)
}

/// Constant-time byte comparison (avoids leaking via early-exit timing).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Small Argon2 cost so tests stay fast; production params live in headers.
    fn test_kdf(salt: &[u8]) -> KdfParams {
        KdfParams {
            algo: "argon2id".into(),
            salt: salt.to_vec(),
            m_cost: 32,
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn correct_passphrase_round_trips() {
        let kdf = test_kdf(b"salt-aaaa");
        let key = derive_key("hunter2", &kdf).unwrap();
        let kcv = key_check_value(&key);
        let nonce = [7u8; NONCE_LEN];
        let ct = seal(&key, &nonce, b"top secret evidence").unwrap();

        let pt = open_payload(&key, &kcv, &nonce, &ct).unwrap();
        assert_eq!(pt, b"top secret evidence");
    }

    #[test]
    fn wrong_passphrase_is_wrong_passphrase() {
        let kdf = test_kdf(b"salt-aaaa");
        let key = derive_key("hunter2", &kdf).unwrap();
        let kcv = key_check_value(&key);
        let nonce = [7u8; NONCE_LEN];
        let ct = seal(&key, &nonce, b"top secret evidence").unwrap();

        // Same salt, different passphrase -> different key -> KCV mismatch.
        let wrong = derive_key("password", &kdf).unwrap();
        assert_eq!(
            open_payload(&wrong, &kcv, &nonce, &ct).unwrap_err(),
            FcbError::WrongPassphrase
        );
    }

    #[test]
    fn tampered_ciphertext_is_corrupt() {
        let kdf = test_kdf(b"salt-bbbb");
        let key = derive_key("hunter2", &kdf).unwrap();
        let kcv = key_check_value(&key);
        let nonce = [7u8; NONCE_LEN];
        let mut ct = seal(&key, &nonce, b"top secret evidence").unwrap();

        // Right key (KCV passes) but a flipped byte -> AEAD fails -> Corrupt.
        ct[0] ^= 0x01;
        assert_eq!(
            open_payload(&key, &kcv, &nonce, &ct).unwrap_err(),
            FcbError::Corrupt
        );
    }

    #[test]
    fn bad_nonce_length_is_malformed() {
        let key = [0u8; KEY_LEN];
        assert!(matches!(
            seal(&key, &[0u8; 12], b"x"),
            Err(FcbError::Malformed(_))
        ));
    }
}
