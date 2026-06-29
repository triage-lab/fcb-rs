//! FCB container framing: the on-disk envelope shared by `.case` and
//! `.casework`.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! magic(4) | KIND(u8) | container_version(u16) | hdr_len(u32)
//!          | header (hdr_len bytes, plaintext CBOR)
//!          | payload (rest; AEAD(zstd(serialized)) once crypto lands)
//! ```
//!
//! The header is plaintext on purpose: the KDF salt/params and AEAD nonce must
//! be read *before* a key exists, and they are not secrets.

use serde::{Deserialize, Serialize};

use crate::error::{FcbError, Result};

/// Constant magic. Byte `0x89` (like PNG) makes accidental text collisions
/// near-impossible and flags 7/8-bit transport damage. The version is a
/// separate field — it is never encoded into the magic.
pub const MAGIC: [u8; 4] = [0x89, b'F', b'C', b'B'];

/// Container layout version this codec writes.
pub const CONTAINER_VERSION: u16 = 1;

/// Highest `min_reader` this codec can satisfy. A bundle demanding more is
/// refused gracefully rather than misparsed. Bumped to 2 when the plaintext
/// header became AEAD-authenticated (bound as additional authenticated data):
/// a v1 reader has no AAD step and would fail to open v2 bundles, so v2 bundles
/// declare `min_reader = 2` to make pre-AAD readers refuse them gracefully.
pub const READER_VERSION: u16 = 2;

/// Distinguishes a challenge bundle from a student submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleKind {
    /// `.case` — teacher-issued challenge (evidence + task spec).
    Case,
    /// `.casework` — student submission (notes/report/activity).
    Work,
}

impl BundleKind {
    fn to_u8(self) -> u8 {
        match self {
            BundleKind::Case => 1,
            BundleKind::Work => 2,
        }
    }

    fn from_u8(b: u8) -> Result<Self> {
        match b {
            1 => Ok(BundleKind::Case),
            2 => Ok(BundleKind::Work),
            other => Err(FcbError::Malformed(format!("unknown KIND byte {other}"))),
        }
    }
}

/// Passphrase KDF parameters (plaintext; not secret).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// KDF identifier, e.g. `argon2id`.
    pub algo: String,
    /// Per-bundle random salt.
    pub salt: Vec<u8>,
    /// Argon2 memory cost (KiB).
    pub m_cost: u32,
    /// Argon2 time cost (iterations).
    pub t_cost: u32,
    /// Argon2 parallelism.
    pub p_cost: u32,
}

/// AEAD parameters (plaintext; nonce is not secret).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AeadParams {
    /// AEAD identifier, e.g. `xchacha20poly1305`.
    pub algo: String,
    /// Per-bundle random nonce.
    pub nonce: Vec<u8>,
}

/// The plaintext header. `meta` stays opaque CBOR here so the container layer
/// does not depend on the evidence/task models; those layers interpret it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    /// Structure version of this header object.
    pub header_schema_ver: u16,
    /// Minimum reader version required to open this bundle.
    pub min_reader: u16,
    /// Stable, teacher-assigned challenge identity.
    pub case_id: String,
    /// Content hash binding work to a specific evidence version.
    pub bundle_hash: String,
    pub kdf: KdfParams,
    pub aead: AeadParams,
    /// Non-secret key-check value: lets the reader tell a wrong passphrase
    /// (KCV mismatch) from a tampered payload (KCV match, AEAD fails). See
    /// `crate::crypto`.
    pub key_check: Vec<u8>,
    /// Opaque metadata: `streams[]` for a case, work refs, or task spec.
    pub meta: ciborium::value::Value,
}

/// A parsed container frame. `payload` is still compressed+encrypted here.
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub kind: BundleKind,
    pub container_version: u16,
    pub header: Header,
    pub payload: Vec<u8>,
}

