//! Utilities for reacting to runtime events inside UI layers.
use runtime::GameEvent;

use crate::message::MessageLog;

#[derive(Clone, Copy, Debug, Default)]
pub struct EventImpact {
    pub requires_redraw: bool,
}

impl EventImpact {
    pub const fn none() -> Self {
        Self {
            requires_redraw: false,
        }
    }

    pub const fn redraw() -> Self {
        Self {
            requires_redraw: true,
        }
    }

    pub fn combine(self, other: Self) -> Self {
        Self {
            requires_redraw: self.requires_redraw || other.requires_redraw,
        }
    }
}

pub trait EventConsumer {
    fn on_event(&mut self, event: &GameEvent) -> EventImpact;
    fn message_log(&self) -> &MessageLog;
    fn message_log_mut(&mut self) -> &mut MessageLog;
    fn take_message_log(self) -> MessageLog
    where
        Self: Sized;
}
