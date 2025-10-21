//! Behavior tree-based action provider.

use async_trait::async_trait;
use behavior_tree::Behavior;
use game_core::{Action, CharacterActionKind, EntityId, GameEnv, GameState};

use crate::api::{ActionProvider, Result};
use crate::providers::ai::AiContext;

/// An action provider that uses a behavior tree to make decisions.
///
/// This provider evaluates a behavior tree for each entity to determine
/// what action it should take. The tree is evaluated synchronously within
/// the `provide_action` call.
///
/// # Design
///
/// The provider:
/// 1. Creates an [`AiContext`] with the entity and game state
/// 2. Evaluates the behavior tree by calling `tick()`
/// 3. Extracts the action from the context
/// 4. Falls back to Wait if no action was generated
pub struct BehaviorTreeProvider {
    tree: Box<dyn Behavior<AiContext<'static>>>,
}

impl BehaviorTreeProvider {
    /// Creates a new behavior tree provider with the given tree.
    ///
    /// # Arguments
    ///
    /// * `tree` - The root behavior tree node
    pub fn new(tree: Box<dyn Behavior<AiContext<'static>>>) -> Self {
        Self { tree }
    }
}

#[async_trait]
impl ActionProvider for BehaviorTreeProvider {
    async fn provide_action(
        &self,
        entity: EntityId,
        state: &GameState,
        env: GameEnv<'_>,
    ) -> Result<Action> {
        // SAFETY: We transmute the lifetime here, but it's safe because:
        // 1. The context is created fresh and doesn't escape this function
        // 2. The tree doesn't store references to the context
        // 3. All borrows from `state` and `env` are dropped before we return
        let mut ctx = AiContext::new(entity, state, env);
        let ctx_static: &mut AiContext<'static> = unsafe { std::mem::transmute(&mut ctx) };

        // Evaluate the behavior tree
        let _status = self.tree.tick(ctx_static);

        // Extract action from context
        match ctx.take_action() {
            Some(action) => Ok(action),
            None => {
                // No action was generated - fallback to Wait
                tracing::warn!(
                    entity = ?entity,
                    status = ?_status,
                    "Behavior tree completed without generating action, falling back to Wait"
                );
                Ok(Action::character(entity, CharacterActionKind::Wait))
            }
        }
    }
}
