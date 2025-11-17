//! Service layer for ViewModel updates and business logic.

pub mod targeting;
pub mod updater;

pub use updater::{UpdateScope, ViewModelUpdater};
