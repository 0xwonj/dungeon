//! Provider types for action generation.
//!
//! While provider implementations live in the runtime crate, the provider KIND
//! is stored in game state to enable deterministic re-execution during optimistic
//! challenge verification.
//!
//! # Design Rationale
//!
//! In an optimistic rollup with challenge games, we need to prove:
//! > "Given this state and this provider, this action would be generated"
//!
//! To enable this verification, the provider kind must be part of the canonical
//! game state, not just runtime orchestration metadata.
//!
//! # Challenge Game Flow
//!
//! 1. **Optimistic execution**: Runtime uses provider to generate actions, submits proof
//! 2. **Challenge**: Someone disputes that the action came from the declared provider
//! 3. **Resolution**: zkVM re-executes the provider deterministically and proves expected action
//! 4. **Fraud detection**: If expected ≠ submitted, slash malicious player

use core::fmt;

/// Provider kind for action generation.
///
/// This enum identifies which type of provider should generate actions for an actor.
/// The actual provider implementation lives in the runtime crate, but the KIND is
/// stored in ActorState for challenge verification.
///
/// # Nested Design
///
/// This nested enum design provides clear separation between:
/// - Interactive sources (human players, network clients, replays)
/// - Automated AI decision makers (combat AI, passive behavior, etc.)
/// - Custom extensibility slots for user-defined providers
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProviderKind {
    /// Interactive input sources (human players, network clients, etc.)
    Interactive(InteractiveKind),

    /// Automated AI decision makers
    Ai(AiKind),

    /// Custom provider types (extensibility slot)
    Custom(u32),
}

impl ProviderKind {
    /// Returns true if this is an interactive provider (human-controlled).
    pub fn is_interactive(&self) -> bool {
        matches!(self, ProviderKind::Interactive(_))
    }

    /// Returns true if this is an AI provider (automated).
    pub fn is_ai(&self) -> bool {
        matches!(self, ProviderKind::Ai(_))
    }

    /// Returns true if this is a custom provider.
    pub fn is_custom(&self) -> bool {
        matches!(self, ProviderKind::Custom(_))
    }
}

/// Interactive input provider types.
///
/// These providers receive actions from external sources (human players, network, replays).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InteractiveKind {
    /// Local CLI keyboard input
    CliInput,

    /// Network/remote player input
    NetworkInput,

    /// Replayed actions from file/log
    Replay,
}

/// AI provider types for automated decision making.
///
/// These providers generate actions algorithmically based on game state.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AiKind {
    /// Simple wait-only AI (no-op, default for unmapped entities)
    Wait,

    /// Utility-based AI (goal-directed with utility scoring)
    ///
    /// Implementation: Goal selection → Generate all action candidates → Score by utility → Select best
    /// - Goal: High-level objective (Attack, Flee, Heal, Idle, etc.)
    /// - Candidates: All possible (Action, Input) pairs
    /// - Scoring: 0-100 utility score based on goal relevance
    /// - Selection: Highest scoring candidate wins
    Utility,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interactive(kind) => write!(f, "interactive/{}", kind),
            Self::Ai(kind) => write!(f, "ai/{}", kind),
            Self::Custom(id) => write!(f, "custom/{}", id),
        }
    }
}

impl fmt::Display for InteractiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::CliInput => "cli",
            Self::NetworkInput => "network",
            Self::Replay => "replay",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for AiKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Wait => "wait",
            Self::Utility => "utility",
        };
        write!(f, "{}", s)
    }
}
