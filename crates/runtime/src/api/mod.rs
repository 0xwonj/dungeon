//! Public runtime API surface.
//!
//! This module gathers the types exposed to consumers of the runtime crate so
//! other layers can stay focused on orchestration, workers, or infrastructure.

pub mod errors;
pub mod handle;
pub mod providers;
pub mod registry;

pub use errors::{AiKind, InteractiveKind, ProviderKind, Result, RuntimeError};
pub use handle::RuntimeHandle;
pub use providers::ActionProvider;
pub use registry::ProviderRegistry;
