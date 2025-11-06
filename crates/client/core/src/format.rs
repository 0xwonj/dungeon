//! Structured message formatting for actions and effects.
//!
//! Two-tier message system:
//! 1. Action message: "Actor performs action on target"
//! 2. Effect messages: Individual results for each affected entity

use game_core::{
    Action, ActionKind, ActionResult, EntityId,
    action::{ActionInput, AppliedValue, EffectResult},
};

/// Formats the primary action message.
///
/// This describes what action was performed and the primary target/direction.
/// Examples:
/// - "Player attacks Goblin#5"
/// - "Player moves north"
/// - "Wizard casts Fireball at (12, 8)"
pub fn format_action_message(action: &Action, actor_name: &str) -> String {
    match action {
        Action::Character(char_action) => {
            let kind_verb = match char_action.kind {
                ActionKind::MeleeAttack => "attacks",
                ActionKind::Move => "moves",
                ActionKind::Wait => "waits",
            };

            match &char_action.input {
                ActionInput::Entity(target_id) => {
                    let target_name = entity_name(*target_id);
                    format!("{} {} {}", actor_name, kind_verb, target_name)
                }
                ActionInput::Direction(dir) => {
                    let dir_str = format!("{:?}", dir).to_lowercase();
                    format!("{} {} {}", actor_name, kind_verb, dir_str)
                }
                ActionInput::Position(pos) => {
                    format!("{} {} at ({}, {})", actor_name, kind_verb, pos.x, pos.y)
                }
                ActionInput::None => {
                    format!("{} {}", actor_name, kind_verb)
                }
                ActionInput::Entities(targets) => {
                    if targets.is_empty() {
                        format!("{} {}", actor_name, kind_verb)
                    } else if targets.len() == 1 {
                        let target_name = entity_name(targets[0]);
                        format!("{} {} {}", actor_name, kind_verb, target_name)
                    } else {
                        format!("{} {} {} targets", actor_name, kind_verb, targets.len())
                    }
                }
            }
        }
        Action::System { kind } => {
            format!("System: {:?}", kind)
        }
    }
}

/// Formats effect result messages with visibility filtering.
///
/// Returns a list of messages describing what happened to each affected entity.
/// Only effects allowed by the visibility filter are included.
///
/// Examples:
/// - "Goblin#5 takes 12 damage (critical!)"
/// - "Goblin#5 takes 8 damage"
/// - "Player moves from (5, 3) to (5, 4)"
/// - "Goblin#5 is poisoned for 3 turns"
pub fn format_effect_messages<F>(effects: &[EffectResult], should_show: F) -> Vec<String>
where
    F: Fn(&AppliedValue) -> bool,
{
    effects
        .iter()
        .filter(|effect| should_show(&effect.applied_value))
        .filter_map(|effect| {
            let target_name = entity_name(effect.target);

            match &effect.applied_value {
                AppliedValue::Damage { actual, .. } => {
                    if *actual > 0 {
                        let mut msg = format!("{} takes {} damage", target_name, actual);
                        if effect.flags.critical {
                            msg.push_str(" (critical!)");
                        }
                        if effect.flags.resisted {
                            msg.push_str(" (resisted)");
                        }
                        if effect.flags.blocked {
                            msg.push_str(" (blocked)");
                        }
                        Some(msg)
                    } else {
                        Some(format!("{} takes no damage", target_name))
                    }
                }

                AppliedValue::Healing { actual, .. } => {
                    if *actual > 0 {
                        let mut msg = format!("{} heals {} HP", target_name, actual);
                        if effect.flags.overheal {
                            msg.push_str(" (overheal)");
                        }
                        Some(msg)
                    } else {
                        None // No healing happened, skip message
                    }
                }

                AppliedValue::Movement { from, to } => Some(format!(
                    "{} moves from ({}, {}) to ({}, {})",
                    target_name, from.x, from.y, to.x, to.y
                )),

                AppliedValue::StatusApplied { status, duration } => Some(format!(
                    "{} is affected by {:?} for {} turns",
                    target_name, status, duration
                )),

                AppliedValue::StatusRemoved { status } => Some(format!(
                    "{} is no longer affected by {:?}",
                    target_name, status
                )),

                AppliedValue::ResourceChange { resource, delta } => {
                    if *delta > 0 {
                        Some(format!("{} gains {} {:?}", target_name, delta, resource))
                    } else if *delta < 0 {
                        Some(format!(
                            "{} loses {} {:?}",
                            target_name,
                            delta.abs(),
                            resource
                        ))
                    } else {
                        None // No change
                    }
                }

                AppliedValue::Summon { entity_id } => {
                    let summoned_name = entity_name(*entity_id);
                    Some(format!("{} summons {}", target_name, summoned_name))
                }

                AppliedValue::None => None, // No message for empty effects
            }
        })
        .collect()
}

/// Formats a complete action with all its effects.
///
/// Returns a tuple of (action_message, effect_messages).
/// Effect messages are filtered according to the visibility predicate.
pub fn format_action_and_effects<F>(
    action: &Action,
    result: &ActionResult,
    should_show: F,
) -> (String, Vec<String>)
where
    F: Fn(&AppliedValue) -> bool,
{
    let actor_name = entity_name(action.actor());
    let action_msg = format_action_message(action, &actor_name);
    let effect_msgs = format_effect_messages(&result.effects, should_show);

    (action_msg, effect_msgs)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Returns a display name for an entity.
fn entity_name(id: EntityId) -> String {
    if id == EntityId::PLAYER {
        "Player".to_string()
    } else if id.is_system() {
        "System".to_string()
    } else {
        // TODO: Get actual name from oracle
        format!("NPC#{}", id.0)
    }
}
