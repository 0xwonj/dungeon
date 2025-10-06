/// Runtime errors
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// No active entities available for turn scheduling
    NoActiveEntities,
    /// Action execution failed
    ExecuteFailed(String),
    /// Repository operation failed
    RepositoryError(String),
    /// Oracle query failed
    OracleError(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
