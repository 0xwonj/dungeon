//! Builder utilities for ergonomic behavior tree construction.
//!
//! This module provides helper functions to reduce boilerplate when building
//! behavior trees. Instead of writing verbose `Box::new(Sequence::new(vec![...]))`,
//! you can use shorter functions like `sequence(vec![...])`.

use crate::{AlwaysSucceed, Behavior, Inverter, Selector, Sequence};

/// Creates a sequence node.
///
/// Shorthand for `Box::new(Sequence::new(children))`.
#[inline]
pub fn sequence<C: 'static>(children: Vec<Box<dyn Behavior<C>>>) -> Box<dyn Behavior<C>> {
    Box::new(Sequence::new(children))
}

/// Creates a selector node.
///
/// Shorthand for `Box::new(Selector::new(children))`.
#[inline]
pub fn selector<C: 'static>(children: Vec<Box<dyn Behavior<C>>>) -> Box<dyn Behavior<C>> {
    Box::new(Selector::new(children))
}

/// Creates an inverter node.
///
/// Shorthand for `Box::new(Inverter::new(child))`.
#[inline]
pub fn inverter<C: 'static>(child: Box<dyn Behavior<C>>) -> Box<dyn Behavior<C>> {
    Box::new(Inverter::new(child))
}

/// Creates an always-succeed node.
///
/// Shorthand for `Box::new(AlwaysSucceed::new(child))`.
#[inline]
pub fn always_succeed<C: 'static>(child: Box<dyn Behavior<C>>) -> Box<dyn Behavior<C>> {
    Box::new(AlwaysSucceed::new(child))
}
