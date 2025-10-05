use std::collections::BTreeMap;

use arrayvec::ArrayVec;

use crate::config::GameConfig;
use crate::env::{MapOracle, StaticTile};

use super::{EntityId, Position};

type OverlaySlots = ArrayVec<Overlay, { GameConfig::MAX_OVERLAYS_PER_TILE }>;
type OccupantSlots = ArrayVec<EntityId, { GameConfig::MAX_OCCUPANTS_PER_TILE }>;

/// Aggregated world-level state layered on top of the static map commitment.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WorldState {
    pub tile_map: TileMap,
}

impl WorldState {
    pub fn new(tile_map: TileMap) -> Self {
        Self { tile_map }
    }

    /// Produces a merged view combining static tile data with dynamic overlays and occupants.
    pub fn tile_view<'a, M>(&'a self, map: &M, position: Position) -> Option<TileView<'a>>
    where
        M: MapOracle + ?Sized,
    {
        let static_tile = map.tile(position)?;
        let overlay = self.tile_map.overlay(&position);
        let occupants = self
            .tile_map
            .occupants(&position)
            .cloned()
            .unwrap_or_default();

        Some(TileView {
            position,
            static_tile,
            overlay,
            occupants,
        })
    }
}

/// Dynamic world deltas layered on top of immutable static tiles.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TileMap {
    overlays: BTreeMap<Position, OverlaySet>,
    occupancy: BTreeMap<Position, OccupantSlots>,
}

impl TileMap {
    pub fn new(
        overlays: BTreeMap<Position, OverlaySet>,
        occupancy: BTreeMap<Position, OccupantSlots>,
    ) -> Self {
        Self {
            overlays,
            occupancy,
        }
    }

    pub fn overlays(&self) -> &BTreeMap<Position, OverlaySet> {
        &self.overlays
    }

    pub fn overlay(&self, position: &Position) -> Option<&OverlaySet> {
        self.overlays.get(position)
    }

    pub fn set_overlay(&mut self, position: Position, overlay: OverlaySet) {
        if overlay.is_empty() {
            self.overlays.remove(&position);
        } else {
            self.overlays.insert(position, overlay);
        }
    }

    pub fn with_overlay<F>(&mut self, position: Position, mutate: F)
    where
        F: FnOnce(&mut OverlaySet),
    {
        let should_remove = {
            let entry = self.overlays.entry(position).or_default();
            mutate(entry);
            entry.is_empty()
        };
        if should_remove {
            self.overlays.remove(&position);
        }
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
        if slot.iter().any(|occupant| *occupant == entity) {
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

/// Collection of dynamic overlays that modify an individual tile.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OverlaySet {
    overlays: OverlaySlots,
}

impl OverlaySet {
    pub fn new(overlays: OverlaySlots) -> Self {
        Self { overlays }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Overlay> + '_ {
        self.overlays.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Overlay> + '_ {
        self.overlays.iter_mut()
    }

    pub fn push_overlay(&mut self, overlay: Overlay) -> Result<(), Overlay> {
        self.overlays.try_push(overlay).map_err(|err| err.element())
    }

    pub fn retain_overlays<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&Overlay) -> bool,
    {
        self.overlays.retain(|overlay| predicate(overlay));
    }

    pub fn clear(&mut self) {
        self.overlays.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    pub fn is_passable(&self) -> bool {
        self.iter().all(|overlay| overlay.is_passable())
    }
}

/// General overlay applied to a tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Overlay {
    /// Lingering hazard that may apply damage or block entry while active.
    Hazard(HazardOverlay),
    /// Example effect showing how scripted events could tag tiles.
    EventMarker(EventId),
}

impl Overlay {
    pub fn hazard(&self) -> Option<&HazardOverlay> {
        if let Overlay::Hazard(hazard) = self {
            Some(hazard)
        } else {
            None
        }
    }

    pub fn hazard_mut(&mut self) -> Option<&mut HazardOverlay> {
        if let Overlay::Hazard(hazard) = self {
            Some(hazard)
        } else {
            None
        }
    }

    pub fn is_passable(&self) -> bool {
        match self {
            Overlay::Hazard(hazard) => hazard.is_passable(),
            Overlay::EventMarker(_) => true,
        }
    }
}

/// Transient hazard applied to the tile (e.g., lingering fire).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HazardOverlay {
    pub remaining_turns: u32,
    pub passable: bool,
}

impl HazardOverlay {
    pub fn is_passable(&self) -> bool {
        self.passable
    }
}

/// Identifier referencing a scripted world event or trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventId(pub u16);

/// Aggregated tile information used by reducers and commands.
pub struct TileView<'a> {
    position: Position,
    static_tile: StaticTile,
    overlay: Option<&'a OverlaySet>,
    occupants: OccupantSlots,
}

impl<'a> TileView<'a> {
    pub fn position(&self) -> Position {
        self.position
    }

    pub fn static_tile(&self) -> &StaticTile {
        &self.static_tile
    }

    pub fn overlay(&self) -> Option<&'a OverlaySet> {
        self.overlay
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

    pub fn has_hazard(&self) -> bool {
        self.overlay
            .map(|overlay| {
                overlay
                    .iter()
                    .any(|overlay| matches!(overlay, Overlay::Hazard(_)))
            })
            .unwrap_or(false)
    }

    pub fn is_passable(&self) -> bool {
        if !self.static_tile.is_passable() {
            return false;
        }

        self.overlay
            .map(|overlay| overlay.is_passable())
            .unwrap_or(true)
    }

    pub fn terrain(&self) -> crate::env::TerrainKind {
        self.static_tile.terrain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{MapDimensions, TerrainKind};

    struct StubMap;

    impl MapOracle for StubMap {
        fn dimensions(&self) -> MapDimensions {
            MapDimensions::new(4, 4)
        }

        fn tile(&self, position: Position) -> Option<StaticTile> {
            if self.dimensions().contains(position) {
                Some(StaticTile::new(TerrainKind::Floor))
            } else {
                None
            }
        }
    }

    #[test]
    fn tile_view_combines_static_and_dynamic_state() {
        let mut world = WorldState::default();
        let pos = Position::new(1, 1);
        world.tile_map.with_overlay(pos, |overlay| {
            overlay
                .push_overlay(Overlay::Hazard(HazardOverlay {
                    remaining_turns: 2,
                    passable: false,
                }))
                .expect("overlay capacity");
        });
        let mut occupants = OccupantSlots::default();
        occupants
            .try_push(EntityId::PLAYER)
            .expect("occupancy capacity");
        world.tile_map.replace_occupants(pos, occupants);

        let map = StubMap;
        let view = world.tile_view(&map, pos).expect("tile should exist");

        assert!(view.is_occupied());
        assert!(!view.is_passable());
        assert_eq!(view.terrain(), TerrainKind::Floor);
        assert!(view.has_hazard());
    }

    #[test]
    fn tile_view_stays_passable_when_overlays_allow_entry() {
        let mut world = WorldState::default();
        let pos = Position::new(0, 0);
        world.tile_map.with_overlay(pos, |overlay| {
            overlay
                .push_overlay(Overlay::EventMarker(EventId(1)))
                .expect("overlay capacity");
            overlay
                .push_overlay(Overlay::Hazard(HazardOverlay {
                    remaining_turns: 1,
                    passable: true,
                }))
                .expect("overlay capacity");
        });

        let map = StubMap;
        let view = world.tile_view(&map, pos).expect("tile should exist");

        assert!(view.is_passable());
    }
}
