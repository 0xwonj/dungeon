//! Goal definition and selection logic.
//!
//! Goals are concrete objectives that drive NPC behavior.
//! Each goal represents a specific intent (e.g., "Attack Player", "Flee from Player").

use game_core::{EntityId, Position};

use super::AiContext;

/// A concrete goal that drives action selection.
///
/// Goals are specific and situation-dependent:
/// - **Attack { target }**: Engage a specific enemy
/// - **FleeFrom { threat }**: Escape from a specific danger
/// - **HealSelf**: Restore own HP
/// - **MoveTo { position }**: Navigate to a location
/// - **ProtectAlly { ally }**: Stay near and support an ally
/// - **Idle**: No specific objective
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Goal {
    /// Attack a specific entity.
    Attack { target: EntityId },

    /// Flee from a specific entity.
    FleeFrom { threat: EntityId },

    /// Heal self.
    HealSelf,

    /// Move towards a position (exploration).
    MoveTo { position: Position },

    /// Protect/stay near an ally.
    ProtectAlly { ally: EntityId },

    /// Do nothing (idle).
    Idle,
}

impl Goal {
    /// Evaluates how well an action candidate serves this goal.
    ///
    /// Returns a score from 0-100:
    /// - 100: Perfect match for this goal
    /// - 50-99: Helpful for this goal
    /// - 1-49: Somewhat relevant
    /// - 0: Not relevant or counterproductive
    pub fn evaluate_action(
        &self,
        kind: game_core::ActionKind,
        input: &game_core::ActionInput,
        ctx: &AiContext,
    ) -> u32 {
        use super::scoring;

        match self {
            Goal::Attack { target } => scoring::score_for_attack(kind, input, *target, ctx),
            Goal::FleeFrom { threat } => scoring::score_for_flee(kind, input, *threat, ctx),
            Goal::HealSelf => scoring::score_for_heal_self(kind, input, ctx),
            Goal::Idle => scoring::score_for_idle(kind, input, ctx),
            Goal::MoveTo { position } => scoring::score_for_move_to(kind, input, *position, ctx),
            Goal::ProtectAlly { ally } => scoring::score_for_protect_ally(kind, input, *ally, ctx),
        }
    }
}

/// Selects a goal based on current situation and NPC personality traits.
pub struct GoalSelector;

impl GoalSelector {
    /// Selects the most appropriate goal for the current situation.
    ///
    /// # Decision Process
    ///
    /// 1. **Critical Survival**: Low HP + immediate danger → Flee or Heal
    /// 2. **Combat**: Enemy visible + sufficient courage → Attack or Flee
    /// 3. **Exploration/Social**: No threats → Explore or interact
    /// 4. **Default**: Nothing to do → Idle
    ///
    /// # Personality Integration
    ///
    /// - **Bravery**: Affects fight vs flight threshold
    /// - **Aggression**: Influences attack initiative (TODO)
    /// - **Loyalty**: Prioritizes ally protection (TODO)
    /// - **Curiosity**: Drives exploration (TODO)
    pub fn select(ctx: &AiContext) -> Goal {
        let my_hp_percent = ctx.hp_ratio();
        let player_distance = ctx.distance_to_player();
        let can_see_player = ctx.can_see_player();

        // Get trait profile for personality-based decisions
        let trait_profile = ctx.trait_profile();

        tracing::debug!(
            "GoalSelector: entity={:?}, hp={}%, player_dist={}, visible={}",
            ctx.entity,
            my_hp_percent,
            player_distance,
            can_see_player
        );

        // ====================================================================
        // Priority 1: Critical Survival (Low HP + Immediate Danger)
        // ====================================================================

        if my_hp_percent < 30 {
            tracing::debug!("  Low HP detected ({}%)", my_hp_percent);

            // If player is very close and we're low HP, flee immediately
            if can_see_player && player_distance <= 5 {
                tracing::debug!("  → Goal: FleeFrom (critical survival)");
                return Goal::FleeFrom {
                    threat: EntityId::PLAYER,
                };
            }

            // If we have healing and are safe, heal
            // TODO: Check for healing items/abilities
            // if ctx.has_healing_item() {
            //     tracing::debug!("  → Goal: HealSelf (safe recovery)");
            //     return Goal::HealSelf;
            // }
        }

        // ====================================================================
        // Priority 2: Combat Decision (Player Visible)
        // ====================================================================

        if can_see_player {
            tracing::debug!("  Player visible at {} tiles", player_distance);

            // Get bravery trait (0-240, normalized to 0-100)
            let bravery = trait_profile
                .map(|p| {
                    let trait_value = p.get(game_content::traits::TraitKind::Bravery);
                    // Convert 0-240 range to 0-100 range
                    (trait_value as u32 * 100) / 240
                })
                .unwrap_or(50);

            // Combine HP and bravery to decide fight vs flight
            // High HP + High Bravery = Fight
            // Low HP + Low Bravery = Flight
            let courage_score = (my_hp_percent + bravery) / 2;

            tracing::debug!(
                "  Courage assessment: hp={}%, bravery={}, courage_score={}",
                my_hp_percent,
                bravery,
                courage_score
            );

            if courage_score > 50 {
                // Brave enough to fight
                tracing::debug!("  → Goal: Attack (courage_score > 50)");
                return Goal::Attack {
                    target: EntityId::PLAYER,
                };
            } else if player_distance <= 3 {
                // Not brave, and player is close - flee!
                tracing::debug!("  → Goal: FleeFrom (low courage + close enemy)");
                return Goal::FleeFrom {
                    threat: EntityId::PLAYER,
                };
            } else {
                // Not brave, but player is far - just stay away (idle for now)
                tracing::debug!("  → Goal: Idle (low courage but safe distance)");
                return Goal::Idle;
            }
        }

        // ====================================================================
        // Priority 3: Exploration/Social (No immediate threats)
        // ====================================================================

        // TODO: Implement exploration goals when map/patrol system exists
        // TODO: Implement social goals when ally system exists

        // ====================================================================
        // Default: Idle
        // ====================================================================

        tracing::debug!("  → Goal: Idle (no pressing concerns)");
        Goal::Idle
    }
}
