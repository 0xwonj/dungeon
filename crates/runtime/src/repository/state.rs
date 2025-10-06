use game_core::GameState;
use std::sync::RwLock;

use super::StateRepository;
use crate::error::Result;

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
            .map_err(|e| crate::error::RuntimeError::RepositoryError(format!("Lock error: {}", e)))?;
        Ok(state.clone())
    }

    fn save(&self, state: &GameState) -> Result<()> {
        let mut current = self.state.write().map_err(|e| {
            crate::error::RuntimeError::RepositoryError(format!("Lock error: {}", e))
        })?;
        *current = state.clone();
        Ok(())
    }
}
