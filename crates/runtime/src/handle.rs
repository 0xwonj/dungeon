use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::Action;

use crate::error::Result;
use crate::event::GameEvent;
use crate::worker::{Command, StepResult};

/// Client-facing handle to interact with the runtime
#[derive(Clone)]
pub struct RuntimeHandle {
    command_tx: mpsc::Sender<Command>,
    event_tx: broadcast::Sender<GameEvent>,
}

impl RuntimeHandle {
    pub(crate) fn new(
        command_tx: mpsc::Sender<Command>,
        event_tx: broadcast::Sender<GameEvent>,
    ) -> Self {
        Self {
            command_tx,
            event_tx,
        }
    }

    /// Execute a player action
    pub async fn execute_action(&self, action: Action) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::ExecuteAction {
                action,
                reply: reply_tx,
            })
            .await
            .map_err(|_| crate::error::RuntimeError::ExecuteFailed("Send failed".into()))?;

        reply_rx
            .await
            .map_err(|_| crate::error::RuntimeError::ExecuteFailed("Reply failed".into()))?
    }

    /// Advance simulation by one turn
    pub async fn step(&self) -> Result<StepResult> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::Step { reply: reply_tx })
            .await
            .map_err(|_| crate::error::RuntimeError::ExecuteFailed("Send failed".into()))?;

        reply_rx
            .await
            .map_err(|_| crate::error::RuntimeError::ExecuteFailed("Reply failed".into()))?
    }

    /// Subscribe to game events
    pub fn subscribe_events(&self) -> broadcast::Receiver<GameEvent> {
        self.event_tx.subscribe()
    }
}
