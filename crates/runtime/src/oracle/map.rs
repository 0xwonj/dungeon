use game_core::{
    EntityId, InitialEntityKind, InitialEntitySpec, MapDimensions, MapOracle, Position,
    StaticTile,
};
use std::sync::Arc;

use crate::repository::MapRepository;

/// MapOracle implementation backed by MapRepository
pub struct MapOracleImpl {
    pub(crate) repo: Arc<dyn MapRepository>,
    dimensions: MapDimensions,
    initial_entities: Vec<InitialEntitySpec>,
}

impl MapOracleImpl {
    pub fn new(
        repo: Arc<dyn MapRepository>,
        dimensions: MapDimensions,
        initial_entities: Vec<InitialEntitySpec>,
    ) -> Self {
        Self {
            repo,
            dimensions,
            initial_entities,
        }
    }

    /// Creates a test map with sample entities for testing
    pub fn test_map_with_entities(
        repo: Arc<dyn MapRepository>,
        dimensions: MapDimensions,
    ) -> Self {
        let initial_entities = vec![
            // Player at origin
            InitialEntitySpec {
                id: EntityId::PLAYER,
                position: Position::new(0, 0),
                kind: InitialEntityKind::Player,
            },
            // Goblin NPC at (5, 5) using template 0
            InitialEntitySpec {
                id: EntityId(1),
                position: Position::new(5, 5),
                kind: InitialEntityKind::Npc { template: 0 },
            },
        ];

        Self::new(repo, dimensions, initial_entities)
    }
}

impl MapOracle for MapOracleImpl {
    fn dimensions(&self) -> MapDimensions {
        self.dimensions
    }

    fn tile(&self, position: Position) -> Option<StaticTile> {
        // Repository is sync (in-memory for MVP)
        self.repo.get_tile(position).ok().flatten()
    }

    fn initial_entities(&self) -> Vec<InitialEntitySpec> {
        self.initial_entities.clone()
    }
}
