//! File-based repository implementations.

mod checkpoint;
mod log;
mod state;

pub use checkpoint::FileCheckpointRepository;
pub use state::FileStateRepository;

// Append-only log repository (generic implementation)
pub use log::FileRepository;

// Type aliases for specific log types
use crate::events::Event;
use crate::repository::types::ActionLogEntry;

/// File-based event log using the append-only log repository
pub type FileEventLog = FileRepository<Event>;

/// File-based action log using the append-only log repository
pub type FileActionLog = FileRepository<ActionLogEntry>;
