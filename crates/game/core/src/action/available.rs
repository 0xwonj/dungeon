//! Generate available actions from actor abilities.
//!
//! This module provides the core function `get_available_actions()` which
//! converts an actor's abilities into concrete action instances based on
//! the current game state.

use crate::action::{
    AttackAction, CardinalDirection, CharacterActionKind, InteractAction, InventoryIndex,
    MoveAction, UseItemAction, WaitAction,
};
use crate::env::GameEnv;
use crate::state::{ActionKind, ActorState, EntityId, GameState, Position};

/// Get all available actions for an actor based on their abilities and game state.
///
/// This is the primary function used by AI to determine what actions an entity
/// can currently perform.
///
/// # Arguments
///
/// * `actor` - The actor whose available actions to generate
/// * `state` - Current game state (for finding targets, checking positions, etc.)
/// * `env` - Game environment (for map/oracle access)
///
/// # Returns
///
/// Vector of all concrete actions the actor can perform right now.
pub fn get_available_actions(
    actor: &ActorState,
    state: &GameState,
    env: &GameEnv,
) -> Vec<CharacterActionKind> {
    let current_tick = state.turn.clock;

    actor
        .actions
        .iter()
        .filter(|ability| ability.is_ready(current_tick))
        .flat_map(|ability| generate_from_ability(ability.kind, actor, state, env))
        .collect()
}

