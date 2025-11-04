//! Effect application - all effect implementations in one place.
//!
//! This module contains:
//! - `EffectContext`: Shared state for effect execution
//! - `apply_effect`: Effect dispatcher (matches on effect kind)
//! - Effect implementations: Damage, healing, movement, resources, status
//!
//! ## Effect Categories
//!
//! - **Damage**: Apply damage with type and crit support
//! - **Resources**: Restore, drain, consume resources (HP/MP/Lucidity)
//! - **Movement**: MoveSelf, MoveTarget, Swap positions
//! - **Status**: Apply/remove status effects (TODO)
//! - **Conditional**: Apply effects based on conditions (TODO)
//!
//! ## Implementation Pattern
//!
//! Each effect type has dedicated helper functions:
//! - `apply_damage` - Damage calculation and application
//! - `apply_movement` - Movement with collision and terrain checking
//! - `restore_resource` - Resource restoration with caps
//! - `drain_resource` - Resource draining
//!
//! ## Future Extensions
//!
//! - Critical hit calculation
//! - Resistance/armor application
//! - Status effect stacking
//! - Conditional effect evaluation
//! - Projectile/area effects

use crate::action::Displacement;
use crate::action::effect::{ActionEffect, EffectKind};
use crate::action::types::ActionInput;
use crate::combat::DamageType;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Position};
use crate::stats::ResourceKind;

use super::formula::evaluate_formula;
use super::validation::ActionError;

// ============================================================================
// Effect Context
// ============================================================================

/// Context for effect execution.
///
/// This provides all data needed for effects and accumulates results
/// across multiple effects in a single action.
///
/// ## Lifetime
/// Created once per target, reused for all effects targeting that entity.
///
/// ## Action Input
/// Effects can access the user/AI input via `action_input`:
/// - Direction for movement effects
/// - Position for teleport/area effects
/// - Entity for targeted effects
///
/// ## Accumulated Data
/// Effects can read accumulated data from previous effects:
/// - `accumulated_damage`: For damage chain formulas
/// - `accumulated_healing`: For healing chain formulas
/// - `was_critical`: For on-crit conditional effects
pub struct EffectContext<'a> {
    /// The entity performing the action.
    pub caster: EntityId,

    /// The current target entity.
    pub target: EntityId,

    /// Mutable game state.
    pub state: &'a mut GameState,

    /// Environment oracles.
    pub env: &'a GameEnv<'a>,

    /// Accumulated result (updated by each effect).
    pub result: &'a mut crate::action::types::ActionResult,

    /// User/AI input for this action (e.g., direction, position).
    /// Effects can read this to get direction for movement, etc.
    pub action_input: &'a ActionInput,

    // ========================================================================
    // Accumulated Data (for formulas like FromPreviousDamage)
    // ========================================================================
    /// Total damage dealt in this action so far.
    pub accumulated_damage: u32,

    /// Total healing done in this action so far.
    pub accumulated_healing: u32,

    /// Whether any effect was a critical hit.
    pub was_critical: bool,
}

impl<'a> EffectContext<'a> {
    /// Creates a new effect context.
    pub fn new(
        caster: EntityId,
        target: EntityId,
        state: &'a mut GameState,
        env: &'a GameEnv<'a>,
        result: &'a mut crate::action::types::ActionResult,
        action_input: &'a ActionInput,
    ) -> Self {
        Self {
            caster,
            target,
            state,
            env,
            result,
            action_input,
            accumulated_damage: 0,
            accumulated_healing: 0,
            was_critical: false,
        }
    }
}

// ============================================================================
// Effect Dispatcher
// ============================================================================

