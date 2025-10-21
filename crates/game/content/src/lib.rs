//! Data-driven content definitions.
//!
//! This crate houses static game content:
//! - NPC behavioral traits (trait system)
//! - Map layouts (future)
//! - NPC templates (future)
//! - Item catalogs (future)
//!
//! Content is consumed by runtime oracles and never appears in game state.

pub mod traits;

pub use traits::{TraitKind, TraitLayer, TraitProfile, TraitWeights, lerp_f32, lerp_u8};
