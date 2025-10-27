//! Status returned by behavior nodes.

/// The result of evaluating a behavior node.
///
/// # Turn-based Semantics
///
/// In a turn-based game, every action completes within a single tick:
/// - Conditions evaluate immediately (e.g., "Is enemy adjacent?")
/// - Actions execute atomically (e.g., "Move north")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// The behavior completed successfully.
    ///
    /// For conditions: The condition was met.
    /// For actions: The action executed without errors.
    Success,

    /// The behavior failed.
    ///
    /// For conditions: The condition was not met.
    /// For actions: The action could not be executed (e.g., invalid move).
    Failure,
}

impl Status {
    /// Returns `true` if this status is `Success`.
    #[inline]
    pub fn is_success(self) -> bool {
        matches!(self, Status::Success)
    }

    /// Returns `true` if this status is `Failure`.
    #[inline]
    pub fn is_failure(self) -> bool {
        matches!(self, Status::Failure)
    }

    /// Inverts the status: Success becomes Failure and vice versa.
    ///
    /// This is useful for implementing negation logic.
    #[inline]
    pub fn invert(self) -> Self {
        match self {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
        }
    }
}
