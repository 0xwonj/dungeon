//! Traits describing read-only world data.
//!
//! Oracles expose static map geometry, item definitions, rule tables, and NPC
//! templates. The [`Env`] aggregate bundles them so the engine can access
//! everything it needs without hard coupling to concrete implementations.
mod items;
mod map;
mod npc;
mod tables;

pub use items::{ItemCategory, ItemDefinition, ItemOracle};
pub use map::{
    InitialEntityKind, InitialEntitySpec, MapDimensions, MapOracle, StaticTile, TerrainKind,
};
pub use npc::{NpcOracle, NpcTemplate};
pub use tables::{AttackProfile, MovementRules, TablesOracle};

/// Aggregates read-only oracles required by the reducer and action pipeline.
#[derive(Clone, Copy, Debug)]
pub struct Env<'a, M, I, T, N>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: TablesOracle + ?Sized,
    N: NpcOracle + ?Sized,
{
    map: Option<&'a M>,
    items: Option<&'a I>,
    tables: Option<&'a T>,
    npcs: Option<&'a N>,
}

pub type GameEnv<'a> =
    Env<'a, dyn MapOracle + 'a, dyn ItemOracle + 'a, dyn TablesOracle + 'a, dyn NpcOracle + 'a>;

impl<'a, M, I, T, N> Env<'a, M, I, T, N>
where
    M: MapOracle + ?Sized,
    I: ItemOracle + ?Sized,
    T: TablesOracle + ?Sized,
    N: NpcOracle + ?Sized,
{
    pub fn new(
        map: Option<&'a M>,
        items: Option<&'a I>,
        tables: Option<&'a T>,
        npcs: Option<&'a N>,
    ) -> Self {
        Self {
            map,
            items,
            tables,
            npcs,
        }
    }

    pub fn with_all(map: &'a M, items: &'a I, tables: &'a T, npcs: &'a N) -> Self {
        Self::new(Some(map), Some(items), Some(tables), Some(npcs))
    }

    pub fn empty() -> Self {
        Self {
            map: None,
            items: None,
            tables: None,
            npcs: None,
        }
    }

    pub fn map(&self) -> Option<&'a M> {
        self.map
    }

    pub fn items(&self) -> Option<&'a I> {
        self.items
    }

    pub fn tables(&self) -> Option<&'a T> {
        self.tables
    }

    pub fn npcs(&self) -> Option<&'a N> {
        self.npcs
    }
}

impl<'a, M, I, T, N> Env<'a, M, I, T, N>
where
    M: MapOracle + 'a,
    I: ItemOracle + 'a,
    T: TablesOracle + 'a,
    N: NpcOracle + 'a,
{
    pub fn into_game_env(self) -> GameEnv<'a> {
        let map: Option<&'a dyn MapOracle> = self.map.map(|map| map as _);
        let items: Option<&'a dyn ItemOracle> = self.items.map(|items| items as _);
        let tables: Option<&'a dyn TablesOracle> = self.tables.map(|tables| tables as _);
        let npcs: Option<&'a dyn NpcOracle> = self.npcs.map(|npcs| npcs as _);
        Env::new(map, items, tables, npcs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AttackStyle;
    use crate::state::{EntityId, ItemHandle, Position};

    struct StubMapOracle {
        entities: Vec<InitialEntitySpec>,
    }

    impl StubMapOracle {
        fn new(entities: Vec<InitialEntitySpec>) -> Self {
            Self { entities }
        }
    }

    impl MapOracle for StubMapOracle {
        fn dimensions(&self) -> MapDimensions {
            MapDimensions::new(10, 10)
        }

        fn tile(&self, position: Position) -> Option<StaticTile> {
            if self.dimensions().contains(position) {
                Some(StaticTile::new(TerrainKind::Floor))
            } else {
                None
            }
        }

        fn initial_entities(&self) -> Vec<InitialEntitySpec> {
            self.entities.clone()
        }
    }

    struct StubItemOracle;

    impl ItemOracle for StubItemOracle {
        fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition> {
            Some(ItemDefinition::new(
                handle,
                ItemCategory::Utility,
                None,
                None,
            ))
        }
    }

    struct StubTablesOracle;

    impl TablesOracle for StubTablesOracle {
        fn movement_rules(&self) -> MovementRules {
            MovementRules::new(1, 1)
        }

        fn attack_profile(&self, _style: AttackStyle) -> Option<AttackProfile> {
            Some(AttackProfile::new(5, 0))
        }
    }

    struct StubNpcOracle;

    impl NpcOracle for StubNpcOracle {
        fn template(&self, _template_id: u16) -> Option<NpcTemplate> {
            Some(NpcTemplate::simple(100, 50))
        }
    }

    #[test]
    fn env_exposes_backing_oracles() {
        let map = StubMapOracle::new(vec![InitialEntitySpec {
            id: EntityId::PLAYER,
            position: Position::new(0, 0),
            kind: InitialEntityKind::Player,
        }]);
        let items = StubItemOracle;
        let tables = StubTablesOracle;
        let npcs = StubNpcOracle;
        let env = Env::with_all(&map, &items, &tables, &npcs);

        let position = Position::new(0, 0);
        let map = env.map().expect("map oracle should be available");
        assert!(map.contains(position));
        let tile = map.tile(position).expect("stub tile available");
        assert!(tile.is_passable());
        assert!(
            env.items()
                .expect("item oracle should be available")
                .definition(ItemHandle(1))
                .is_some()
        );
        assert!(
            env.tables()
                .expect("tables oracle should be available")
                .attack_profile(AttackStyle::Melee)
                .is_some()
        );

        let initial_entities = map.initial_entities();
        assert_eq!(initial_entities.len(), 1);
        assert!(matches!(
            initial_entities[0].kind,
            InitialEntityKind::Player
        ));
    }
}
