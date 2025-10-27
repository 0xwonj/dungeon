//! Decorator behavior nodes.
//!
//! Decorators wrap a single child behavior and modify its result or execution.
//! This module provides [`Inverter`] (NOT logic) and [`AlwaysSucceed`] (error suppression).

use crate::{Behavior, Status};

/// Inverts the result of its child behavior.
///
/// # Semantics
///
/// - If the child returns `Success`, the inverter returns `Failure`
/// - If the child returns `Failure`, the inverter returns `Success`
///
/// This is analogous to a logical NOT (!) operation.
pub struct Inverter<C> {
    child: Box<dyn Behavior<C>>,
}

impl<C> Inverter<C> {
    /// Creates a new inverter that wraps the given child behavior.
    pub fn new(child: Box<dyn Behavior<C>>) -> Self {
        Self { child }
    }
}

impl<C> Behavior<C> for Inverter<C> {
    fn tick(&self, ctx: &mut C) -> Status {
        self.child.tick(ctx).invert()
    }
}

/// Always returns `Success`, regardless of the child's result.
///
/// # Semantics
///
/// - If the child returns `Success`, returns `Success`
/// - If the child returns `Failure`, **still returns `Success`**
///
/// This is useful for:
/// - Optional behaviors that shouldn't cause a sequence to fail
/// - Logging/debugging nodes that observe state without affecting control flow
/// - Error suppression in non-critical paths
pub struct AlwaysSucceed<C> {
    child: Box<dyn Behavior<C>>,
}

impl<C> AlwaysSucceed<C> {
    /// Creates a new always-succeed wrapper around the given child behavior.
    pub fn new(child: Box<dyn Behavior<C>>) -> Self {
        Self { child }
    }
}

impl<C> Behavior<C> for AlwaysSucceed<C> {
    fn tick(&self, ctx: &mut C) -> Status {
        // Execute child but ignore the result
        let _ = self.child.tick(ctx);
        Status::Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext {
        value: i32,
    }

    struct IsPositive;
    impl Behavior<TestContext> for IsPositive {
        fn tick(&self, ctx: &mut TestContext) -> Status {
            if ctx.value > 0 {
                Status::Success
            } else {
                Status::Failure
            }
        }
    }

    struct Increment;
    impl Behavior<TestContext> for Increment {
        fn tick(&self, ctx: &mut TestContext) -> Status {
            ctx.value += 1;
            Status::Success
        }
    }

    struct FailAndIncrement;
    impl Behavior<TestContext> for FailAndIncrement {
        fn tick(&self, ctx: &mut TestContext) -> Status {
            ctx.value += 1;
            Status::Failure
        }
    }

    #[test]
    fn inverter_inverts_success() {
        let inverter = Inverter::new(Box::new(IsPositive));

        let mut ctx = TestContext { value: 10 };
        assert_eq!(inverter.tick(&mut ctx), Status::Failure);
    }

    #[test]
    fn inverter_inverts_failure() {
        let inverter = Inverter::new(Box::new(IsPositive));

        let mut ctx = TestContext { value: -10 };
        assert_eq!(inverter.tick(&mut ctx), Status::Success);
    }

    #[test]
    fn always_succeed_on_success() {
        let always = AlwaysSucceed::new(Box::new(Increment));

        let mut ctx = TestContext { value: 0 };
        assert_eq!(always.tick(&mut ctx), Status::Success);
        assert_eq!(ctx.value, 1);
    }

    #[test]
    fn always_succeed_on_failure() {
        let always = AlwaysSucceed::new(Box::new(FailAndIncrement));

        let mut ctx = TestContext { value: 0 };
        assert_eq!(always.tick(&mut ctx), Status::Success);
        assert_eq!(ctx.value, 1); // Child still executed
    }
}
