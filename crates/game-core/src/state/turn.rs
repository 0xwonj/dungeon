/// Turn index plus intra-turn cursor to keep the phase explicit.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TurnState {
    pub index: u64,
    pub phase: TurnPhase,
}

impl TurnState {
    pub fn new(index: u64, phase: TurnPhase) -> Self {
        Self { index, phase }
    }
}

/// Phases within a single turn.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TurnPhase {
    /// Player is selecting/executing their action.
    #[default]
    Player,
    /// NPCs are resolving in a stable order; `cursor` tracks which index is next.
    Npc { cursor: usize },
    /// End-of-turn cleanup (statuses, hazards, cooldown ticks).
    EndOfTurn,
}
