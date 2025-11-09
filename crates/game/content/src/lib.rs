//! Data-driven content definitions and loaders.
//!
//! This crate houses static game content and provides loaders for RON/TOML data files:
//! - NPC behavioral traits (trait system)
//! - Map layouts (data-driven via RON)
//! - NPC templates (data-driven via RON)
//! - Item catalogs (data-driven via RON)
//! - Game rules tables (data-driven via TOML)
//! - Game configuration (data-driven via TOML)
//!
//! Content is consumed by runtime oracles and never appears in game state.
//!
//! All loaders use game-core types directly with serde for RON/TOML deserialization.

pub mod traits;

#[cfg(feature = "loaders")]
pub mod loaders;

pub use traits::{
    TraitKind, TraitLayer, TraitProfile, TraitProfileSpec, TraitRegistry, TraitWeights,
    build_layer_from_pairs, lerp_f32, lerp_u8,
};

#[cfg(feature = "loaders")]
pub use loaders::{
    ActionProfileRegistry, ActorLoader, AiKindSpec, ConfigLoader, ContentFactory,
    InteractiveKindSpec, ItemLoader, MapLoader, ProviderKindSpec, TablesLoader,
    load_trait_registry,
};
