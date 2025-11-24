//! System action provider for reactive action generation.
//!
//! This provider analyzes state deltas and generates reactive system actions
//! in response to game events.

use game_core::{Action, GameState, StateDelta};

use crate::events::{GameEvent, extract_events};
use crate::handlers::{EventContext, HandlerCriticality};
use crate::oracle::OracleBundle;

/// Handler for generating specific types of system actions.
///
/// SystemActionHandlers are composable units that react to GameEvents
/// and generate system actions. They are the building blocks of the
/// SystemActionProvider.
pub trait SystemActionHandler: Send + Sync {
    /// Returns the handler name for logging and debugging.
    fn name(&self) -> &'static str;

    /// Returns execution priority (lower values execute first).
    ///
    /// Priority is used to order handlers within a single event processing pass.
    /// Typical values:
    /// - `-100..0`: Critical system handlers (cost, death)
    /// - `0`: Default priority
    /// - `1..100`: Optional handlers
    fn priority(&self) -> i32 {
        0
    }

    /// Returns the criticality level for error handling.
    fn criticality(&self) -> HandlerCriticality {
        HandlerCriticality::Important
    }

    /// Generate system actions in response to an event.
    ///
    /// Returns a vector of actions to execute. Empty vector means no reaction.
    /// Handlers should pattern match on the event type and return actions only
    /// for events they care about.
    fn generate_actions(&self, event: &GameEvent, ctx: &EventContext) -> Vec<Action>;
}

/// Provider that generates system actions based on state deltas.
///
/// The SystemActionProvider analyzes state changes (deltas) and generates
/// reactive system actions through a collection of handlers. This enables
/// extensible game logic that responds to events like entity death, damage,
/// movement, etc.
///
/// # Architecture
///
/// 1. Extract high-level GameEvents from StateDelta
/// 2. Pass events to all registered handlers
/// 3. Collect generated system actions
/// 4. Return actions for execution (cascading handled by caller)
pub struct SystemActionProvider {
    handlers: Vec<Box<dyn SystemActionHandler>>,
}

impl SystemActionProvider {
    /// Create a new empty provider.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Create a provider with default handlers.
    ///
    /// Default handlers:
    /// - DeathHandler: Remove dead entities from turn scheduling and world
    /// - ActivationHandler: Activate/deactivate NPCs based on player position
    pub fn with_defaults() -> Self {
        use crate::handlers::{ActivationHandler, DeathHandler};

        let mut provider = Self::new();
        provider.add_handler(Box::new(DeathHandler));
        provider.add_handler(Box::new(ActivationHandler));
        provider
    }

    /// Add a handler to the provider.
    ///
    /// Handlers are automatically sorted by priority after addition.
    pub fn add_handler(&mut self, handler: Box<dyn SystemActionHandler>) {
        self.handlers.push(handler);
        self.handlers.sort_by_key(|h| h.priority());
    }

    /// Generate system actions from a state delta.
    ///
    /// This is the core method that:
    /// 1. Extracts events from the delta
    /// 2. Runs all handlers to collect actions
    /// 3. Returns all generated actions
    ///
    /// The caller is responsible for executing these actions and handling
    /// any cascading effects (multi-pass processing).
    pub fn generate_actions(
        &self,
        delta: &StateDelta,
        state_before: &GameState,
        state_after: &GameState,
        oracles: &OracleBundle,
    ) -> Vec<(Action, &'static str, HandlerCriticality)> {
        // Extract high-level events from delta
        let events = extract_events(delta, state_before, state_after);

        if events.is_empty() {
            return vec![];
        }

        // Create context for handlers
        let ctx = EventContext {
            state_before,
            state_after,
            oracles,
        };

        // Collect actions from all handlers
        let mut actions = Vec::new();

        for event in &events {
            for handler in &self.handlers {
                let handler_actions = handler.generate_actions(event, &ctx);

                for action in handler_actions {
                    actions.push((action, handler.name(), handler.criticality()));
                }
            }
        }

        actions
    }

    /// Get all registered handlers (for inspection/debugging).
    pub fn handlers(&self) -> &[Box<dyn SystemActionHandler>] {
        &self.handlers
    }
}

impl Default for SystemActionProvider {
    fn default() -> Self {
        Self::with_defaults()
    }
}
