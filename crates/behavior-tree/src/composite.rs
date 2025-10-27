//! Composite behavior nodes.
//!
//! Composite nodes control the execution flow of multiple child behaviors.
//! This module provides the fundamental building blocks for creating complex
//! decision trees: [`Sequence`] (AND logic) and [`Selector`] (OR logic).

use crate::{Behavior, Status};

/// Type alias for a scored behavior option in utility-based decision making.
///
/// Each option consists of:
/// - A behavior to execute
/// - A scoring function that evaluates desirability (0-100)
type ScoredOption<C> = (Box<dyn Behavior<C>>, Box<dyn Fn(&C) -> u32 + Send + Sync>);

/// Executes child behaviors in sequence until one fails.
///
/// # Semantics
///
/// A `Sequence` node evaluates its children from left to right:
/// - If a child returns `Failure`, the sequence **stops immediately** and returns `Failure`
/// - If a child returns `Success`, the sequence **continues** to the next child
/// - If all children return `Success`, the sequence returns `Success`
///
/// This is analogous to a short-circuited logical AND (&&) operation.
pub struct Sequence<C> {
    children: Vec<Box<dyn Behavior<C>>>,
}

impl<C> Sequence<C> {
    /// Creates a new sequence with the given child behaviors.
    ///
    /// # Panics
    ///
    /// Panics if `children` is empty. A sequence with no children is
    /// meaningless and likely indicates a programming error.
    pub fn new(children: Vec<Box<dyn Behavior<C>>>) -> Self {
        assert!(
            !children.is_empty(),
            "Sequence must have at least one child"
        );
        Self { children }
    }
}

impl<C> Behavior<C> for Sequence<C> {
    fn tick(&self, ctx: &mut C) -> Status {
        // Execute children in order until one fails
        for child in &self.children {
            match child.tick(ctx) {
                Status::Success => continue,               // Move to next child
                Status::Failure => return Status::Failure, // Short-circuit
            }
        }
        // All children succeeded
        Status::Success
    }
}

/// Executes child behaviors in sequence until one succeeds.
///
/// # Semantics
///
/// A `Selector` node evaluates its children from left to right:
/// - If a child returns `Success`, the selector **stops immediately** and returns `Success`
/// - If a child returns `Failure`, the selector **continues** to the next child
/// - If all children return `Failure`, the selector returns `Failure`
///
/// This is analogous to a short-circuited logical OR (||) operation.
pub struct Selector<C> {
    children: Vec<Box<dyn Behavior<C>>>,
}

/// Executes child behaviors based on utility scores.
///
/// # Semantics
///
/// A `UtilitySelector` node evaluates all children's scores and picks the highest:
/// - Each child has an associated scoring function that returns a score (0-100)
/// - All scoring functions are evaluated before execution
/// - The child with the highest score is executed
/// - If all scores are 0, the selector returns `Failure`
///
/// # Design
///
/// This is a **utility-based** decision system rather than priority-based.
/// Unlike `Selector` which tries children in fixed order, `UtilitySelector`
/// dynamically chooses based on current context.
///
/// # Example
///
/// ```rust,ignore
/// use behavior_tree::UtilitySelector;
///
/// let selector = UtilitySelector::new(vec![
///     (flee_behavior(), Box::new(|ctx| compute_flee_score(ctx))),
///     (attack_behavior(), Box::new(|ctx| compute_attack_score(ctx))),
/// ]);
/// ```
pub struct UtilitySelector<C> {
    options: Vec<ScoredOption<C>>,
}

impl<C> Selector<C> {
    /// Creates a new selector with the given child behaviors.
    ///
    /// # Panics
    ///
    /// Panics if `children` is empty. A selector with no children is
    /// meaningless and likely indicates a programming error.
    pub fn new(children: Vec<Box<dyn Behavior<C>>>) -> Self {
        assert!(
            !children.is_empty(),
            "Selector must have at least one child"
        );
        Self { children }
    }
}

impl<C> Behavior<C> for Selector<C> {
    fn tick(&self, ctx: &mut C) -> Status {
        // Try children in order until one succeeds
        for child in &self.children {
            match child.tick(ctx) {
                Status::Success => return Status::Success, // Short-circuit
                Status::Failure => continue,               // Try next child
            }
        }
        // All children failed
        Status::Failure
    }
}

impl<C> UtilitySelector<C> {
    /// Creates a new utility selector with the given options.
    ///
    /// Each option is a tuple of (behavior, scoring_function).
    /// The scoring function takes a read-only reference to the context
    /// and returns a score from 0 to 100.
    ///
    /// # Panics
    ///
    /// Panics if `options` is empty.
    pub fn new(options: Vec<ScoredOption<C>>) -> Self {
        assert!(
            !options.is_empty(),
            "UtilitySelector must have at least one option"
        );
        Self { options }
    }
}

impl<C> Behavior<C> for UtilitySelector<C> {
    fn tick(&self, ctx: &mut C) -> Status {
        // Compute scores for all options
        let mut scores: Vec<(usize, u32)> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, (_, scorer))| (i, scorer(ctx)))
            .collect();

        // Sort by score (descending)
        scores.sort_by(|a, b| b.1.cmp(&a.1));

        // Execute highest-scoring option
        if let Some((best_idx, score)) = scores.first()
            && *score > 0
        {
            return self.options[*best_idx].0.tick(ctx);
        }

        // All options scored 0 (all impossible/undesirable)
        Status::Failure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext {
        value: i32,
    }

    struct Increment;
    impl Behavior<TestContext> for Increment {
        fn tick(&self, ctx: &mut TestContext) -> Status {
            ctx.value += 1;
            Status::Success
        }
    }

    struct Decrement;
    impl Behavior<TestContext> for Decrement {
        fn tick(&self, ctx: &mut TestContext) -> Status {
            ctx.value -= 1;
            Status::Success
        }
    }

    struct FailAlways;
    impl Behavior<TestContext> for FailAlways {
        fn tick(&self, _ctx: &mut TestContext) -> Status {
            Status::Failure
        }
    }

    #[test]
    fn sequence_all_success() {
        let seq = Sequence::new(vec![Box::new(Increment), Box::new(Increment)]);

        let mut ctx = TestContext { value: 0 };
        assert_eq!(seq.tick(&mut ctx), Status::Success);
        assert_eq!(ctx.value, 2);
    }

    #[test]
    fn sequence_fails_on_first_failure() {
        let seq = Sequence::new(vec![
            Box::new(Increment),
            Box::new(FailAlways),
            Box::new(Increment), // Should not execute
        ]);

        let mut ctx = TestContext { value: 0 };
        assert_eq!(seq.tick(&mut ctx), Status::Failure);
        assert_eq!(ctx.value, 1); // Only first increment executed
    }

    #[test]
    fn selector_succeeds_on_first_success() {
        let sel = Selector::new(vec![
            Box::new(FailAlways),
            Box::new(Increment),
            Box::new(Decrement), // Should not execute
        ]);

        let mut ctx = TestContext { value: 0 };
        assert_eq!(sel.tick(&mut ctx), Status::Success);
        assert_eq!(ctx.value, 1); // Only Increment executed
    }

    #[test]
    fn selector_fails_when_all_fail() {
        let sel = Selector::new(vec![Box::new(FailAlways), Box::new(FailAlways)]);

        let mut ctx = TestContext { value: 0 };
        assert_eq!(sel.tick(&mut ctx), Status::Failure);
    }
}
