//! Utility-based AI action provider.

use async_trait::async_trait;
use game_core::{Action, CharacterActionKind, EntityId, GameEnv, GameState, WaitAction};

use super::scoring::actions::ActionScorer;
use super::scoring::selector::{IntentScorer, TacticScorer};
use crate::api::{ActionProvider, Result};
use crate::providers::ai::AiContext;

/// Utility-based AI provider using three-layer decision-making.
///
/// This provider implements intelligent NPC behavior using a structured
/// utility scoring approach:
///
/// 1. **Layer 1 (Intent)**: What does the NPC want to do?
/// 2. **Layer 2 (Tactic)**: How should they achieve it?
/// 3. **Layer 3 (Action)**: Which specific action to execute?
///
/// # Design
///
/// The provider:
/// 1. Gets available actions from game-core
/// 2. Builds an [`AiContext`] with game state
/// 3. Selects intent using [`IntentScorer`]
/// 4. Selects tactic using [`TacticScorer`]
/// 5. Selects action using [`ActionScorer`]
/// 6. Falls back to Wait if no valid action found
///
/// All scoring is deterministic and based on:
/// - Feasibility (can this be done?)
/// - Situation (is this favorable?)
/// - Personality (does this fit NPC traits?)
/// - Modifiers (contextual adjustments)
#[derive(Debug, Clone, Default)]
pub struct UtilityAiProvider;

impl UtilityAiProvider {
    /// Creates a new utility AI provider.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ActionProvider for UtilityAiProvider {
    async fn provide_action(
        &self,
        entity: EntityId,
        state: &GameState,
        env: GameEnv<'_>,
    ) -> Result<Action> {
        // Get actor
        let actor = state
            .entities
            .actor(entity)
            .ok_or_else(|| crate::api::errors::RuntimeError::InvalidEntityId(entity))?;

        // Get available actions from game-core
        let available_actions = game_core::get_available_actions(actor, state, &env);

        tracing::debug!(
            "NPC {:?} has {} available actions",
            entity,
            available_actions.len()
        );

        // Build AI context
        let ctx =
            AiContext::new(entity, state, env).with_available_actions(available_actions.clone());

        // Debug: Check if trait profile is loaded
        // TODO: Restore after actor system migration
        if let Some(_profile) = ctx.trait_profile() {
            tracing::debug!("NPC {:?} has trait profile loaded", entity);
        } else {
            tracing::debug!(
                "NPC {:?} has NO trait profile loaded (expected during migration)",
                entity
            );
        }

        // Layer 1: Select Intent
        let (intent, intent_score) = IntentScorer::select(&ctx);
        tracing::debug!(
            "NPC {:?} selected Intent: {:?} (score: {})",
            entity,
            intent,
            intent_score.value()
        );

        // Layer 2: Select Tactic
        let (tactic, tactic_score) = TacticScorer::select(intent, &ctx);
        tracing::debug!(
            "NPC {:?} selected Tactic: {:?} (score: {})",
            entity,
            tactic,
            tactic_score.value()
        );

        // Layer 3: Select Action
        let action_kind = ActionScorer::select(tactic, &available_actions, &ctx);

        tracing::debug!("NPC {:?} selected Action: {:?}", entity, action_kind);

        // Convert to Action or fallback to Wait
        let action = match action_kind {
            Some(kind) => Action::character(entity, kind),
            None => {
                tracing::warn!(
                    "NPC {:?} - No valid action found for tactic {:?}, falling back to Wait",
                    entity,
                    tactic
                );
                Action::character(entity, CharacterActionKind::Wait(WaitAction::new(entity)))
            }
        };

        Ok(action)
    }
}
