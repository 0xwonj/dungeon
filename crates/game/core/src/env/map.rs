use crate::state::{EntityId, ItemHandle, Position, PropKind};

/// Static map oracle exposing immutable layout information and initial entity placement.
pub trait MapOracle: Send + Sync {
    fn dimensions(&self) -> MapDimensions;
    fn tile(&self, position: Position) -> Option<StaticTile>;

    /// Returns the entities that should exist when the scenario starts.
    fn initial_entities(&self) -> Vec<InitialEntitySpec> {
        Vec::new()
    }

    fn contains(&self, position: Position) -> bool {
        self.dimensions().contains(position)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MapDimensions {
    pub width: u32,
    pub height: u32,
}

impl MapDimensions {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn contains(&self, position: Position) -> bool {
        position.x >= 0
            && position.y >= 0
            && position.x < self.width as i32
            && position.y < self.height as i32
    }
}

/// Immutable descriptor for a tile in the static layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StaticTile {
    terrain: TerrainKind,
}

impl StaticTile {
    /// Creates a tile with the given base terrain. Gameplay-only semantics such as
    /// hazards, doors, or triggers belong to runtime state rather than the map oracle.
    pub const fn new(terrain: TerrainKind) -> Self {
        Self { terrain }
    }

    pub fn terrain(self) -> TerrainKind {
        self.terrain
    }

    pub fn is_passable(self) -> bool {
        self.terrain.is_passable()
    }
}

/// Canonical terrain classes for static map tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TerrainKind {
    Floor,
    Wall,
    Void,
    Water,
    Custom(u16),
}

impl TerrainKind {
    pub fn is_passable(self) -> bool {
        matches!(self, TerrainKind::Floor)
    }
}

/// Blueprint describing an entity that should exist at the start of a session.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitialEntitySpec {
    pub id: EntityId,
    pub position: Position,
    pub kind: InitialEntityKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InitialEntityKind {
    Player,
    Npc { template: u16 },
    Prop { kind: PropKind, is_active: bool },
    Item { handle: ItemHandle },
}
