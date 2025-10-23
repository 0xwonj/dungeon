//! Actor oracle implementing [`game_core::ActorOracle`].
//!
//! This module provides the unified actor system that treats both players and NPCs
//! uniformly. The only difference is their default action provider.

use std::collections::HashMap;

use game_content::traits::TraitProfile;
use game_core::{ActorOracle, ActorTemplate};
use serde::{Deserialize, Serialize};

use crate::api::ProviderKind;

/// AI configuration for an actor.
///
/// Contains all AI-related data that affects how an actor makes decisions:
/// - Behavioral trait profile (Bravery, Aggression, Perception, etc.)
/// - Default action provider (which provider to use by default)
///
/// # Design
///
/// Both players and NPCs have AI config:
/// - **Player**: Has traits (for auto-play, companion mode), default provider is Interactive
/// - **NPC**: Has traits (for behavior), default provider is Ai
///
/// The default provider can be overridden at runtime (e.g., debugging NPC by controlling it).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    /// Behavioral trait profile (0..240 per trait).
    ///
    /// These traits influence decision-making in AI systems:
    /// - Bravery: Risk tolerance, flee threshold
    /// - Aggression: Combat initiation, pursuit
    /// - Caution: Preparation, trap avoidance
    /// - Perception: Vision range, stealth detection
    /// - etc. (20 traits total)
    ///
    /// Even players have traits for auto-play or companion behaviors.
    pub traits: TraitProfile,

    /// Default action provider for this actor.
    ///
    /// - `Interactive(CliInput)`: Human player control
    /// - `Ai(Utility)`: AI-controlled NPC
    /// - Can be overridden at runtime via ProviderRegistry
    pub default_provider: ProviderKind,
}

impl Default for AiConfig {
    fn default() -> Self {
        use crate::api::{AiKind, ProviderKind};
        Self {
            traits: TraitProfile::default(),
            default_provider: ProviderKind::Ai(AiKind::Wait),
        }
    }
}

/// Oracle providing actor templates and AI configurations.
///
/// # Design
///
/// Separates templates from AI configs at the storage level:
/// - `templates`: HashMap<String, ActorTemplate> (for ActorOracle)
/// - `ai_configs`: HashMap<String, AiConfig> (for runtime AI setup)
///
/// This separation allows game-core to access templates via ActorOracle
/// without exposing AI configuration details.
///
/// Both players and NPCs are stored here uniformly. The only difference
/// is their AI config's default_provider field.
///
/// # Usage
///
/// ```rust
/// let mut oracle = ActorOracleImpl::new();
///
/// // Add player
/// oracle.add("player", player_template, player_ai_config);
///
/// // Add NPC
/// oracle.add("goblin_scout", goblin_template, goblin_ai_config);
///
/// // game-core accesses templates only (via ActorOracle trait)
/// let template = oracle.template("goblin_scout");
///
/// // runtime accesses AI configs
/// let ai_config = oracle.ai_config("goblin_scout");
/// ```
pub struct ActorOracleImpl {
    templates: HashMap<String, ActorTemplate>,
    ai_configs: HashMap<String, AiConfig>,
}

impl ActorOracleImpl {
    /// Create an empty oracle.
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            ai_configs: HashMap::new(),
        }
    }

    /// Add an actor to the oracle.
    ///
    /// Stores template and AI config separately.
    pub fn add(&mut self, id: impl Into<String>, template: ActorTemplate, ai_config: AiConfig) {
        let id = id.into();
        self.templates.insert(id.clone(), template);
        self.ai_configs.insert(id, ai_config);
    }

    /// Get actor template by ID.
    ///
    /// This is used by game-core via ActorOracle trait.
    pub fn template(&self, id: &str) -> Option<&ActorTemplate> {
        self.templates.get(id)
    }

    /// Get AI configuration by ID.
    ///
    /// This is used by runtime for provider setup and trait access.
    pub fn ai_config(&self, id: &str) -> Option<&AiConfig> {
        self.ai_configs.get(id)
    }

    /// Check if an actor exists.
    pub fn contains(&self, id: &str) -> bool {
        self.templates.contains_key(id)
    }

    /// Get all actor IDs.
    pub fn actor_ids(&self) -> impl Iterator<Item = &String> {
        self.templates.keys()
    }

    /// Get number of actors in catalog.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

impl Default for ActorOracleImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorOracle for ActorOracleImpl {
    fn template(&self, def_id: &str) -> Option<ActorTemplate> {
        self.templates.get(def_id).cloned()
    }

    fn all_ids(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
}