/// Apply a single effect to current context.
///
/// This is the main dispatcher that routes to specific effect implementations.
///
/// ## Implemented Effects
/// - `Damage`: Deal damage with type/crit
/// - `RestoreResource`: Heal HP/MP/Lucidity
/// - `DrainResource`: Drain resources (with optional transfer)
/// - `MoveSelf`: Move caster
/// - `MoveTarget`: Move target
///
/// ## Not Yet Implemented
/// - `ApplyStatus`: Status effects
/// - `RemoveStatus`: Status removal
/// - `Swap`: Position swap
/// - `ConsumeResource`: Resource cost
/// - `Conditional`: Conditional effects
/// - `Projectile`: Projectile creation
/// - `AreaEffect`: AoE targeting
pub(super) fn apply_effect(
    effect: &ActionEffect,
    ctx: &mut EffectContext,
) -> Result<(), ActionError> {
    match &effect.kind {
        EffectKind::Damage {
            formula,
            damage_type,
            can_crit: _,
        } => {
            let amount = evaluate_formula(formula, ctx)?;
            let actual = apply_damage(ctx, amount, *damage_type)?;
            ctx.accumulated_damage += actual;
            ctx.result.add_damage(actual);
        }

        EffectKind::RestoreResource {
            resource,
            formula,
            overfill_allowed: _,
        } => {
            let amount = evaluate_formula(formula, ctx)?;
            let actual = restore_resource(ctx, *resource, amount)?;
            if *resource == ResourceKind::Hp {
                ctx.accumulated_healing += actual;
                ctx.result.add_healed(actual);
            }
        }

        EffectKind::DrainResource {
            resource,
            formula,
            transfer_to_caster,
        } => {
            let amount = evaluate_formula(formula, ctx)?;
            let drained = drain_resource(ctx, *resource, amount)?;
            if *transfer_to_caster {
                restore_resource_to(ctx, ctx.caster, *resource, drained)?;
            }
        }

        EffectKind::ApplyStatus {
            status: _,
            duration: _,
        } => {
            // TODO: Implement status effects
            return Err(ActionError::NotImplemented(
                "Status effects not yet implemented".to_string(),
            ));
        }

        EffectKind::MoveSelf { displacement } => {
            apply_movement(ctx, ctx.caster, displacement)?;
        }

        EffectKind::MoveTarget { displacement } => {
            apply_movement(ctx, ctx.target, displacement)?;
        }

        EffectKind::Swap => {
            // TODO: Implement position swap (not needed for minimal implementation)
            return Err(ActionError::NotImplemented(
                "Swap effect not yet implemented".to_string(),
            ));
        }

        _ => {
            return Err(ActionError::NotImplemented(format!(
                "Effect {:?} not yet implemented",
                effect.kind
            )));
        }
    }

    Ok(())
}

// ============================================================================
// Damage Effects
// ============================================================================

/// Apply damage to target in context.
///
/// ## Implementation
/// 1. Get target actor (mutable)
/// 2. Calculate actual damage (capped at current HP)
/// 3. Subtract from HP using saturating arithmetic
/// 4. Add target to affected list
/// 5. Return actual damage dealt
///
/// ## Future Extensions
/// - Critical hit calculation (based on DEX, luck, etc.)
/// - Resistance/armor calculation (based on damage type)
/// - Damage type effectiveness (fire vs cold, etc.)
/// - Death state handling (currently just sets HP to 0)
fn apply_damage(
    ctx: &mut EffectContext,
    damage: u32,
    _damage_type: DamageType,
) -> Result<u32, ActionError> {
    let actor = ctx
        .state
        .entities
        .actor_mut(ctx.target)
        .ok_or(ActionError::TargetNotFound)?;

    // TODO: Apply resistance/armor based on damage_type
    // TODO: Check for critical hit based on can_crit flag

    let actual_damage = damage.min(actor.resources.hp);
    actor.resources.hp = actor.resources.hp.saturating_sub(actual_damage);

    // Add to affected targets if not already present
    if !ctx.result.affected_targets.contains(&ctx.target) {
        ctx.result.affected_targets.push(ctx.target);
    }

    Ok(actual_damage)
}

// ============================================================================
// Resource Effects
// ============================================================================

/// Restore resource to target in context.
///
/// Wrapper around `restore_resource_to` targeting `ctx.target`.
fn restore_resource(
    ctx: &mut EffectContext,
    resource: ResourceKind,
    amount: u32,
) -> Result<u32, ActionError> {
    restore_resource_to(ctx, ctx.target, resource, amount)
}

