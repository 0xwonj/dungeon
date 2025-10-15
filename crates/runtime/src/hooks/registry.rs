//! Hook registry for managing and executing post-action hooks.

use std::collections::HashMap;
use std::sync::Arc;

use crate::OracleManager;
use tracing::{debug, error};

use super::{HookCriticality, PostExecutionHook};

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

    /// Executes all root hooks for the given delta and state.
    ///
    /// Only root hooks are evaluated in priority order. Lookup-only hooks
    /// are executed only when explicitly chained from another hook.
    ///
    /// # Error Handling
    ///
    /// Hook execution errors are handled based on criticality level:
    /// - `Critical`: Returns error immediately, failing the action
    /// - `Important`: Logs error and continues to next hook (default)
    /// - `Optional`: Logs at debug level and continues silently
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all critical hooks succeeded
    /// - `Err(e)` if a critical hook failed
    pub fn execute_hooks(
        &self,
        delta: &game_core::StateDelta,
        state: &mut game_core::GameState,
        oracles: &OracleManager,
    ) -> Result<(), game_core::ExecuteError> {
        let env = oracles.as_game_env();

        // Only execute root hooks - lookup hooks are called via chaining
        for hook in self.root_hooks.iter() {
            if let Err(e) = hook.execute(delta, state, oracles, &env, self, 0) {
                self.handle_hook_error(hook.as_ref(), e)?;
            }
        }

        Ok(())
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

    /// Handles hook execution errors based on criticality level.
    ///
    /// Returns Ok(()) for Important/Optional hooks, Err for Critical hooks.
    fn handle_hook_error(
        &self,
        hook: &dyn PostExecutionHook,
        error: game_core::ExecuteError,
    ) -> Result<(), game_core::ExecuteError> {
        let (level, message) = match hook.criticality() {
            HookCriticality::Critical => {
                error!(
                    target: "runtime::hooks",
                    hook = hook.name(),
                    criticality = "critical",
                    error = ?error,
                    "Critical hook failed, aborting action"
                );
                return Err(error);
            }
            HookCriticality::Important => ("important", "Hook failed, continuing"),
            HookCriticality::Optional => ("optional", "Optional hook failed"),
        };

        match hook.criticality() {
            HookCriticality::Important => error!(
                target: "runtime::hooks",
                hook = hook.name(),
                criticality = level,
                error = ?error,
                "{}", message
            ),
            HookCriticality::Optional => debug!(
                target: "runtime::hooks",
                hook = hook.name(),
                criticality = level,
                error = ?error,
                "{}", message
            ),
            HookCriticality::Critical => unreachable!(),
        }

        Ok(())
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::default_hooks()
    }
}
