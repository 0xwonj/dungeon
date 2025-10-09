//! Helpers for constructing oracle bundles consumed by the runtime.
use std::sync::Arc;

use runtime::{ItemOracleImpl, MapOracleImpl, NpcOracleImpl, OracleManager, TablesOracleImpl};

use crate::config::MapSize;

/// Bundle of oracle implementations that the runtime consumes.
#[derive(Clone)]
pub struct OracleBundle {
    pub map: Arc<MapOracleImpl>,
    pub items: Arc<ItemOracleImpl>,
    pub tables: Arc<TablesOracleImpl>,
    pub npcs: Arc<NpcOracleImpl>,
}

impl OracleBundle {
    pub fn manager(&self) -> OracleManager {
        OracleManager::new(
            Arc::clone(&self.map),
            Arc::clone(&self.items),
            Arc::clone(&self.tables),
            Arc::clone(&self.npcs),
        )
    }
}

pub trait OracleFactory: Send + Sync {
    fn build(&self) -> OracleBundle;
}

/// Temporary factory that relies on runtime-provided test fixtures.
///
/// As `game-content` becomes populated this will move to use real data.
#[derive(Clone, Debug)]
pub struct TestOracleFactory {
    size: MapSize,
}

impl TestOracleFactory {
    pub const fn new(size: MapSize) -> Self {
        Self { size }
    }
}

impl OracleFactory for TestOracleFactory {
    fn build(&self) -> OracleBundle {
        let map = Arc::new(MapOracleImpl::test_map(self.size.width, self.size.height));
        let items = Arc::new(ItemOracleImpl::test_items());
        let tables = Arc::new(TablesOracleImpl::test_tables());
        let npcs = Arc::new(NpcOracleImpl::test_npcs());

        OracleBundle {
            map,
            items,
            tables,
            npcs,
        }
    }
}
