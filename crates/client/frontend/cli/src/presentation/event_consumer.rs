//! Maintains the CLI message log in response to runtime events.
use game_core::{Action, ActionKind};
use runtime::{Event, GameStateEvent, ProofEvent, TurnEvent};

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
    fn on_event(&mut self, event: &Event) -> EventImpact {
        match event {
            Event::Turn(TurnEvent { .. }) => EventImpact::redraw(),
            Event::GameState(GameStateEvent::ActionExecuted {
                action,
                delta: _,
                clock,
                before_state: _,
                after_state: _,
            }) => {
                self.push_action(action, *clock);
                // TODO: Use delta for more detailed feedback (e.g., "HP -5", "Item acquired")
                EventImpact::redraw()
            }
            Event::GameState(GameStateEvent::ActionFailed {
                action,
                phase,
                error,
                clock,
            }) => {
                self.push_failure(action, phase.as_str(), error, *clock);
                EventImpact::redraw()
            }
            Event::Proof(ProofEvent::ProofStarted { action, clock }) => {
                let text = format!("🔐 Generating proof for {} at tick {}", action.actor, clock);
                self.log
                    .push(MessageEntry::new(text, Some(*clock), MessageLevel::Info));
                EventImpact::redraw()
            }
            Event::Proof(ProofEvent::ProofGenerated {
                action,
                clock,
                generation_time_ms,
                ..
            }) => {
                let text = format!(
                    "✅ Proof generated for {} at tick {} ({}ms)",
                    action.actor, clock, generation_time_ms
                );
                self.log
                    .push(MessageEntry::new(text, Some(*clock), MessageLevel::Info));
                EventImpact::redraw()
            }
            Event::Proof(ProofEvent::ProofFailed {
                action,
                clock,
                error,
            }) => {
                let text = format!(
                    "❌ Proof failed for {} at tick {}: {}",
                    action.actor, clock, error
                );
                self.log
                    .push(MessageEntry::new(text, Some(*clock), MessageLevel::Error));
                EventImpact::redraw()
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
