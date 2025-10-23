//! Placeholder tables loader.
//!
//! TablesOracle is currently a placeholder with no methods.
//! This loader exists for future expansion when tables are needed.

use std::path::Path;

use crate::loaders::LoadResult;

/// Placeholder loader for game rules tables.
///
/// This will be implemented when TablesOracle gains actual methods.
pub struct TablesLoader;

impl TablesLoader {
    /// Placeholder load method.
    ///
    /// Returns an empty unit type since TablesOracle has no data yet.
    #[allow(unused_variables)]
    pub fn load(path: &Path) -> LoadResult<()> {
        // TODO: Implement when TablesOracle is expanded
        Ok(())
    }
}
