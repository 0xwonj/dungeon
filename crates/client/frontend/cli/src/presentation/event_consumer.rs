//! Maintains the CLI message log in response to runtime events.
use game_core::{Action, ActionKind};
use runtime::GameEvent;

use frontend_core::{
    event::{EventConsumer, EventImpact},
    message::{MessageEntry, MessageLevel, MessageLog},
};

pub struct CliEventConsumer {
    log: MessageLog,
}

impl CliEventConsumer {
    pub fn new(log: MessageLog) -> Self {
        Self { log }
    }

    fn push_action(&mut self, action: &Action, timestamp: u64) {
        let text = match &action.kind {
            ActionKind::Move(movement) => format!(
                "{} moves {:?} by {}",
                action.actor, movement.direction, movement.distance
            ),
            ActionKind::Wait => format!("{} waits", action.actor),
            other => format!("{} performs {:?}", action.actor, other),
        };
        self.log
            .push(MessageEntry::new(text, Some(timestamp), MessageLevel::Info));
    }

    fn push_failure(&mut self, action: &Action, phase: &str, error: &str, timestamp: u64) {
        let text = format!("{} failed during {}: {}", action.actor, phase, error);
        self.log.push(MessageEntry::new(
            text,
            Some(timestamp),
            MessageLevel::Error,
        ));
    }
}

impl EventConsumer for CliEventConsumer {
    fn on_event(&mut self, event: &GameEvent) -> EventImpact {
        match event {
            GameEvent::TurnCompleted { .. } => EventImpact::redraw(),
            GameEvent::ActionExecuted {
                action,
                delta: _,
                clock,
            } => {
                self.push_action(action, clock.0);
                // TODO: Use delta for more detailed feedback (e.g., "HP -5", "Item acquired")
                EventImpact::redraw()
            }
            GameEvent::ActionFailed {
                action,
                phase,
                error,
                clock,
            } => {
                self.push_failure(action, phase.as_str(), error, clock.0);
                EventImpact::redraw()
            }
        }
    }

    fn message_log(&self) -> &MessageLog {
        &self.log
    }

    fn message_log_mut(&mut self) -> &mut MessageLog {
        &mut self.log
    }

    fn take_message_log(self) -> MessageLog {
        self.log
    }
}
