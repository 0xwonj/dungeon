use async_trait::async_trait;
use game_core::{Action, EntityId, GameState};
use runtime::ActionProvider;
use tokio::sync::{Mutex, mpsc};

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
    ) -> runtime::Result<Action> {
        let mut rx = self.rx_action.lock().await;

        match rx.recv().await {
            Some(action) => {
                if action.actor != entity {
                    tracing::warn!(
                        "Received action for entity {:?}, but expected {:?}",
                        action.actor,
                        entity
                    );
                }
                Ok(action)
            }
            None => Err(runtime::RuntimeError::ActionProviderChannelClosed),
        }
    }
}
