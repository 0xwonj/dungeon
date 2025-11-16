//! Scenario system for entity placement and game initialization.
//!
//! Scenarios define which map to use and where to place entities.
//! This separation allows:
//! - Same map with different entity placements (easy/hard mode)
//! - Procedural entity generation while keeping map data static
//! - Clean responsibility separation: MapOracle = terrain, Scenario = entities

use std::path::Path;

use game_core::{
    GameState, ItemHandle, ItemOracle, ItemState, MapOracle, Position, PropKind, PropState,
};
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

    /// Validate scenario against oracles and map.
    ///
    /// Checks:
    /// - Exactly one Player placement
    /// - All positions are within map bounds
    /// - All positions are passable (not walls)
    /// - No duplicate positions
    /// - All actor def_ids exist in ActorOracle
    /// - All item handles exist in ItemOracle
    ///
    /// # Arguments
    ///
    /// * `oracles` - Oracle bundle containing map, actors, items
    ///
    /// # Returns
    ///
    /// - `Ok(())` if scenario is valid
    /// - `Err(RuntimeError::InvalidConfig)` with detailed error message
    pub fn validate(&self, oracles: &OracleBundle) -> Result<()> {
        use std::collections::HashSet;

        // 1. Check player count
        let player_count = self
            .placements
            .iter()
            .filter(|p| matches!(p.kind, EntityKind::Player))
            .count();

        if player_count == 0 {
            return Err(RuntimeError::InvalidConfig(
                "Scenario must have exactly one Player placement".to_string(),
            ));
        }
        if player_count > 1 {
            return Err(RuntimeError::InvalidConfig(format!(
                "Scenario has {} Player placements (must be exactly 1)",
                player_count
            )));
        }

        // 2. Get map dimensions
        let map = &oracles.map;
        let dimensions = map.dimensions();

        tracing::debug!(
            "Validating scenario with {} placements against map {:?}",
            self.placements.len(),
            dimensions
        );

        // 3. Check all placements
        let mut used_positions = HashSet::new();

        for (idx, placement) in self.placements.iter().enumerate() {
            let pos = placement.position;

            // Check bounds
            if !dimensions.contains(pos) {
                return Err(RuntimeError::InvalidConfig(format!(
                    "Placement #{}: Position {:?} is outside map bounds (width={}, height={})",
                    idx, pos, dimensions.width, dimensions.height
                )));
            }

            // Check passability
            if let Some(tile) = map.tile(pos) {
                if !tile.is_passable() {
                    return Err(RuntimeError::InvalidConfig(format!(
                        "Placement #{}: Position {:?} is not passable (terrain: {:?})",
                        idx,
                        pos,
                        tile.terrain()
                    )));
                }
            } else {
                return Err(RuntimeError::InvalidConfig(format!(
                    "Placement #{}: Position {:?} has no tile data",
                    idx, pos
                )));
            }

            // Check duplicates
            if !used_positions.insert(pos) {
                return Err(RuntimeError::InvalidConfig(format!(
                    "Placement #{}: Duplicate entity at position {:?}",
                    idx, pos
                )));
            }

            // Check entity-specific validity
            match &placement.kind {
                EntityKind::Player => {
                    // Verify player template exists
                    if oracles.actors().template("player").is_none() {
                        return Err(RuntimeError::InvalidConfig(
                            "Player template 'player' not found in ActorOracle".to_string(),
                        ));
                    }
                }

                EntityKind::Actor { def_id } => {
                    if oracles.actors().template(def_id).is_none() {
                        return Err(RuntimeError::InvalidConfig(format!(
                            "Placement #{}: Actor template '{}' not found in ActorOracle",
                            idx, def_id
                        )));
                    }
                }

                EntityKind::Item { handle } => {
                    if oracles.items.definition(*handle).is_none() {
                        return Err(RuntimeError::InvalidConfig(format!(
                            "Placement #{}: Item definition {:?} not found in ItemOracle",
                            idx, handle
                        )));
                    }
                }

                EntityKind::Prop { .. } => {
                    // Props don't need oracle validation (kind is self-contained)
                }
            }
        }

        tracing::info!(
            "Scenario validation passed: {} placements, {} unique positions",
            self.placements.len(),
            used_positions.len()
        );

        Ok(())
    }

    /// Initialize GameState from this scenario.
    ///
    /// This allocates EntityIds, creates entities from templates,
    /// and sets up initial world occupancy.
    ///
    /// # Validation
    ///
    /// This method validates the scenario before creating state. If validation fails,
    /// returns `Err(RuntimeError::InvalidConfig)` with a detailed error message.
    ///
    /// # Arguments
    ///
    /// * `oracles` - Oracle bundle containing map, actors, items
    ///
    /// # Returns
    ///
    /// - `Ok(GameState)` - Initialized game state
    /// - `Err(RuntimeError::InvalidConfig)` - Validation failed or entity creation failed
    pub fn create_initial_state(&self, oracles: &OracleBundle) -> Result<GameState> {
        // Validate scenario first - fail fast with clear error messages
        self.validate(oracles)?;

        // Start with empty state - scenario will add all entities explicitly
        let mut state = GameState::empty();

        tracing::info!(
            "Creating initial state from scenario with {} placements (validation passed)",
            self.placements.len()
        );

        for placement in &self.placements {
            match &placement.kind {
                EntityKind::Player => {
                    // Player always gets EntityId::PLAYER (0)
                    tracing::info!("Processing Player placement at {:?}", placement.position);
                    let template = oracles.actors.template("player").ok_or_else(|| {
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
                    let template = oracles.actors.template(def_id).ok_or_else(|| {
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
