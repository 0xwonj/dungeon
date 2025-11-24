//! In-memory StateRepository implementation for tests and local runs.

use std::collections::HashMap;
use std::sync::RwLock;

use game_core::GameState;

use crate::repository::{RepositoryError, Result, StateRepository};

/// In-memory implementation of StateRepository.
///
/// Stores states indexed by nonce for testing and local development.
pub struct InMemoryStateRepo {
    states: RwLock<HashMap<u64, GameState>>,
}

impl InMemoryStateRepo {
    /// Create a new empty in-memory repository.
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Create with an initial state at nonce 0.
    pub fn with_initial_state(initial_state: GameState) -> Self {
        let mut states = HashMap::new();
        states.insert(0, initial_state);
        Self {
            states: RwLock::new(states),
        }
    }
}

impl Default for InMemoryStateRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl StateRepository for InMemoryStateRepo {
    fn save(&self, nonce: u64, state: &GameState) -> Result<()> {
        let mut states = self
            .states
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;
        states.insert(nonce, state.clone());
        Ok(())
    }

    fn load(&self, nonce: u64) -> Result<Option<GameState>> {
        let states = self
            .states
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;
        Ok(states.get(&nonce).cloned())
    }

    fn exists(&self, nonce: u64) -> bool {
        self.states
            .read()
            .map(|states| states.contains_key(&nonce))
            .unwrap_or(false)
    }

    fn delete(&self, nonce: u64) -> Result<()> {
        let mut states = self
            .states
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;
        states.remove(&nonce);
        Ok(())
    }

    fn list_nonces(&self) -> Result<Vec<u64>> {
        let states = self
            .states
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;
        let mut nonces: Vec<u64> = states.keys().copied().collect();
        nonces.sort_unstable();
        Ok(nonces)
    }
}