/// Restore resource to specific entity.
///
/// ## Implementation
/// 1. Get entity's current and max resource values
/// 2. Calculate how much can be restored (capped at max)
/// 3. Add to resource
/// 4. Add entity to affected list
/// 5. Return actual amount restored
///
/// ## Future Extensions
/// - Overfill support (healing above max)
/// - Healing effectiveness modifiers
fn restore_resource_to(
    ctx: &mut EffectContext,
    entity: EntityId,
    resource: ResourceKind,
    amount: u32,
) -> Result<u32, ActionError> {
    let actor = ctx
        .state
        .entities
        .actor_mut(entity)
        .ok_or(ActionError::TargetNotFound)?;

    let max = actor.snapshot().resource_max.get(resource);
    let current = match resource {
        ResourceKind::Hp => actor.resources.hp,
        ResourceKind::Mp => actor.resources.mp,
        ResourceKind::Lucidity => actor.resources.lucidity,
    };

    let missing = max.saturating_sub(current);
    let actual = amount.min(missing);

    // Apply restoration
    match resource {
        ResourceKind::Hp => actor.resources.hp += actual,
        ResourceKind::Mp => actor.resources.mp += actual,
        ResourceKind::Lucidity => actor.resources.lucidity += actual,
    }

    // Add to affected targets if not already present
    if !ctx.result.affected_targets.contains(&entity) {
        ctx.result.affected_targets.push(entity);
    }

    Ok(actual)
}

/// Drain resource from target.
///
/// ## Implementation
/// 1. Get target's current resource value
/// 2. Calculate actual drain (capped at current)
/// 3. Subtract from resource
/// 4. Add target to affected list
/// 5. Return actual amount drained
fn drain_resource(
    ctx: &mut EffectContext,
    resource: ResourceKind,
    amount: u32,
) -> Result<u32, ActionError> {
    let actor = ctx
        .state
        .entities
        .actor_mut(ctx.target)
        .ok_or(ActionError::TargetNotFound)?;

    let current = match resource {
        ResourceKind::Hp => actor.resources.hp,
        ResourceKind::Mp => actor.resources.mp,
        ResourceKind::Lucidity => actor.resources.lucidity,
    };

    let actual = amount.min(current);

    // Apply drain
    match resource {
        ResourceKind::Hp => actor.resources.hp -= actual,
        ResourceKind::Mp => actor.resources.mp -= actual,
        ResourceKind::Lucidity => actor.resources.lucidity -= actual,
    }

    // Add to affected targets if not already present
    if !ctx.result.affected_targets.contains(&ctx.target) {
        ctx.result.affected_targets.push(ctx.target);
    }

    Ok(actual)
}

// ============================================================================
// Movement Effects
// ============================================================================

/// Apply movement displacement to an entity.
///
/// ## Implementation
/// 1. Calculate destination based on displacement type
/// 2. Validate destination (bounds, terrain, occupancy)
/// 3. Update entity position
/// 4. Add to affected targets
///
/// ## Displacement Types
/// - `Direction`: Move in cardinal direction by distance
/// - `TowardTarget`: Move toward target by distance
/// - `AwayFromTarget`: Move away from target (knockback)
/// - `AwayFromCaster`: Move away from caster
/// - `ToPosition`: Teleport to exact position (TODO)
/// - `RandomInRange`: Random teleport (TODO)
///
/// ## Validation
/// - Bounds: Destination must be within map dimensions
/// - Terrain: Destination tile must be passable
/// - Occupancy: No other entity at destination
///
/// ## Future Extensions
/// - Line of sight checking
/// - Forced movement (push through entities)
/// - Terrain interaction (fall into pit, trigger trap)
fn apply_movement(
    ctx: &mut EffectContext,
    entity_id: EntityId,
    displacement: &Displacement,
) -> Result<(), ActionError> {
    // Get entity's current position
    let actor = ctx
        .state
        .entities
        .actor(entity_id)
        .ok_or(ActionError::ActorNotFound)?;
    let current_pos = actor.position;

    // Calculate destination based on displacement type
    let destination = match displacement {
        Displacement::FromInput { distance } => {
            // Read direction from action input
            let direction = match ctx.action_input {
                ActionInput::Direction(dir) => dir,
                _ => {
                    return Err(ActionError::EffectFailed(
                        "FromInput displacement requires Direction input".to_string(),
                    ));
                }
            };

            // Move in specified direction
            let (dx, dy) = direction.offset();
            let new_x = current_pos.x + dx * (*distance as i32);
            let new_y = current_pos.y + dy * (*distance as i32);
            Position::new(new_x, new_y)
        }

        Displacement::TowardTarget { distance } => {
            // Move toward target
            let target = ctx
                .state
                .entities
                .actor(ctx.target)
                .ok_or(ActionError::TargetNotFound)?;
            let target_pos = target.position;

            calculate_destination_toward(current_pos, target_pos, *distance)
        }

        Displacement::AwayFromTarget { distance } => {
            // Move away from target
            let target = ctx
                .state
                .entities
                .actor(ctx.target)
                .ok_or(ActionError::TargetNotFound)?;
            let target_pos = target.position;

            calculate_destination_away(current_pos, target_pos, *distance)
        }

        Displacement::AwayFromCaster { distance } => {
            // Move away from caster
            let caster = ctx
                .state
                .entities
                .actor(ctx.caster)
                .ok_or(ActionError::ActorNotFound)?;
            let caster_pos = caster.position;

            calculate_destination_away(current_pos, caster_pos, *distance)
        }

        Displacement::ToInputPosition => {
            // Read position from action input
            match ctx.action_input {
                ActionInput::Position(pos) => *pos,
                _ => {
                    return Err(ActionError::EffectFailed(
                        "ToInputPosition displacement requires Position input".to_string(),
                    ));
                }
            }
        }

        Displacement::RandomInRange { range: _ } => {
            // Random teleport (not implemented yet)
            return Err(ActionError::NotImplemented(
                "RandomInRange displacement not yet implemented".to_string(),
            ));
        }
    };

    // Validate destination
    validate_destination(ctx, entity_id, destination)?;

    // Apply movement
    let actor_mut = ctx
        .state
        .entities
        .actor_mut(entity_id)
        .ok_or(ActionError::ActorNotFound)?;
    actor_mut.position = destination;

    // Add to affected targets
    if !ctx.result.affected_targets.contains(&entity_id) {
        ctx.result.affected_targets.push(entity_id);
    }

    Ok(())
}

