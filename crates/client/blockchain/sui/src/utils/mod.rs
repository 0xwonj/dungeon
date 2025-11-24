//! Utility modules for Sui blockchain integration.
//!
//! This module provides common utilities for interacting with the Sui blockchain,
//! including type conversions and helper functions.
//!
//! ## Modules
//!
//! - [`conversion`]: Type conversions and validation with Adapter Pattern

pub mod conversion;

// Re-export commonly used items
pub use conversion::{object_id_to_session_id, session_id_to_object_id};
