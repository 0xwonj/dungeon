//! Hook that reacts to entities taking damage and triggers follow-up effects.

use game_core::{Action, ActionKind};

use super::{HookContext, HookCriticality, PostExecutionHook};

/// Entry hook for damage-related effect chains.
///
/// This hook does NOT apply damage (damage is already applied by Damage effects).
/// Instead, it detects when entities have taken damage and chains to follow-up hooks
/// like death checks, bleeding effects, damage reactions, etc.
///
/// # Role
///
/// DamageHook serves as the **entry point** for damage-related cascades:
/// - Detects damaged entities from the delta
/// - Chains to next hooks (death_check, bleeding, etc.) for each damaged entity
/// - Allows damage reactions to be modular and extensible
///
/// # Example Flow
///
/// ```text
/// Player attacks Goblin
///   → MeleeAttack action executes Damage effect (HP -= 10)
///   → DamageHook detects damage in delta
///   → Chains to death_check (checks if HP <= 0)
///   → Chains to bleeding_check (checks if bleeding should apply)
///   → Chains to damage_reaction (on-hit effects)
/// ```
///
/// # Multiple Entities
///
/// When multiple entities take damage (AoE attacks, explosions), this hook
/// detects all of them and triggers follow-up chains accordingly.
#[derive(Debug, Clone, Copy)]
pub struct DamageHook;

impl PostExecutionHook for DamageHook {
    fn name(&self) -> &'static str {
        "damage"
    }

    fn priority(&self) -> i32 {
        0 // Standard priority
    }

    fn criticality(&self) -> HookCriticality {
        // Important: DamageHook is an entry point for damage chains.
        // Failure means death checks and other follow-up effects won't trigger,
        // but the game state itself is still consistent (damage was already applied).
        HookCriticality::Important
    }

    fn should_trigger(&self, ctx: &HookContext<'_>) -> bool {
        // Trigger when an Attack action was executed
        // (damage was already applied during action execution)
        match &ctx.delta.action {
            Action::Character(action) => action.kind == ActionKind::MeleeAttack,
            Action::System { .. } => false,
        }

        // TODO: Later, check delta for HP changes:
        // ctx.delta.entities.actors.iter().any(|(_, patch)| patch.hp.is_some())
    }

    fn create_actions(&self, _ctx: &HookContext<'_>) -> Vec<Action> {
        // DamageHook doesn't create actions - it just serves as an entry point
        // for chaining to death_check, bleeding, etc.
        //
        // The damage itself was already applied by Damage effects.
        // This hook just detects "damage happened" and triggers the chain.
        vec![]
    }

    fn next_hook_names(&self) -> &[&'static str] {
        // Chain to follow-up damage effects
        &["death_check"]

        // Future: multiple chains
        // &["death_check", "bleeding_check", "damage_reaction"]
    }
}
