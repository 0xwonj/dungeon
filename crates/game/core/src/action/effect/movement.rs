//! Movement effect implementations.

use crate::action::effect::{Displacement, ExecutionPhase};
use crate::action::error::ActionError;
use crate::action::execute::EffectContext;
use crate::action::types::{ActionInput, AppliedValue};
use crate::state::Position;

/// Move the caster.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoveSelfEffect {
    pub displacement: Displacement,
}

impl MoveSelfEffect {
    /// Create a new MoveSelf effect.
    pub fn new(displacement: Displacement) -> Self {
        Self { displacement }
    }

    /// Pre-validate: Check if destination is valid.
    ///
    /// This validates BEFORE any state changes:
    /// - Destination is within map bounds
    /// - Destination tile is passable
    /// - Destination is not occupied
    pub fn pre_validate(&self, ctx: &EffectContext) -> Result<(), ActionError> {
        let destination = self.calculate_destination(ctx, ctx.caster)?;
        validate_destination(ctx, ctx.caster, destination)
    }

    /// Apply movement to caster.
    pub fn apply(&self, ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        let from = ctx
            .state
            .actor_position(ctx.caster)
            .ok_or(ActionError::ActorNotFound)?;

        // Calculate and validate destination
        let to = self.calculate_destination(ctx, ctx.caster)?;

        // Validate again (defensive, in case state changed)
        validate_destination(ctx, ctx.caster, to)?;

        // Update occupancy: remove from old position
        ctx.state.world.tile_map.remove_occupant(&from, ctx.caster);

        // Update occupancy: add to new position
        ctx.state.world.tile_map.add_occupant(to, ctx.caster);

        // Apply movement to actor
        ctx.state
            .entities
            .actor_mut(ctx.caster)
            .ok_or(ActionError::ActorNotFound)?
            .position = Some(to);

        Ok(AppliedValue::Movement { from, to })
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Calculate destination based on displacement type.
    fn calculate_destination(
        &self,
        ctx: &EffectContext,
        entity_id: crate::state::EntityId,
    ) -> Result<Position, ActionError> {
        let current_pos = ctx
            .state
            .actor_position(entity_id)
            .ok_or(ActionError::ActorNotFound)?;

        match &self.displacement {
            Displacement::FromInput { distance } => {
                let direction = match ctx.action_input {
                    ActionInput::Direction(dir) => dir,
                    _ => {
                        return Err(ActionError::EffectFailed(
                            "FromInput displacement requires Direction input".to_string(),
                        ));
                    }
                };

                let (dx, dy) = direction.offset();
                let new_x = current_pos.x + dx * (*distance as i32);
                let new_y = current_pos.y + dy * (*distance as i32);
                Ok(Position::new(new_x, new_y))
            }

            Displacement::TowardTarget { distance } => {
                let target_pos = ctx
                    .state
                    .actor_position(ctx.target)
                    .ok_or(ActionError::TargetNotFound)?;

                Ok(calculate_destination_toward(
                    current_pos,
                    target_pos,
                    *distance,
                ))
            }

            Displacement::AwayFromTarget { distance } => {
                let target_pos = ctx
                    .state
                    .actor_position(ctx.target)
                    .ok_or(ActionError::TargetNotFound)?;

                Ok(calculate_destination_away(
                    current_pos,
                    target_pos,
                    *distance,
                ))
            }

            Displacement::AwayFromCaster { distance } => {
                let caster_pos = ctx
                    .state
                    .actor_position(ctx.caster)
                    .ok_or(ActionError::ActorNotFound)?;

                Ok(calculate_destination_away(
                    current_pos,
                    caster_pos,
                    *distance,
                ))
            }

            Displacement::ToInputPosition => match ctx.action_input {
                ActionInput::Position(pos) => Ok(*pos),
                _ => Err(ActionError::EffectFailed(
                    "ToInputPosition displacement requires Position input".to_string(),
                )),
            },

            Displacement::RandomInRange { range: _ } => Err(ActionError::NotImplemented(
                "RandomInRange displacement not yet implemented".to_string(),
            )),
        }
    }

    /// Get default execution phase for MoveSelf effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::PreEffect
    }
}

/// Move the target.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoveTargetEffect {
    pub displacement: Displacement,
}

impl MoveTargetEffect {
    /// Create a new MoveTarget effect.
    pub fn new(displacement: Displacement) -> Self {
        Self { displacement }
    }

    /// Pre-validate: Check if destination is valid.
    pub fn pre_validate(&self, ctx: &EffectContext) -> Result<(), ActionError> {
        let destination = self.calculate_destination(ctx, ctx.target)?;
        validate_destination(ctx, ctx.target, destination)
    }

