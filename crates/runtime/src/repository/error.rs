//! Error types raised by repository implementations.
use thiserror::Error;

/// Errors surfaced by repository implementations.
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("state repository lock was poisoned")]
    LockPoisoned,
}
