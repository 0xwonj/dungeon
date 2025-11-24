//! Error types raised by repository implementations.

use thiserror::Error;

/// Errors surfaced by repository implementations.
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("state repository lock was poisoned")]
    LockPoisoned,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("corrupted data: {0}")]
    CorruptedData(String),

    #[error("log already exists: {0}")]
    LogAlreadyExists(String),

    #[error("invalid offset {offset} for file size {file_size}")]
    InvalidOffset { offset: u64, file_size: u64 },

    #[error("partial write detected at offset {offset}: expected {expected} bytes, found {actual}")]
    PartialWrite {
        offset: u64,
        expected: usize,
        actual: usize,
    },
}

pub type Result<T> = std::result::Result<T, RepositoryError>;
