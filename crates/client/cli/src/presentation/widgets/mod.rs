//! Widget modules for UI rendering.
//!
//! Each widget is a pure function that reads ViewModel and renders to a terminal frame.
//! Widgets follow these principles:
//! - Read-only access to ViewModel (immutable)
//! - No side effects or state mutations
//! - Framework-specific (Ratatui) but follow PresentationMapper pattern where applicable

pub mod examine;
pub mod footer;
pub mod game_area;
pub mod header;
pub mod map;
pub mod messages;
pub mod player_stats;
