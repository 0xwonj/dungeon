//! Scenario system for entity placement and game initialization.
//!
//! Scenarios define which map to use and where to place entities.
//! This separation allows:
//! - Same map with different entity placements (easy/hard mode)
//! - Procedural entity generation while keeping map data static
//! - Clean responsibility separation: MapOracle = terrain, Scenario = entities

use std::path::Path;

use game_core::{GameState, ItemHandle, ItemState, Position, PropKind, PropState};
use serde::{Deserialize, Serialize};

use crate::api::{Result, RuntimeError};
use crate::oracle::OracleBundle;

/// Entity placement specification for scenario setup.
///
/// Unlike the old InitialEntitySpec, this has no EntityId - IDs are allocated at runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityPlacement {
    pub position: Position,
    pub kind: EntityKind,
}

/// Type of entity to place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    /// Player character
    Player,

    /// Actor (NPC or enemy) with definition ID
    Actor { def_id: String },

    /// Prop entity
    Prop { kind: PropKind, is_active: bool },

    /// Item on the ground
    Item { handle: ItemHandle },
}

/// Scenario configuration for game initialization.
///
/// Scenarios define which map to use and where to place entities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario {
    /// Map identifier to load
    pub map_id: String,

    /// Entity placements for this scenario
    pub placements: Vec<EntityPlacement>,
}

impl Scenario {
    /// Creates a new scenario.
    pub fn new(map_id: String, placements: Vec<EntityPlacement>) -> Self {
        Self { map_id, placements }
    }

    /// Initialize GameState from this scenario.
    ///
    /// This allocates EntityIds, creates entities from templates,
    /// and sets up initial world occupancy.
    pub fn create_initial_state(&self, oracles: &OracleBundle) -> Result<GameState> {
        // Start with empty state - scenario will add all entities explicitly
        let mut state = GameState::empty();

        tracing::info!(
            "Creating initial state from scenario with {} placements",
            self.placements.len()
        );

        for placement in &self.placements {
            match &placement.kind {
                EntityKind::Player => {
                    // Player always gets EntityId::PLAYER (0)
                    tracing::info!("Processing Player placement at {:?}", placement.position);
                    let template = oracles.actors().template("player").ok_or_else(|| {
                        RuntimeError::InvalidConfig(
                            "Player template 'player' not found".to_string(),
                        )
                    })?;

                    state
                        .add_player(template, placement.position)
                        .map_err(|e| {
                            RuntimeError::InvalidConfig(format!("Failed to add player: {}", e))
                        })?;
                    tracing::info!(
                        "Player added successfully. Active actors: {:?}",
                        state.turn.active_actors
                    );
                }

                EntityKind::Actor { def_id } => {
                    let template = oracles.actors().template(def_id).ok_or_else(|| {
                        RuntimeError::InvalidConfig(format!(
                            "Actor template '{}' not found",
                            def_id
                        ))
                    })?;

                    state.add_npc(template, placement.position).map_err(|e| {
                        RuntimeError::InvalidConfig(format!(
                            "Failed to add NPC '{}': {}",
                            def_id, e
                        ))
                    })?;
                }

                EntityKind::Prop { kind, is_active } => {
                    let id = state.allocate_entity_id().map_err(|e| {
                        RuntimeError::InvalidConfig(format!("Failed to allocate entity ID: {}", e))
                    })?;
                    let prop = PropState {
                        id,
                        position: placement.position,
                        kind: kind.clone(),
                        is_active: *is_active,
                    };
                    state.entities.props.push(prop).map_err(|_| {
                        RuntimeError::InvalidConfig(
                            "Failed to add prop (props list full)".to_string(),
                        )
                    })?;

                    state.world.tile_map.add_occupant(placement.position, id);
                }

                EntityKind::Item { handle } => {
                    let id = state.allocate_entity_id().map_err(|e| {
                        RuntimeError::InvalidConfig(format!("Failed to allocate entity ID: {}", e))
                    })?;
                    let item = ItemState {
                        id,
                        handle: *handle,
                        position: placement.position,
                        quantity: 1,
                    };
                    state.entities.items.push(item).map_err(|_| {
                        RuntimeError::InvalidConfig(
                            "Failed to add item (items list full)".to_string(),
                        )
                    })?;

                    state.world.tile_map.add_occupant(placement.position, id);
                }
            }
        }

        Ok(state)
    }

    /// Load scenario from a RON file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the RON file containing Scenario data
    ///
    /// # Returns
    ///
    /// Returns a Scenario with map_id and entity placements.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            RuntimeError::InvalidConfig(format!("Failed to read scenario file: {}", e))
        })?;

        let scenario: Scenario = ron::from_str(&content).map_err(|e| {
            RuntimeError::InvalidConfig(format!("Failed to parse scenario RON: {}", e))
        })?;

        Ok(scenario)
    }
}
