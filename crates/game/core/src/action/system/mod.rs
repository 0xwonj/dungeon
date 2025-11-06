//! System-level actions that maintain game invariants.
//!
//! System actions are executed by [`EntityId::SYSTEM`] and represent deterministic
//! state transitions that are not initiated by in-game entities. These include:
//!
//! - Turn scheduling and actor selection
//! - Entity activation/deactivation based on game rules
//! - Entity removal from world and turn scheduling
//!
//! Note: Action cost application is now handled directly within character action
//! execution (see `action::execute::pipeline`) to avoid double validation overhead.
//!
//! All system actions implement [`ActionTransition`] and follow the same three-phase
//! validation pipeline as player/NPC actions, ensuring they are fully auditable and
//! provable in zero-knowledge proof systems.

mod activation;
mod deactivate;
mod prepare_turn;
mod remove_from_world;

pub use activation::ActivationAction;
pub use deactivate::DeactivateAction;
pub use prepare_turn::PrepareTurnAction;
pub use remove_from_world::RemoveFromWorldAction;
