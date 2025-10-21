use async_trait::async_trait;
use game_core::{Action, EntityId, GameEnv, GameState};
use runtime::ActionProvider;
use tokio::sync::{mpsc, Mutex};

/// Action provider that waits for player input from CLI
pub struct CliActionProvider {
    /// Receiver for player actions from CLI UI (wrapped in Mutex for interior mutability)
    rx_action: Mutex<mpsc::Receiver<Action>>,
}

impl CliActionProvider {
    pub fn new(rx_action: mpsc::Receiver<Action>) -> Self {
        Self {
            rx_action: Mutex::new(rx_action),
        }
    }
}

#[async_trait]
impl ActionProvider for CliActionProvider {
    async fn provide_action(
        &self,
        entity: EntityId,
        _state: &GameState,
        _env: GameEnv<'_>,
    ) -> runtime::Result<Action> {
        let mut rx = self.rx_action.lock().await;

        match rx.recv().await {
            Some(action) => {
                // Validate that the action is for the correct entity
                if action.actor() != entity {
                    tracing::error!(
                        "Action actor mismatch: received {:?}, expected {:?}",
                        action.actor(),
                        entity
                    );
                    return Err(runtime::RuntimeError::InvalidEntityId(action.actor()));
                }
                Ok(action)
            }
            None => Err(runtime::RuntimeError::ActionProviderChannelClosed),
        }
    }
}
