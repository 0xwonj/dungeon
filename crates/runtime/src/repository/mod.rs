//! Repository layer for dynamic runtime data.
//!
//! Repositories handle data that CHANGES during gameplay:
//! - Game state (for save/load)
//! - Checkpoints (for replay/rollback)
//! - Event logs (for persistence)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │ SnapshotService │  (Facade)
//! └────────┬────────┘
//!          │
//!    ┌─────┴─────┐
//!    ▼           ▼
//! StateRepo  CheckpointRepo
//!    │           │
//!    ▼           ▼
//! [Trait]    [Trait]
//!    │           │
//!    ├─ File    ├─ File
//!    └─ Memory  └─ Memory  (future: DB, Cloud)
//! ```
//!
//! # Module Organization
//!
//! - `traits`: Repository trait definitions
//! - `types`: Shared data structures (Checkpoint, etc.)
//! - `file`: File-based implementations
//! - `memory`: In-memory implementation (testing)
//! - `snapshot`: High-level facade service

pub mod file;
pub mod memory;
pub mod types;

mod error;
mod traits;

// Re-export main types
pub use error::RepositoryError;
pub use traits::{
    ActionLogReader, ActionRepository, CheckpointRepository, EventRepository,
    ProofIndexRepository, StateRepository,
};

// Re-export shared types
pub use types::{
    ActionLogEntry, Checkpoint, EventReference, ProofEntry, ProofIndex, ProofReference,
    StateReference,
};

// Re-export file implementations
pub use file::{
    FileActionLog, FileCheckpointRepository, FileEventLog, FileProofIndexRepository,
    FileRepository, FileStateRepository, MmapActionLogReader,
};

// Re-export memory implementations
pub use memory::{
    InMemoryActionLogReader, InMemoryCheckpointRepository, InMemoryEventRepository,
    InMemoryStateRepo,
};