    /// Apply movement to target.
    pub fn apply(&self, ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        let from = ctx
            .state
            .actor_position(ctx.target)
            .ok_or(ActionError::TargetNotFound)?;

        let to = self.calculate_destination(ctx, ctx.target)?;

        // Validate again (defensive)
        validate_destination(ctx, ctx.target, to)?;

        // Update occupancy: remove from old position
        ctx.state.world.tile_map.remove_occupant(&from, ctx.target);

        // Update occupancy: add to new position
        ctx.state.world.tile_map.add_occupant(to, ctx.target);

        // Apply movement to target
        ctx.state
            .entities
            .actor_mut(ctx.target)
            .ok_or(ActionError::TargetNotFound)?
            .position = Some(to);

        Ok(AppliedValue::Movement { from, to })
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Calculate destination (same logic as MoveSelf).
    fn calculate_destination(
        &self,
        ctx: &EffectContext,
        entity_id: crate::state::EntityId,
    ) -> Result<Position, ActionError> {
        // Same logic as MoveSelf - could be refactored into shared function
        let current_pos = ctx
            .state
            .actor_position(entity_id)
            .ok_or(ActionError::ActorNotFound)?;

        match &self.displacement {
            Displacement::FromInput { distance } => {
                let direction = match ctx.action_input {
                    ActionInput::Direction(dir) => dir,
                    _ => {
                        return Err(ActionError::EffectFailed(
                            "FromInput displacement requires Direction input".to_string(),
                        ));
                    }
                };

                let (dx, dy) = direction.offset();
                let new_x = current_pos.x + dx * (*distance as i32);
                let new_y = current_pos.y + dy * (*distance as i32);
                Ok(Position::new(new_x, new_y))
            }

            Displacement::TowardTarget { distance } => {
                let target_pos = ctx
                    .state
                    .actor_position(ctx.target)
                    .ok_or(ActionError::TargetNotFound)?;

                Ok(calculate_destination_toward(
                    current_pos,
                    target_pos,
                    *distance,
                ))
            }

            Displacement::AwayFromTarget { distance } => {
                let target_pos = ctx
                    .state
                    .actor_position(ctx.target)
                    .ok_or(ActionError::TargetNotFound)?;

                Ok(calculate_destination_away(
                    current_pos,
                    target_pos,
                    *distance,
                ))
            }

            Displacement::AwayFromCaster { distance } => {
                let caster_pos = ctx
                    .state
                    .actor_position(ctx.caster)
                    .ok_or(ActionError::ActorNotFound)?;

                Ok(calculate_destination_away(
                    current_pos,
                    caster_pos,
                    *distance,
                ))
            }

            Displacement::ToInputPosition => match ctx.action_input {
                ActionInput::Position(pos) => Ok(*pos),
                _ => Err(ActionError::EffectFailed(
                    "ToInputPosition displacement requires Position input".to_string(),
                )),
            },

            Displacement::RandomInRange { range: _ } => Err(ActionError::NotImplemented(
                "RandomInRange displacement not yet implemented".to_string(),
            )),
        }
    }

    /// Get default execution phase for MoveTarget effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::PostEffect
    }
}

/// Swap positions with target.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapEffect;

impl SwapEffect {
    /// Create a new Swap effect.
    pub fn new() -> Self {
        Self
    }

    /// Pre-validate: Check both entities exist.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        // Basic validation done at action-level
        Ok(())
    }

    /// Apply position swap.
    pub fn apply(&self, _ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // TODO: Implement swap
        Err(ActionError::NotImplemented(
            "Swap effect not yet implemented".to_string(),
        ))
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for Swap effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::Primary
    }
}

impl Default for SwapEffect {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate destination is legal for movement.
fn validate_destination(
    ctx: &EffectContext,
    mover: crate::state::EntityId,
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
        if actor.id != mover && actor.position == Some(destination) {
            return Err(ActionError::Occupied);
        }
    }

    Ok(())
}

/// Calculate destination moving toward a target.
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
fn calculate_destination_away(from: Position, away_from: Position, distance: u32) -> Position {
    let dx = from.x - away_from.x;
    let dy = from.y - away_from.y;

    // If already at same position, move in default direction (north)
    let (step_x, step_y) = if dx == 0 && dy == 0 {
        (0, -1)
    } else {
        let step_x = if dx != 0 { dx / dx.abs() } else { 0 };
        let step_y = if dy != 0 { dy / dy.abs() } else { 0 };
        (step_x, step_y)
    };

    let new_x = from.x + step_x * distance as i32;
    let new_y = from.y + step_y * distance as i32;

    Position::new(new_x, new_y)
}
