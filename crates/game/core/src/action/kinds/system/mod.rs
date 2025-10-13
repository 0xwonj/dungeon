//! System-level actions that maintain game invariants.
//!
//! System actions are executed by [`EntityId::SYSTEM`] and represent deterministic
//! state transitions that are not initiated by in-game entities. These include:
//!
//! - Turn scheduling and actor selection
//! - Action cost application and cooldown management
//! - Entity activation/deactivation based on game rules
//!
//! All system actions implement [`ActionTransition`] and follow the same three-phase
//! validation pipeline as player/NPC actions, ensuring they are fully auditable and
//! provable in zero-knowledge proof systems.

mod activation;
mod prepare_turn;
mod action_cost;

pub use activation::ActivationAction;
pub use prepare_turn::PrepareTurnAction;
pub use action_cost::ActionCostAction;
