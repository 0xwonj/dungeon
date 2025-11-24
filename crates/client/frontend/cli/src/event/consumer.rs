//! Maintains the CLI message log in response to runtime events.
use runtime::{Event, GameStateEvent};

use client_frontend_core::{
    EffectVisibility,
    event::{EventConsumer, EventImpact},
    format::format_action_and_effects,
    message::{MessageEntry, MessageLevel, MessageLog},
};

pub struct CliEventConsumer {
    log: MessageLog,
    effect_visibility: EffectVisibility,
}

impl CliEventConsumer {
    pub fn new(log: MessageLog, effect_visibility: EffectVisibility) -> Self {
        Self {
            log,
            effect_visibility,
        }
    }

    fn push_action(
        &mut self,
        action: &game_core::Action,
        action_result: &game_core::ActionResult,
        timestamp: u64,
    ) {
        // Use two-tier message formatting: action message + effect messages
        let (action_msg, effect_msgs) =
            format_action_and_effects(action, action_result, |applied_value| {
                self.effect_visibility.should_show(applied_value)
            });

        // Push the main action message
        self.log.push(MessageEntry::new(
            action_msg,
            Some(timestamp),
            MessageLevel::Info,
        ));

        // Push each effect message
        for effect_msg in effect_msgs {
            self.log.push(MessageEntry::new(
                effect_msg,
                Some(timestamp),
                MessageLevel::Info,
            ));
        }
    }

    fn push_failure(
        &mut self,
        action: &game_core::Action,
        phase: &str,
        error: &str,
        timestamp: u64,
    ) {
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
                action,
                action_result,
                clock,
                ..
            }) => {
                // Filter out system actions from message log
                if !action.actor().is_system() {
                    self.push_action(action, action_result, *clock);
                }
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
            Event::GameState(GameStateEvent::StateRestored {
                from_nonce,
                to_nonce,
            }) => {
                // Game state was restored from a checkpoint
                self.message_log_mut().push_text(format!(
                    "Game state restored: nonce {} → {}",
                    from_nonce, to_nonce
                ));
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
