use thiserror::Error;

/// Errors surfaced by the FCB codec.
///
/// `WrongPassphrase` and `Corrupt` are deliberately distinct so a caller can
/// tell "the password was wrong" from "the file was tampered with"; both come
/// from AEAD verification but mean different things to a student/operator.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FcbError {
    /// First bytes are not the FCB magic.
    #[error("not an FCB container (bad magic)")]
    BadMagic,

    /// The bundle's `min_reader` is newer than this reader supports.
    #[error("unsupported FCB version: bundle requires reader >= {min_reader}, this reader supports {supported}")]
    UnsupportedVersion { min_reader: u16, supported: u16 },

    /// Structurally invalid container (bad length prefix, bad CBOR, etc.).
    #[error("malformed FCB container: {0}")]
    Malformed(String),

    /// AEAD verification failed and the inputs are consistent with a wrong key.
    #[error("wrong passphrase")]
    WrongPassphrase,

    /// AEAD verification failed on otherwise well-formed input (tampering).
    #[error("corrupt or tampered bundle")]
    Corrupt,
}

/// Convenience alias used throughout the crate.
pub type Result<T> = core::result::Result<T, FcbError>;
