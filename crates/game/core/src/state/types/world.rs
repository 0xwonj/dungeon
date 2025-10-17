use std::collections::BTreeMap;

use arrayvec::ArrayVec;

use crate::config::GameConfig;
use crate::env::{MapOracle, StaticTile};

use super::{EntityId, Position};

type OccupantSlots = ArrayVec<EntityId, { GameConfig::MAX_OCCUPANTS_PER_TILE }>;

/// Aggregated world-level state layered on top of the static map commitment.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldState {
    pub tile_map: TileMap,
}

impl WorldState {
    pub fn new(tile_map: TileMap) -> Self {
        Self { tile_map }
    }

    /// Produces a merged view combining static tile data with dynamic occupants.
    pub fn tile_view<M>(&self, map: &M, position: Position) -> Option<TileView>
    where
        M: MapOracle + ?Sized,
    {
        let static_tile = map.tile(position)?;
        let occupants = self
            .tile_map
            .occupants(&position)
            .cloned()
            .unwrap_or_default();

        Some(TileView {
            position,
            static_tile,
            occupants,
        })
    }
}

/// Dynamic world deltas layered on top of immutable static tiles.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TileMap {
    occupancy: BTreeMap<Position, OccupantSlots>,
}

impl TileMap {
    pub fn new(occupancy: BTreeMap<Position, OccupantSlots>) -> Self {
        Self { occupancy }
    }

    pub fn occupancy(&self) -> &BTreeMap<Position, OccupantSlots> {
        &self.occupancy
    }

    pub fn occupants(&self, position: &Position) -> Option<&OccupantSlots> {
        self.occupancy.get(position)
    }

    pub fn replace_occupants(&mut self, position: Position, occupants: OccupantSlots) {
        if occupants.is_empty() {
            self.occupancy.remove(&position);
        } else {
            self.occupancy.insert(position, occupants);
        }
    }

    pub fn add_occupant(&mut self, position: Position, entity: EntityId) -> bool {
        let slot = self.occupancy.entry(position).or_default();
        if slot.contains(&entity) {
            return true;
        }

        slot.try_push(entity).is_ok()
    }

    pub fn remove_occupant(&mut self, position: &Position, entity: EntityId) -> bool {
        if let Some(slot) = self.occupancy.get_mut(position) {
            if let Some(index) = slot.iter().position(|occupant| *occupant == entity) {
                slot.swap_remove(index);
                if slot.is_empty() {
                    self.occupancy.remove(position);
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn clear_occupants(&mut self, position: &Position) {
        self.occupancy.remove(position);
    }
}

/// Aggregated tile information used by reducers and commands.
pub struct TileView {
    position: Position,
    static_tile: StaticTile,
    occupants: OccupantSlots,
}

impl TileView {
    pub fn position(&self) -> Position {
        self.position
    }

    pub fn static_tile(&self) -> &StaticTile {
        &self.static_tile
    }

    pub fn occupants(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.occupants.iter().copied()
    }

    pub fn occupants_slots(&self) -> &OccupantSlots {
        &self.occupants
    }

    pub fn is_occupied(&self) -> bool {
        !self.occupants.is_empty()
    }

    pub fn is_passable(&self) -> bool {
        self.static_tile.is_passable()
    }

    pub fn terrain(&self) -> crate::env::TerrainKind {
        self.static_tile.terrain()
    }
}
