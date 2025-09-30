use crate::state::EntityId;

/// Offensive action against a target entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackAction {
    pub target: EntityId,
    pub style: AttackStyle,
}

impl AttackAction {
    pub fn new(target: EntityId, style: AttackStyle) -> Self {
        Self { target, style }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttackStyle {
    Melee,
}