/// Validate destination is legal for movement.
///
/// ## Checks
/// 1. Bounds: Within map dimensions
/// 2. Terrain: Tile is passable
/// 3. Occupancy: No entity at position (except mover)
fn validate_destination(
    ctx: &EffectContext,
    mover: EntityId,
    destination: Position,
) -> Result<(), ActionError> {
    let map = ctx.env.map().map_err(|_| ActionError::MapNotAvailable)?;

    // Check bounds
    if !map.dimensions().contains(destination) {
        return Err(ActionError::OutOfBounds);
    }

    // Check if tile is passable
    let tile = map.tile(destination).ok_or(ActionError::InvalidPosition)?;
    if !tile.is_passable() {
        return Err(ActionError::Blocked);
    }

    // Check for entity collision
    for actor in ctx.state.entities.all_actors() {
        if actor.id != mover && actor.position == destination {
            return Err(ActionError::Occupied);
        }
    }

    Ok(())
}

/// Calculate destination moving toward a target.
///
/// Uses normalized direction vector to move up to `distance` steps.
fn calculate_destination_toward(from: Position, toward: Position, distance: u32) -> Position {
    let dx = toward.x - from.x;
    let dy = toward.y - from.y;

    // Normalize direction
    let steps = dx.abs().max(dy.abs());
    if steps == 0 {
        return from; // Already at target
    }

    let step_x = if dx != 0 { dx / dx.abs() } else { 0 };
    let step_y = if dy != 0 { dy / dy.abs() } else { 0 };

    // Move up to 'distance' steps toward target
    let actual_steps = (distance as i32).min(steps);
    let new_x = from.x + step_x * actual_steps;
    let new_y = from.y + step_y * actual_steps;

    Position::new(new_x, new_y)
}

/// Calculate destination moving away from a position.
///
/// Uses inverted direction vector to move away by `distance` steps.
fn calculate_destination_away(from: Position, away_from: Position, distance: u32) -> Position {
    let dx = from.x - away_from.x;
    let dy = from.y - away_from.y;

    // If already at same position, move in random direction (for now, just north)
    let (step_x, step_y) = if dx == 0 && dy == 0 {
        (0, -1) // North
    } else {
        // Normalize direction (away)
        let step_x = if dx != 0 { dx / dx.abs() } else { 0 };
        let step_y = if dy != 0 { dy / dy.abs() } else { 0 };
        (step_x, step_y)
    };

    let new_x = from.x + step_x * distance as i32;
    let new_y = from.y + step_y * distance as i32;

    Position::new(new_x, new_y)
}
