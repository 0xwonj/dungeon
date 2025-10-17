//! Action log entry type for proof generation.
//!
//! This module defines the `ActionLogEntry` type, which is the dedicated format
//! for storing executed actions in the actions.log file. This is separate from
//! the general event log to optimize proof generation.

use serde::{Deserialize, Serialize};

use game_core::{Action, GameState, StateDelta, Tick};

/// Action log entry for proof generation.
///
/// This struct contains all the data needed to generate a zero-knowledge proof
/// for a single executed action. It is stored in the actions.log file in a
/// compact, sequential format optimized for proof generation.
///
/// # Layout
///
/// Each entry is serialized using bincode and stored with a length prefix:
/// ```text
/// [u32 length][bincode serialized ActionLogEntry]
/// ```
///
/// # Example
///
/// ```rust,ignore
/// let entry = ActionLogEntry {
///     nonce: 5,
///     clock: 42,
///     action: player_move_action,
///     before_state: Box::new(state_before),
///     after_state: Box::new(state_after),
///     delta: Some(Box::new(state_delta)),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    /// Sequential action nonce
    pub nonce: u64,

    /// Game clock (turn number) when this action was executed
    pub clock: Tick,

    /// The action that was executed
    pub action: Action,

    /// Game state before the action was executed
    ///
    /// This is used by the zkVM to verify the state transition.
    pub before_state: Box<GameState>,

    /// Game state after the action was executed
    ///
    /// This is used by the zkVM to verify the state transition.
    pub after_state: Box<GameState>,

    /// State delta (changes made by the action)
    ///
    /// This is optional and can be used for optimization purposes.
    /// In the future, we might store only the delta and reconstruct
    /// the full states on demand.
    pub delta: Option<Box<StateDelta>>,
}

impl ActionLogEntry {
    /// Create a new action log entry.
    pub fn new(
        nonce: u64,
        clock: Tick,
        action: Action,
        before_state: GameState,
        after_state: GameState,
        delta: StateDelta,
    ) -> Self {
        Self {
            nonce,
            clock,
            action,
            before_state: Box::new(before_state),
            after_state: Box::new(after_state),
            delta: Some(Box::new(delta)),
        }
    }

    /// Create an action log entry without delta (minimal version).
    pub fn without_delta(
        nonce: u64,
        clock: Tick,
        action: Action,
        before_state: GameState,
        after_state: GameState,
    ) -> Self {
        Self {
            nonce,
            clock,
            action,
            before_state: Box::new(before_state),
            after_state: Box::new(after_state),
            delta: None,
        }
    }
}
