//! Maintains the CLI message log in response to runtime events.
use game_core::action::AppliedValue;
use game_core::{Action, ActionKind, ActionResult, EntityId};
use runtime::{Event, GameStateEvent};

use client_core::{
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

    fn push_action(&mut self, action: &Action, action_result: &ActionResult, timestamp: u64) {
        // Build message from action and result
        let text = match action {
            Action::Character(character_action) => {
                self.format_character_action(character_action, action_result)
            }
            Action::System { kind } => {
                format!("System: {:?}", kind)
            }
        };

        self.log
            .push(MessageEntry::new(text, Some(timestamp), MessageLevel::Info));
    }

    fn format_character_action(
        &self,
        action: &game_core::CharacterAction,
        result: &ActionResult,
    ) -> String {
        let actor_name = self.actor_name(action.actor);

        match action.kind {
            ActionKind::MeleeAttack => {
                // Check if there was damage
                if result.summary.total_damage > 0 {
                    if result.summary.any_critical {
                        format!(
                            "{} CRITICALLY hits for {} damage!",
                            actor_name, result.summary.total_damage
                        )
                    } else {
                        format!(
                            "{} attacks for {} damage",
                            actor_name, result.summary.total_damage
                        )
                    }
                } else {
                    format!("{} attacks but misses!", actor_name)
                }
            }

            ActionKind::Move => {
                // Check for movement in effects
                if let Some(effect) = result.effects.first() {
                    if let AppliedValue::Movement { from, to } = &effect.applied_value {
                        let dx = to.x - from.x;
                        let dy = to.y - from.y;
                        let dir_str = match (dx, dy) {
                            (0, -1) => "north",
                            (0, 1) => "south",
                            (1, 0) => "east",
                            (-1, 0) => "west",
                            (1, -1) => "northeast",
                            (-1, -1) => "northwest",
                            (1, 1) => "southeast",
                            (-1, 1) => "southwest",
                            _ => "somewhere",
                        };
                        format!("{} moves {}", actor_name, dir_str)
                    } else {
                        format!("{} moves", actor_name)
                    }
                } else {
                    format!("{} moves", actor_name)
                }
            }

            ActionKind::Wait => {
                format!("{} waits", actor_name)
            } // TODO: Re-enable when Heal ActionKind is implemented
              // ActionKind::Heal => {
              //     if result.summary.total_healing > 0 {
              //         format!(
              //             "{} heals for {} HP",
              //             actor_name, result.summary.total_healing
              //         )
              //     } else {
              //         format!("{} attempts to heal", actor_name)
              //     }
              // }
        }
    }

    fn push_failure(&mut self, action: &Action, phase: &str, error: &str, timestamp: u64) {
        let text = format!("{} failed during {}: {}", action.actor(), phase, error);
        self.log.push(MessageEntry::new(
            text,
            Some(timestamp),
            MessageLevel::Error,
        ));
    }

    fn actor_name(&self, id: EntityId) -> String {
        if id == EntityId::PLAYER {
            "Player".to_string()
        } else {
            // TODO: Get actor name from oracle
            format!("NPC#{}", id.0)
        }
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
