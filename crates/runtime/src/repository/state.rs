//! In-memory [`StateRepository`] used for tests and local runs.
use game_core::GameState;
use std::sync::RwLock;

use super::{RepositoryError, StateRepository};
use crate::api::Result;

/// In-memory implementation of StateRepository
pub struct InMemoryStateRepo {
    state: RwLock<GameState>,
}

impl InMemoryStateRepo {
    pub fn new(initial_state: GameState) -> Self {
        Self {
            state: RwLock::new(initial_state),
        }
    }
}

impl StateRepository for InMemoryStateRepo {
    fn load(&self) -> Result<GameState> {
        let state = self
            .state
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;
        Ok(state.clone())
    }

    fn save(&self, state: &GameState) -> Result<()> {
        let mut current = self
            .state
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;
        *current = state.clone();
        Ok(())
    }
}
