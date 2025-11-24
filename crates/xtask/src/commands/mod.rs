//! Command implementations for xtask
//!
//! Each command is a separate module that implements its own CLI args and execution logic.

mod clean;
mod extract_vk;
mod inspect_proof;
mod read_actions;
mod read_state;
pub mod sui;
mod tail_logs;

pub use clean::Clean;
pub use extract_vk::ExtractVk;
pub use inspect_proof::InspectProof;
pub use read_actions::ReadActions;
pub use read_state::ReadState;
pub use sui::{Keygen as SuiKeygen, Setup as SuiSetup};
pub use tail_logs::TailLogs;
