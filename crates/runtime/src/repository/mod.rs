//! Repository layer for dynamic runtime data
//!
//! Repositories handle data that CHANGES during gameplay:
//! - Game state (for save/load)
//! - Checkpoints (for replay/rollback)
//!
//! Static game content (items, NPCs, maps) is handled by Oracles, not Repositories.

mod error;
mod state;
mod traits;

pub use error::RepositoryError;
pub use state::InMemoryStateRepo;
pub use traits::StateRepository;
