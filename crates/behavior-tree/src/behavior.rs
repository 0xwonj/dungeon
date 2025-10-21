//! Core behavior trait.
//!
//! This module defines the [`Behavior`] trait, which is the fundamental
//! abstraction for all behavior tree nodes. The trait is generic over a
//! context type `C`, allowing nodes to access game state and make decisions.

use crate::Status;

/// A behavior tree node that can be evaluated against a context.
pub trait Behavior<C>: Send + Sync {
    /// Evaluate this behavior node against the given context.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Mutable reference to the context/blackboard. Nodes can read
    ///   game state and modify it (e.g., to store intermediate results).
    ///
    /// # Returns
    ///
    /// - `Status::Success` if the behavior succeeded
    /// - `Status::Failure` if the behavior failed
    fn tick(&self, ctx: &mut C) -> Status;
}

/// Blanket implementation for boxed behaviors.
///
/// This allows `Box<dyn Behavior<C>>` to also implement `Behavior<C>`,
/// enabling dynamic dispatch and heterogeneous collections of nodes.
impl<C> Behavior<C> for Box<dyn Behavior<C>> {
    #[inline]
    fn tick(&self, ctx: &mut C) -> Status {
        (**self).tick(ctx)
    }
}
