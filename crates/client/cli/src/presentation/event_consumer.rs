//! Maintains the CLI message log in response to runtime events.
use game_core::{Action, ActionResult, AttackOutcome, CharacterActionKind, EntityId};
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
        let text = match (action, action_result) {
            // Attack with result
            (
                Action::Character {
                    actor,
                    kind: CharacterActionKind::Attack(attack),
                },
                ActionResult::Attack(attack_result),
            ) => {
                let attacker_name = self.actor_name(*actor);
                let defender_name = self.actor_name(attack.target);

                match attack_result.outcome {
                    AttackOutcome::Miss => {
                        format!("{} attacks {} but misses!", attacker_name, defender_name)
                    }
                    AttackOutcome::Hit => {
                        let damage = attack_result.damage.unwrap_or(0);
                        format!(
                            "{} attacks {} for {} damage",
                            attacker_name, defender_name, damage
                        )
                    }
                    AttackOutcome::Critical => {
                        let damage = attack_result.damage.unwrap_or(0);
                        format!(
                            "{} CRITICALLY hits {} for {} damage!",
                            attacker_name, defender_name, damage
                        )
                    }
                }
            }

            // Move
            (
                Action::Character {
                    actor,
                    kind: CharacterActionKind::Move(movement),
                },
                ActionResult::Move,
            ) => {
                format!("{} moves {:?}", self.actor_name(*actor), movement.direction)
            }

            // Wait
            (
                Action::Character {
                    actor,
                    kind: CharacterActionKind::Wait(_),
                },
                ActionResult::Wait,
            ) => {
                format!("{} waits", self.actor_name(*actor))
            }

            // Other actions (UseItem, Interact, etc.)
            (Action::Character { actor, kind }, _) => {
                format!("{} performs {:?}", self.actor_name(*actor), kind)
            }

            // System actions
            (Action::System { kind }, _) => {
                format!("System: {:?}", kind)
            }
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
