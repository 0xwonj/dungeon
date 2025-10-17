//! Maintains the CLI message log in response to runtime events.
use game_core::{Action, CharacterActionKind};
use runtime::{Event, GameStateEvent};

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
        let text = match action {
            Action::Character { actor, kind } => match kind {
                CharacterActionKind::Move(movement) => format!(
                    "{} moves {:?} by {}",
                    actor, movement.direction, movement.distance
                ),
                CharacterActionKind::Wait => format!("{} waits", actor),
                other => format!("{} performs {:?}", actor, other),
            },
            Action::System { kind } => format!("System: {:?}", kind),
        };
        self.log
            .push(MessageEntry::new(text, Some(timestamp), MessageLevel::Info));
    }

    fn push_failure(&mut self, action: &Action, phase: &str, error: &str, timestamp: u64) {
        let text = format!("{} failed during {}: {}", action.actor(), phase, error);
        self.log.push(MessageEntry::new(
            text,
            Some(timestamp),
            MessageLevel::Error,
        ));
    }
}

impl EventConsumer for CliEventConsumer {
    fn on_event(&mut self, event: &Event) -> EventImpact {
        match event {
            Event::GameState(GameStateEvent::ActionExecuted {
                nonce: _,
                action,
                delta: _,
                clock,
                before_state: _,
                after_state: _,
            }) => {
                // Filter out system actions from message log
                if !action.actor().is_system() {
                    self.push_action(action, *clock);
                }
                // TODO: Use delta for more detailed feedback (e.g., "HP -5", "Item acquired")
                EventImpact::redraw()
            }
            Event::GameState(GameStateEvent::ActionFailed {
                nonce: _,
                action,
                phase,
                error,
                clock,
            }) => {
                // Filter out system actions from message log
                if !action.actor().is_system() {
                    self.push_failure(action, phase.as_str(), error, *clock);
                }
                EventImpact::redraw()
            }
            Event::Proof(_) => {
                // Proof events are not displayed in CLI to keep focus on gameplay
                EventImpact::none()
            }
            Event::ActionRef(_) => {
                // ActionRef is for internal persistence only, not displayed to user
                EventImpact::none()
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