/// Generate concrete actions from a single ability.
fn generate_from_ability(
    kind: ActionKind,
    actor: &ActorState,
    state: &GameState,
    env: &GameEnv,
) -> Vec<CharacterActionKind> {
    match kind {
        ActionKind::Move => generate_moves(actor, state, env),
        ActionKind::Wait => vec![CharacterActionKind::Wait(WaitAction::new(actor.id))],
        ActionKind::UseItem => generate_use_items(actor, state),
        ActionKind::Interact => generate_interacts(actor, state),
        ActionKind::MeleeAttack => generate_melee_attacks(actor, state),
        ActionKind::RangedAttack => generate_ranged_attacks(actor, state),
        ActionKind::PowerAttack => generate_power_attacks(actor, state),
        ActionKind::Backstab => generate_backstabs(actor, state),
        ActionKind::Cleave => generate_cleaves(actor, state),
        ActionKind::AimedShot => generate_aimed_shots(actor, state),
        ActionKind::Fireball => generate_fireballs(actor, state),
        ActionKind::Lightning => generate_lightning(actor, state),
        ActionKind::Heal => generate_heals(actor, state),
        ActionKind::Shield => generate_shields(actor, state),
        ActionKind::Teleport => generate_teleports(actor, state, env),
        ActionKind::Dash => generate_dashes(actor, state, env),
        ActionKind::Stealth => generate_stealth(actor),
        ActionKind::SneakAttack => generate_sneak_attacks(actor, state),
        ActionKind::CallAllies => generate_call_allies(actor),
        ActionKind::Intimidate => generate_intimidates(actor, state),
        ActionKind::Rally => generate_rallies(actor),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Step from position in a direction.
fn step(pos: Position, dir: CardinalDirection) -> Position {
    let (dx, dy) = dir.delta();
    Position::new(pos.x + dx, pos.y + dy)
}

// ============================================================================
// Basic Actions
// ============================================================================

fn generate_moves(
    actor: &ActorState,
    state: &GameState,
    env: &GameEnv,
) -> Vec<CharacterActionKind> {
    let map = match env.map() {
        Some(m) => m,
        None => return vec![],
    };

    CardinalDirection::ALL
        .iter()
        .filter_map(|&dir| {
            let new_pos = step(actor.position, dir);
            if state.can_enter(map, new_pos) {
                Some(CharacterActionKind::Move(MoveAction::new(actor.id, dir)))
            } else {
                None
            }
        })
        .collect()
}

fn generate_use_items(actor: &ActorState, _state: &GameState) -> Vec<CharacterActionKind> {
    // Generate UseItem actions for each inventory slot
    actor
        .inventory
        .items
        .iter()
        .enumerate()
        .map(|(idx, _slot)| {
            CharacterActionKind::UseItem(UseItemAction::new(
                actor.id,
                InventoryIndex::new(idx as u8),
                None, // TODO: determine target based on item type
            ))
        })
        .collect()
}

fn generate_interacts(actor: &ActorState, state: &GameState) -> Vec<CharacterActionKind> {
    // Find adjacent props that can be interacted with
    let mut actions = Vec::new();

    for dir in CardinalDirection::ALL.iter() {
        let pos = step(actor.position, *dir);
        // Check if there's a prop at this position
        if let Some(prop) = state.entities.props.iter().find(|p| p.position == pos) {
            actions.push(CharacterActionKind::Interact(InteractAction::new(
                actor.id, prop.id,
            )));
        }
    }

    actions
}

// ============================================================================
// Combat - Melee
// ============================================================================

fn generate_melee_attacks(actor: &ActorState, state: &GameState) -> Vec<CharacterActionKind> {
    find_adjacent_enemies(actor, state)
        .map(|target| CharacterActionKind::Attack(AttackAction::new(actor.id, target)))
        .collect()
}

fn generate_power_attacks(actor: &ActorState, state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement PowerAttackAction
    find_adjacent_enemies(actor, state)
        .map(|target| CharacterActionKind::Attack(AttackAction::new(actor.id, target)))
        .collect()
}

fn generate_backstabs(actor: &ActorState, state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement BackstabAction
    // TODO: Check if enemy is facing away
    find_adjacent_enemies(actor, state)
        .map(|target| CharacterActionKind::Attack(AttackAction::new(actor.id, target)))
        .collect()
}

fn generate_cleaves(_actor: &ActorState, _state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement CleaveAction (hits all adjacent enemies at once)
    // For now, just return empty since basic Attack only hits one target
    vec![]
}

fn generate_sneak_attacks(actor: &ActorState, state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement SneakAttackAction
    // TODO: Check if actor is stealthed/invisible
    find_adjacent_enemies(actor, state)
        .map(|target| CharacterActionKind::Attack(AttackAction::new(actor.id, target)))
        .collect()
}

// ============================================================================
// Combat - Ranged
// ============================================================================

fn generate_ranged_attacks(actor: &ActorState, state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Check line of sight and range
    find_visible_enemies(actor, state)
        .map(|target| CharacterActionKind::Attack(AttackAction::new(actor.id, target)))
        .collect()
}

fn generate_aimed_shots(actor: &ActorState, state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement AimedShotAction
    find_visible_enemies(actor, state)
        .map(|target| CharacterActionKind::Attack(AttackAction::new(actor.id, target)))
        .collect()
}

// ============================================================================
// Magic & Skills (Placeholder implementations)
// ============================================================================

fn generate_fireballs(_actor: &ActorState, _state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement when Fireball action exists
    vec![]
}

fn generate_lightning(_actor: &ActorState, _state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement when Lightning action exists
    vec![]
}

fn generate_heals(_actor: &ActorState, _state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement when Heal action exists
    vec![]
}

fn generate_shields(_actor: &ActorState, _state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement when Shield action exists
    vec![]
}

fn generate_teleports(
    _actor: &ActorState,
    _state: &GameState,
    _env: &GameEnv,
) -> Vec<CharacterActionKind> {
    // TODO: Implement when Teleport action exists
    vec![]
}

fn generate_dashes(
    _actor: &ActorState,
    _state: &GameState,
    _env: &GameEnv,
) -> Vec<CharacterActionKind> {
    // TODO: Implement when Dash action exists
    vec![]
}

fn generate_stealth(_actor: &ActorState) -> Vec<CharacterActionKind> {
    // TODO: Implement when Stealth action exists
    vec![]
}

fn generate_call_allies(_actor: &ActorState) -> Vec<CharacterActionKind> {
    // TODO: Implement when CallAllies action exists
    vec![]
}

fn generate_intimidates(_actor: &ActorState, _state: &GameState) -> Vec<CharacterActionKind> {
    // TODO: Implement when Intimidate action exists
    vec![]
}

fn generate_rallies(_actor: &ActorState) -> Vec<CharacterActionKind> {
    // TODO: Implement when Rally action exists
    vec![]
}

/// Find all enemy entities adjacent to the actor.
fn find_adjacent_enemies<'a>(
    actor: &ActorState,
    state: &'a GameState,
) -> impl Iterator<Item = EntityId> + 'a {
    let actor_pos = actor.position;
    let actor_id = actor.id;

    CardinalDirection::ALL
        .iter()
        .map(move |&dir| step(actor_pos, dir))
        .filter_map(move |pos| {
            // Check all actors
            state
                .entities
                .all_actors()
                .find(|actor| actor.position == pos && actor.id != actor_id)
                .map(|actor| actor.id)
        })
}

/// Find all enemy entities visible to the actor.
fn find_visible_enemies<'a>(
    actor: &ActorState,
    state: &'a GameState,
) -> impl Iterator<Item = EntityId> + 'a {
    let actor_id = actor.id;

    // TODO: Implement proper line-of-sight checking
    // For now, just return all other actors
    state
        .entities
        .all_actors()
        .filter(move |a| a.id != actor_id && a.is_alive())
        .map(|a| a.id)
}
