//! Cloneable façade for issuing commands to the runtime.
//!
//! [`RuntimeHandle`] hides channel plumbing and offers async helpers for
//! stepping the simulation or streaming events.
use tokio::sync::{broadcast, mpsc, oneshot};

use game_core::{Action, EntityId, GameState};

use super::errors::{Result, RuntimeError};
use super::events::GameEvent;
use crate::workers::Command;

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

    /// Prepare the next turn - determines which entity acts next and returns game state clone
    pub async fn prepare_next_turn(&self) -> Result<(EntityId, GameState)> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::PrepareNextTurn { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Execute an action for the current turn entity
    pub async fn execute_action(&self, action: Action) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::ExecuteAction {
                action,
                reply: reply_tx,
            })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)?
    }

    /// Subscribe to game events
    pub fn subscribe_events(&self) -> broadcast::Receiver<GameEvent> {
        self.event_tx.subscribe()
    }

    /// Query the current game state (read-only snapshot)
    pub async fn query_state(&self) -> Result<GameState> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(Command::QueryState { reply: reply_tx })
            .await
            .map_err(|_| RuntimeError::CommandChannelClosed)?;

        reply_rx.await.map_err(RuntimeError::ReplyChannelClosed)
    }
}
