//! Small CBOR helpers shared by the evidence, task, and bundle layers.
//!
//! The plaintext header keeps `meta` as an opaque [`ciborium::value::Value`] so
//! the container layer stays decoupled from the evidence/task models. These
//! helpers convert typed structs to/from that opaque value, and serialize the
//! encrypted payload.

use ciborium::value::Value;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::{FcbError, Result};

/// Serialize a typed value into an opaque CBOR `Value` (for `header.meta`).
pub fn to_value<T: Serialize>(t: &T) -> Result<Value> {
    let mut buf = Vec::new();
    ciborium::into_writer(t, &mut buf)
        .map_err(|e| FcbError::Malformed(format!("cbor encode: {e}")))?;
    ciborium::from_reader(&buf[..]).map_err(|e| FcbError::Malformed(format!("cbor to value: {e}")))
}

/// Interpret an opaque CBOR `Value` (from `header.meta`) as a typed struct.
pub fn from_value<T: DeserializeOwned>(v: &Value) -> Result<T> {
    let mut buf = Vec::new();
    ciborium::into_writer(v, &mut buf)
        .map_err(|e| FcbError::Malformed(format!("cbor encode value: {e}")))?;
    ciborium::from_reader(&buf[..]).map_err(|e| FcbError::Malformed(format!("cbor decode: {e}")))
}

/// Serialize a value to CBOR bytes (for the encrypted payload).
pub fn encode<T: Serialize>(t: &T) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::into_writer(t, &mut buf)
        .map_err(|e| FcbError::Malformed(format!("cbor encode: {e}")))?;
    Ok(buf)
}

/// Deserialize CBOR bytes from a (decrypted, decompressed) payload.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    ciborium::from_reader(bytes).map_err(|_| FcbError::Corrupt)
}