fn read_u16(b: &[u8], pos: &mut usize) -> Result<u16> {
    let s = b
        .get(*pos..*pos + 2)
        .ok_or_else(|| FcbError::Malformed("truncated u16".into()))?;
    *pos += 2;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

fn read_u32(b: &[u8], pos: &mut usize) -> Result<u32> {
    let s = b
        .get(*pos..*pos + 4)
        .ok_or_else(|| FcbError::Malformed("truncated u32".into()))?;
    *pos += 4;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Slice the `hdr_len`-byte header beginning at `pos`, computing the end offset
/// with overflow-safe arithmetic. `hdr_len` is derived from an untrusted `u32`
/// length prefix; on a 32-bit target (e.g. `wasm32`) a value near `u32::MAX`
/// makes `pos + hdr_len` overflow `usize`, which panics in a debug build (the
/// reachable defect — an abort/DoS). In release it wraps, but because `pos` is
/// the small fixed-prefix offset the wrapped end lands below `pos`, so the
/// backwards `bytes.get(pos..end)` is already rejected as `None` today.
/// `checked_add` turns any overflowing or out-of-bounds length into a
/// `Malformed` rejection on every target — panic-free, and robust even if
/// `pos` were ever large — without changing the result for a well-formed
/// container.
fn header_slice(bytes: &[u8], pos: usize, hdr_len: usize) -> Result<&[u8]> {
    pos.checked_add(hdr_len)
        .and_then(|end| bytes.get(pos..end))
        .ok_or_else(|| FcbError::Malformed("header length out of bounds".into()))
}

/// Serialize a header to its plaintext CBOR bytes.
pub fn encode_header(header: &Header) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::into_writer(header, &mut buf)
        .map_err(|e| FcbError::Malformed(format!("encode header: {e}")))?;
    Ok(buf)
}

/// Serialize the plaintext container prefix that precedes the payload: magic,
/// KIND, `container_version`, `hdr_len`, and the header CBOR. This is exactly
/// the byte range bound as the AEAD additional authenticated data, so the pack
/// path seals against these bytes and the open path reconstructs the identical
/// slice from what it read — any tamper of magic, KIND, version, or any header
/// field changes the AAD and fails authentication.
pub fn encode_prefix(kind: BundleKind, header: &Header) -> Result<Vec<u8>> {
    let hdr = encode_header(header)?;
    let mut out = Vec::with_capacity(4 + 1 + 2 + 4 + hdr.len());
    out.extend_from_slice(&MAGIC);
    out.push(kind.to_u8());
    out.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    out.extend_from_slice(&(hdr.len() as u32).to_le_bytes());
    out.extend_from_slice(&hdr);
    Ok(out)
}

/// Frame a container into its on-disk bytes: the [`encode_prefix`] bytes
/// followed by `payload` verbatim.
pub fn write_container(kind: BundleKind, header: &Header, payload: &[u8]) -> Result<Vec<u8>> {
    let mut out = encode_prefix(kind, header)?;
    out.extend_from_slice(payload);
    Ok(out)
}

/// Read just the plaintext header without copying the payload — usable before
/// any passphrase is supplied. Enforces magic, KIND, and `min_reader`.
pub fn peek_header(bytes: &[u8]) -> Result<Header> {
    if bytes.len() < 4 || bytes[0..4] != MAGIC {
        return Err(FcbError::BadMagic);
    }
    let mut pos = 4;
    let _kind = BundleKind::from_u8(
        *bytes
            .get(pos)
            .ok_or_else(|| FcbError::Malformed("missing KIND".into()))?,
    )?;
    pos += 1;
    let _container_version = read_u16(bytes, &mut pos)?;
    let hdr_len = read_u32(bytes, &mut pos)? as usize;
    let header_bytes = header_slice(bytes, pos, hdr_len)?;
    let header: Header = ciborium::from_reader(header_bytes)
        .map_err(|e| FcbError::Malformed(format!("bad header CBOR: {e}")))?;
    if header.min_reader > READER_VERSION {
        return Err(FcbError::UnsupportedVersion {
            min_reader: header.min_reader,
            supported: READER_VERSION,
        });
    }
    Ok(header)
}

/// Parse a full container frame. Dispatches by `container_version` and refuses
/// (rather than misparses) when `min_reader` exceeds this reader.
pub fn read_container(bytes: &[u8]) -> Result<Container> {
    if bytes.len() < 4 || bytes[0..4] != MAGIC {
        return Err(FcbError::BadMagic);
    }
    let mut pos = 4;
    let kind = BundleKind::from_u8(
        *bytes
            .get(pos)
            .ok_or_else(|| FcbError::Malformed("missing KIND".into()))?,
    )?;
    pos += 1;
    let container_version = read_u16(bytes, &mut pos)?;
    let hdr_len = read_u32(bytes, &mut pos)? as usize;
    let header_bytes = header_slice(bytes, pos, hdr_len)?;
    pos += hdr_len;
    let header: Header = ciborium::from_reader(header_bytes)
        .map_err(|e| FcbError::Malformed(format!("bad header CBOR: {e}")))?;
    if header.min_reader > READER_VERSION {
        return Err(FcbError::UnsupportedVersion {
            min_reader: header.min_reader,
            supported: READER_VERSION,
        });
    }
    // container_version is reserved for future parse-path dispatch; v1 is the
    // only known layout today.
    let payload = bytes[pos..].to_vec();
    Ok(Container {
        kind,
        container_version,
        header,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_header(min_reader: u16) -> Header {
        Header {
            header_schema_ver: 1,
            min_reader,
            case_id: "acme-ir-2026-03".into(),
            bundle_hash: "b3:9f2c".into(),
            kdf: KdfParams {
                algo: "argon2id".into(),
                salt: vec![1, 2, 3, 4],
                m_cost: 19456,
                t_cost: 2,
                p_cost: 1,
            },
            aead: AeadParams {
                algo: "xchacha20poly1305".into(),
                nonce: vec![9; 24],
            },
            key_check: vec![0; 32],
            meta: ciborium::value::Value::Map(vec![]),
        }
    }

    #[test]
    fn roundtrip_preserves_kind_version_and_header() {
        let header = sample_header(1);
        let bytes = write_container(BundleKind::Case, &header, b"payload-bytes").unwrap();
        let c = read_container(&bytes).unwrap();
        assert_eq!(c.kind, BundleKind::Case);
        assert_eq!(c.container_version, CONTAINER_VERSION);
        assert_eq!(c.header, header);
        assert_eq!(c.payload, b"payload-bytes");
    }

    #[test]
    fn kind_is_distinguished() {
        let header = sample_header(1);
        let case = write_container(BundleKind::Case, &header, b"x").unwrap();
        let work = write_container(BundleKind::Work, &header, b"x").unwrap();
        assert_eq!(read_container(&case).unwrap().kind, BundleKind::Case);
        assert_eq!(read_container(&work).unwrap().kind, BundleKind::Work);
    }

    #[test]
    fn non_fcb_prefix_is_bad_magic() {
        assert_eq!(
            read_container(b"\x00ELF not fcb").unwrap_err(),
            FcbError::BadMagic
        );
        assert_eq!(
            read_container(b"PK\x03\x04zip").unwrap_err(),
            FcbError::BadMagic
        );
        assert_eq!(read_container(b"ab").unwrap_err(), FcbError::BadMagic);
    }

    #[test]
    fn reader_version_is_two_for_aad_format() {
        // The AAD-authenticated format is reader version 2. Guards against an
        // accidental downgrade that would silently change the min_reader gate.
        assert_eq!(READER_VERSION, 2);
    }

    #[test]
    fn min_reader_too_new_is_unsupported() {
        let header = sample_header(READER_VERSION + 1);
        let bytes = write_container(BundleKind::Case, &header, b"x").unwrap();
        assert_eq!(
            read_container(&bytes).unwrap_err(),
            FcbError::UnsupportedVersion {
                min_reader: READER_VERSION + 1,
                supported: READER_VERSION,
            }
        );
    }

    #[test]
    fn header_readable_before_decryption() {
        // peek_header reads case_id from plaintext without touching payload.
        let header = sample_header(1);
        let bytes = write_container(BundleKind::Case, &header, b"opaque-encrypted").unwrap();
        let peeked = peek_header(&bytes).unwrap();
        assert_eq!(peeked.case_id, "acme-ir-2026-03");
        assert_eq!(peeked.kdf.salt, vec![1, 2, 3, 4]);
        assert_eq!(peeked.aead.nonce.len(), 24);
    }

    #[test]
    fn truncated_header_is_malformed() {
        let header = sample_header(1);
        let mut bytes = write_container(BundleKind::Case, &header, b"x").unwrap();
        bytes.truncate(9); // keep magic+kind+ver+partial hdr_len region
        assert!(matches!(
            read_container(&bytes),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn header_slice_rejects_overflowing_offset() {
        // `hdr_len` comes from an untrusted u32. On a 32-bit target a value near
        // u32::MAX makes `pos + hdr_len` overflow usize; we force that exact
        // overflow branch on any target by passing pos = usize::MAX. A naive
        // `pos + hdr_len` panics in a debug build — header_slice must instead
        // reject with Malformed and never panic.
        assert!(matches!(
            header_slice(&[], usize::MAX, 1),
            Err(FcbError::Malformed(_))
        ));
    }

    #[test]
    fn oversized_hdr_len_is_malformed() {
        // A container that declares hdr_len = u32::MAX but carries only a few
        // header bytes must be rejected as Malformed (not panic) on every
        // target, including 32-bit where pos + hdr_len would overflow usize.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAGIC);
        bytes.push(BundleKind::Case.to_u8());
        bytes.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(b"short");
        assert!(matches!(
            read_container(&bytes),
            Err(FcbError::Malformed(_))
        ));
        assert!(matches!(peek_header(&bytes), Err(FcbError::Malformed(_))));
    }
}
