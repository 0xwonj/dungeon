//! Utility-based AI action provider.
//!
//! This provider uses a goal-directed utility approach:
//! 1. Select a goal based on current situation and traits
//! 2. Generate all possible action candidates
//! 3. Score each candidate by utility (how well it serves the goal)
//! 4. Execute the highest-scoring candidate

use async_trait::async_trait;
use game_core::{Action, CharacterAction, EntityId, GameEnv, GameState};

use super::AiContext;
use super::generator::ActionCandidateGenerator;
use super::goal::GoalSelector;
use crate::api::{ActionProvider, Result};

/// Utility-based AI provider with goal-directed decision making.
///
/// # Algorithm
///
/// 1. **Goal Selection**: Pick a concrete goal based on situation and traits
///    - Examples: "Attack Player", "Flee from Player", "Heal Self", "Idle"
/// 2. **Candidate Generation**: Generate all possible (Action, Input) pairs
/// 3. **Utility Scoring**: Score each candidate by how well it serves the goal (0-100)
/// 4. **Selection**: Execute the highest-scoring candidate
///
/// # Design Philosophy
///
/// - **Natural**: Goals match how we think ("I want to attack that enemy")
/// - **Simple**: One decision (goal) → one evaluation (score actions) → one output
/// - **Flexible**: Easy to add new goals without restructuring layers
/// - **Debuggable**: Clear trace of goal → action → score
#[derive(Debug, Clone, Default)]
pub struct UtilityAiProvider;

impl UtilityAiProvider {
    /// Creates a new utility-based AI provider.
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
        // Validate entity exists
        let _actor = state
            .entities
            .actor(entity)
            .ok_or_else(|| crate::api::errors::RuntimeError::InvalidEntityId(entity))?;

        // Get available ActionKinds from game-core
        let available_kinds = game_core::get_available_actions(entity, state, &env);

        tracing::debug!(
            "UtilityAI: entity={:?} has {} available actions",
            entity,
            available_kinds.len()
        );

        // Build AI context
        let ctx =
            AiContext::new(entity, state, env).with_available_actions(available_kinds.clone());

        // ====================================================================
        // Step 1: Select Goal
        // ====================================================================

        let goal = GoalSelector::select(&ctx);

        tracing::debug!("UtilityAI: entity={:?} selected goal: {:?}", entity, goal);

        // ====================================================================
        // Step 2: Generate Candidates
        // ====================================================================

        let candidates = ActionCandidateGenerator::generate(&available_kinds, &ctx);

        if candidates.is_empty() {
            tracing::debug!(
                "UtilityAI: entity={:?} has no action candidates, falling back to Wait",
                entity
            );
            return Ok(Action::Character(CharacterAction::new(
                entity,
                game_core::ActionKind::Wait,
                game_core::ActionInput::None,
            )));
        }

        tracing::debug!(
            "UtilityAI: entity={:?} evaluating {} candidates",
            entity,
            candidates.len()
        );

        // ====================================================================
        // Step 3: Score Candidates by Goal
        // ====================================================================

        let mut best_candidate = None;
        let mut best_score = 0;

        for (kind, input) in candidates {
            let score = goal.evaluate_action(kind, &input, &ctx);

            tracing::debug!("  Candidate: {:?} + {:?} = score {}", kind, input, score);

            if score > best_score {
                best_score = score;
                best_candidate = Some((kind, input));
            }
        }

        // ====================================================================
        // Step 4: Build Final Action
        // ====================================================================

        let (kind, input) =
            best_candidate.unwrap_or((game_core::ActionKind::Wait, game_core::ActionInput::None));

        tracing::debug!(
            "UtilityAI: entity={:?} selected action: {:?} with input {:?} (score={})",
            entity,
            kind,
            input,
            best_score
        );

        let character_action = CharacterAction::new(entity, kind, input);

        Ok(Action::Character(character_action))
    }
}
