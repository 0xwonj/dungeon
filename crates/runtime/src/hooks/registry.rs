//! Hook registry for managing and executing post-action hooks.

use std::collections::HashMap;
use std::sync::Arc;

use super::PostExecutionHook;

/// Registry that manages and executes post-execution hooks.
///
/// The registry separates hooks into two categories:
/// - **Root hooks**: Execute on every action, checked via `should_trigger()`
/// - **Lookup hooks**: Only execute when explicitly chained from another hook
///
/// This separation improves performance by avoiding unnecessary `should_trigger()`
/// calls for hooks that are only meant to be chained.
///
/// # Design Pattern
///
/// HookRegistry implements the Chain of Responsibility pattern, where each
/// hook in the chain has the opportunity to handle (generate an action for)
/// the execution context.
pub struct HookRegistry {
    root_hooks: Arc<[Arc<dyn PostExecutionHook>]>,
    lookup_table: HashMap<&'static str, Arc<dyn PostExecutionHook>>,
}

impl HookRegistry {
    /// Creates a new hook registry with separate root and lookup hooks.
    ///
    /// # Arguments
    ///
    /// * `root_hooks` - Hooks that execute on every action (checked via `should_trigger()`)
    /// * `all_hooks` - All hooks (root + lookup-only) for the lookup table
    ///
    /// Root hooks are automatically sorted by priority (lower values first).
    pub fn new(
        mut root_hooks: Vec<Arc<dyn PostExecutionHook>>,
        all_hooks: Vec<Arc<dyn PostExecutionHook>>,
    ) -> Self {
        root_hooks.sort_by_key(|h| h.priority());

        let lookup_table = all_hooks
            .iter()
            .map(|h| (h.name(), Arc::clone(h)))
            .collect();

        Self {
            root_hooks: root_hooks.into(),
            lookup_table,
        }
    }

    /// Creates a registry with the default set of hooks.
    ///
    /// Default hooks include:
    /// - ActionCostHook: Updates actor ready_at timestamps (root)
    /// - ActivationHook: Manages entity activation based on player proximity (root)
    pub fn default_hooks() -> Self {
        use super::{ActionCostHook, ActivationHook};

        let root_hooks = vec![
            Arc::new(ActionCostHook) as Arc<dyn PostExecutionHook>,
            Arc::new(ActivationHook) as Arc<dyn PostExecutionHook>,
        ];

        // For default, root and all hooks are the same
        let all_hooks = root_hooks.clone();

        Self::new(root_hooks, all_hooks)
    }

    /// Returns an iterator over root hooks in priority order.
    ///
    /// Root hooks are executed on every action. Lookup-only hooks
    /// are executed only when explicitly chained from another hook.
    pub fn root_hooks(&self) -> &[Arc<dyn PostExecutionHook>] {
        &self.root_hooks
    }

    /// Finds a hook by name from the lookup table.
    ///
    /// Used by hooks to look up next hooks in their chain.
    /// This uses the lookup table for O(1) access instead of linear search.
    pub fn find(&self, name: &str) -> Option<&Arc<dyn PostExecutionHook>> {
        self.lookup_table.get(name)
    }

    /// Returns the number of root hooks.
    pub fn len(&self) -> usize {
        self.root_hooks.len()
    }

    /// Returns true if no root hooks are registered.
    pub fn is_empty(&self) -> bool {
        self.root_hooks.is_empty()
    }

    /// Returns the total number of hooks in the lookup table.
    pub fn total_hooks(&self) -> usize {
        self.lookup_table.len()
    }

    /// Returns an iterator over root hook names and priorities (for debugging).
    pub fn hooks(&self) -> impl Iterator<Item = (&'static str, i32)> + '_ {
        self.root_hooks.iter().map(|h| (h.name(), h.priority()))
    }

    /// Returns an iterator over all hook names in the lookup table (for debugging).
    pub fn all_hook_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.lookup_table.keys().copied()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::default_hooks()
    }
}
