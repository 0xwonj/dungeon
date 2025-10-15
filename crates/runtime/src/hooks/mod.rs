//! Post-execution hook system for runtime orchestration.
//!
//! Hooks provide a flexible way to trigger system actions after player/NPC actions.
//! Each hook can inspect the state delta and conditionally generate system actions
//! that are executed through the same validation pipeline.
//!
//! # Architecture
//!
//! - Hooks are registered in the RuntimeBuilder and sorted by priority
//! - After each action execution, hooks are evaluated in priority order
//! - Hooks can chain to other hooks via `next_hook_names()`, enabling reactive cascades
//! - Hooks that trigger create system actions which are executed immediately
//! - All state mutations remain auditable through the action pipeline
//!
//! # Hook Chaining
//!
//! Hooks can specify next hooks to execute after their action completes:
//! - DamageHook → DeathCheckHook → OnDeathHook → DamageHook (recursive!)
//! - Chains automatically terminate when `should_trigger()` returns false
//! - Maximum depth limit prevents infinite loops

mod action_cost;
mod activation;
mod context;
mod damage;
mod registry;

pub use action_cost::ActionCostHook;
pub use activation::ActivationHook;
pub use context::HookContext;
pub use damage::DamageHook;
pub use registry::HookRegistry;

use game_core::Action;

/// Defines the criticality level of a hook for error handling.
///
/// This enum determines how hook failures are handled during execution:
/// - Critical hooks must succeed or the action fails
/// - Important hooks log errors but allow continuation
/// - Optional hooks can fail silently
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookCriticality {
    /// Hook failure should fail the entire action.
    ///
    /// Use for hooks that maintain game state consistency (e.g., ActionCostHook).
    /// If a critical hook fails, the action is rolled back or marked as failed.
    Critical,

    /// Hook failure should be logged as error but allow continuation.
    ///
    /// This is the default level. Use for hooks that have side effects but
    /// aren't essential for game state consistency (e.g., ActivationHook).
    Important,

    /// Hook failure is expected and can be silently ignored.
    ///
    /// Use for cosmetic or optional effects that don't impact gameplay
    /// (e.g., visual effects, sound hooks, achievement notifications).
    Optional,
}

/// Post-execution hook that can generate system actions based on state changes.
///
/// Hooks follow the Strategy pattern, allowing different behaviors to be composed
/// at runtime. Each hook examines the delta from an executed action and optionally
/// generates a system action to be executed.
///
/// # Execution Order
///
/// Hooks are sorted by priority (lower values execute first):
/// - Negative priorities: Critical system hooks (cost, activation)
/// - Zero: Default priority for most hooks
/// - Positive priorities: Optional or cosmetic hooks
pub trait PostExecutionHook: Send + Sync {
    /// Returns a human-readable name for this hook (used in logging and debugging).
    fn name(&self) -> &'static str;

    /// Returns the execution priority for root-level hook ordering.
    ///
    /// Lower values execute first. Typical ranges:
    /// - `-100..0`: Critical system hooks that must run early
    /// - `0`: Default priority for most hooks
    /// - `1..100`: Optional or cosmetic hooks
    ///
    /// Note: Priority only affects root hook ordering. Chained hooks execute
    /// in the order specified by `next_hook_names()`.
    fn priority(&self) -> i32 {
        0
    }

    /// Returns the criticality level of this hook for error handling.
    ///
    /// - `Critical`: Hook failure causes the entire action to fail
    /// - `Important`: Hook failure is logged but execution continues (default)
    /// - `Optional`: Hook failure is silently ignored
    ///
    /// This allows the system to distinguish between essential hooks (like ActionCost)
    /// and optional hooks (like cosmetic effects).
    fn criticality(&self) -> HookCriticality {
        HookCriticality::Important
    }

    /// Determines whether this hook should trigger based on the execution context.
    ///
    /// This method is called for every action execution. Hooks should check the
    /// delta to see if they need to generate a system action.
    fn should_trigger(&self, ctx: &HookContext<'_>) -> bool;

    /// Creates system actions to be executed if this hook triggers.
    ///
    /// Returns a vector of actions to execute. Each action must have `EntityId::SYSTEM` as the actor.
    /// Empty vec means no actions to execute.
    ///
    /// This allows hooks to generate multiple actions when needed (e.g., applying damage to multiple entities).
    fn create_actions(&self, _ctx: &HookContext<'_>) -> Vec<Action> {
        // Default: try old create_action for backward compatibility
        vec![]
    }

    /// Returns names of hooks to execute after this hook's action completes.
    ///
    /// Enables hook chaining for reactive cascades:
    /// - damage → death_check → on_death → damage (recursive)
    /// - Chains terminate when `should_trigger()` returns false
    /// - Empty slice (default) means no chaining
    fn next_hook_names(&self) -> &[&'static str] {
        &[]
    }
}
