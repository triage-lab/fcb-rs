//! Compression and the compress-then-encrypt payload pipeline.
//!
//! Order matters: zstd first, AEAD second. Ciphertext is ~random and does not
//! compress, so encrypt-then-compress would save nothing. Compressing first
//! shrinks the highly repetitive log/evidence data, *then* we seal it.
//!
//! Backend: the **pure-Rust** `ruzstd` codec (no C/FFI), so the exact same
//! crate compiles to native *and* `wasm32-unknown-unknown`. It emits standard
//! zstd frames, so a native C-zstd reader (authoring CLI, teacher platform)
//! interoperates with browser-produced bundles and vice versa.

use std::io::Read;

use ruzstd::decoding::StreamingDecoder;
use ruzstd::encoding::{compress_to_vec, CompressionLevel};

use crate::crypto::{self, KEY_LEN};
use crate::error::{FcbError, Result};

/// Magic bytes at the start of every zstd frame (`0x28 0xB5 0x2F 0xFD`).
pub const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Compress with zstd (pure-Rust backend), emitting a standard zstd frame.
///
/// `ruzstd` 0.8 implements `Fastest` (and `Uncompressed`); higher levels are
/// not yet available. `Fastest` already gives strong ratios on repetitive log
/// data, and the frame is upgradeable to higher levels with no format change
/// once the backend implements them.
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    Ok(compress_to_vec(data, CompressionLevel::Fastest))
}

/// Decompress a zstd frame. A malformed frame is treated as corruption.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = StreamingDecoder::new(data).map_err(|_| FcbError::Corrupt)?;
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|_| FcbError::Corrupt)?;
    Ok(out)
}

/// Pack a payload: **compress, then encrypt**.
pub fn pack_payload(key: &[u8; KEY_LEN], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let compressed = compress(plaintext)?;
    crypto::seal(key, nonce, &compressed)
}

/// Unpack a payload: **decrypt, then decompress** (reverse order).
pub fn unpack_payload(
    key: &[u8; KEY_LEN],
    expected_kcv: &[u8],
    nonce: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    let compressed = crypto::open_payload(key, expected_kcv, nonce, ciphertext)?;
    decompress(&compressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::KdfParams;
    use crate::crypto::{derive_key, key_check_value, open, NONCE_LEN};

    fn key_kcv() -> ([u8; KEY_LEN], Vec<u8>) {
        let kdf = KdfParams {
            algo: "argon2id".into(),
            salt: b"salt-cccc".to_vec(),
            m_cost: 32,
            t_cost: 1,
            p_cost: 1,
        };
        let key = derive_key("hunter2", &kdf).unwrap();
        let kcv = key_check_value(&key);
        (key, kcv)
    }

    #[test]
    fn payload_round_trips() {
        let (key, kcv) = key_kcv();
        let nonce = [3u8; NONCE_LEN];
        // Repetitive data so compression actually does something.
        let data = b"login failed login failed login failed login failed".repeat(20);
        let packed = pack_payload(&key, &nonce, &data).unwrap();
        let out = unpack_payload(&key, &kcv, &nonce, &packed).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn order_is_compress_then_encrypt() {
        let (key, _kcv) = key_kcv();
        let nonce = [3u8; NONCE_LEN];
        let data = b"alpha beta alpha beta alpha beta".repeat(10);

        let compressed = compress(&data).unwrap();
        let packed = pack_payload(&key, &nonce, &data).unwrap();

        // Decrypting the packed payload yields exactly the zstd frame, proving
        // encryption wraps already-compressed bytes.
        let decrypted = open(&key, &nonce, &packed).unwrap();
        assert_eq!(decrypted, compressed);
        // The inner bytes are a real zstd frame...
        assert_eq!(&compressed[0..4], &ZSTD_MAGIC);
        // ...and the outer (encrypted) bytes are NOT a bare zstd frame.
        assert_ne!(&packed[0..4], &ZSTD_MAGIC);
    }
}
