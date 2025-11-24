//! Event extraction from state deltas.
//!
//! This module provides functions to convert low-level StateDelta into
//! high-level GameEvent instances.

use game_core::{ActorFields, GameState, StateDelta, SystemActionKind};

use super::game_event::{GameEvent, HealthThreshold};

/// Extract high-level game events from a state delta.
///
/// This function analyzes the delta and produces semantically meaningful events
/// that event handlers can react to. Multiple events may be generated from a
/// single delta (e.g., damage taken + entity died).
///
/// # Parameters
///
/// - `delta`: The state changes that occurred
/// - `state_before`: Game state before the action
/// - `state_after`: Game state after the action
///
/// # Event Ordering
///
/// Events are ordered by semantic priority:
/// 1. ActionCompleted (if non-system action)
/// 2. Entity state changes (damage, movement, etc.)
/// 3. Derived events (death, threshold crossing)
pub fn extract_events(
    delta: &StateDelta,
    state_before: &GameState,
    state_after: &GameState,
) -> Vec<GameEvent> {
    let mut events = Vec::new();

    // Debug: Check if delta is empty
    if delta.is_empty() {
        tracing::warn!(
            target: "runtime::events",
            "extract_events called with empty delta for action: {:?}",
            delta.action.as_snake_case()
        );
    }

    // Always emit ActionCompleted for non-system actions
    if !delta.action.actor().is_system() {
        let actor_id = delta.action.actor();

        // Calculate actual cost from ready_at delta
        // Cost is already applied within the action, so we extract it from state delta
        let cost = if let Some(actor_before) = state_before.entities.actor(actor_id) {
            if let Some(actor_after) = state_after.entities.actor(actor_id) {
                if let (Some(ready_before), Some(ready_after)) =
                    (actor_before.ready_at, actor_after.ready_at)
                {
                    // Cost is the difference in ready_at timestamps
                    ready_after.saturating_sub(ready_before)
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        events.push(GameEvent::ActionCompleted {
            actor: actor_id,
            action: delta.action.clone(),
            cost,
        });
    }

    // Check for EntityRemovedFromActive (from Deactivate system action)
    if let game_core::Action::System {
        kind: SystemActionKind::Deactivate(action),
    } = &delta.action
    {
        events.push(GameEvent::EntityRemovedFromActive {
            entity: action.entity,
        });
    }

    // Check for EntityRemovedFromWorld (from RemoveFromWorld system action)
    if let game_core::Action::System {
        kind: SystemActionKind::RemoveFromWorld(action),
    } = &delta.action
    {
        events.push(GameEvent::EntityRemovedFromWorld {
            entity: action.entity,
        });
    }

    // Analyze entity changes
    for actor_change in &delta.entities.actors.updated {
        let Some(actor_before) = state_before.entities.actor(actor_change.id) else {
            continue;
        };
        let Some(actor_after) = state_after.entities.actor(actor_change.id) else {
            continue;
        };

        // Check for HP changes (damage or healing)
        if actor_change.fields.contains(ActorFields::RESOURCES) {
            let old_hp = actor_before.resources.hp;
            let new_hp = actor_after.resources.hp;

            if new_hp != old_hp {
                if new_hp < old_hp {
                    // Damage taken
                    events.push(GameEvent::DamageTaken {
                        entity: actor_change.id,
                        amount: old_hp - new_hp,
                        hp_before: old_hp,
                        hp_after: new_hp,
                        source: Some(delta.action.actor()),
                    });
                }

                // Check for death (HP dropped to 0)
                if old_hp > 0 && new_hp == 0 {
                    tracing::info!(
                        target: "runtime::events",
                        entity = ?actor_change.id,
                        old_hp = old_hp,
                        new_hp = new_hp,
                        "EntityDied event generated"
                    );
                    events.push(GameEvent::EntityDied {
                        entity: actor_change.id,
                        position: actor_after.position,
                        killer: Some(delta.action.actor()),
                    });
                }

                // Check health threshold crossing
                let max_hp = actor_after.snapshot().resource_max.hp_max;
                let old_threshold = HealthThreshold::from_hp(old_hp, max_hp);
                let new_threshold = HealthThreshold::from_hp(new_hp, max_hp);

                if old_threshold != new_threshold {
                    events.push(GameEvent::HealthThresholdCrossed {
                        entity: actor_change.id,
                        threshold: new_threshold,
                        hp_percent: if max_hp > 0 {
                            (new_hp * 100) / max_hp
                        } else {
                            0
                        },
                    });
                }
            }
        }

        // Check for movement
        if actor_change.fields.contains(ActorFields::POSITION) {
            let old_position = actor_before.position;
            let new_position = actor_after.position;

            if old_position != new_position {
                events.push(GameEvent::EntityMoved {
                    entity: actor_change.id,
                    from: old_position,
                    to: new_position,
                });
            }
        }

        // Check for ready_at changes
        if actor_change.fields.contains(ActorFields::READY_AT) {
            let old_ready_at = actor_before.ready_at;
            let new_ready_at = actor_after.ready_at;

            if old_ready_at != new_ready_at {
                events.push(GameEvent::ReadyAtUpdated {
                    entity: actor_change.id,
                    old_ready_at,
                    new_ready_at,
                });
            }
        }
    }

    events
}
