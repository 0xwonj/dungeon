//! Formula evaluation logic.
//!
//! This module implements the evaluation of Formula expressions within an EffectContext.

use crate::action::formula::Formula;
use crate::stats::{CoreEffective, CoreStatKind, ResourceCurrent, ResourceKind};

// We need EffectContext and ActionError from execute module
use crate::action::error::ActionError;
use crate::action::execute::EffectContext;

// ============================================================================
// Formula Evaluation
// ============================================================================

/// Evaluate a formula to get a numeric value.
///
/// ## Supported Formulas
/// - `Constant`: Fixed value
/// - `CasterStat`: Percentage of caster's stat
/// - `TargetStat`: Percentage of target's stat
/// - `WeaponDamage`: Percentage of weapon damage
/// - `FromPreviousDamage`: Percentage of accumulated damage
/// - `FromPreviousHealing`: Percentage of accumulated healing
/// - `TargetResource`: Percentage of target's current resource
/// - `TargetMissingResource`: Percentage of target's missing resource
/// - `TargetMaxResource`: Percentage of target's maximum resource
/// - `Sum`: Sum of sub-formulas
/// - `Product`: Product of sub-formulas (percent-based multiplication)
/// - `Min`: Minimum of sub-formulas
/// - `Max`: Maximum of sub-formulas
///
/// ## Error Handling
/// - `ActorNotFound` if caster doesn't exist
/// - `TargetNotFound` if target doesn't exist
/// - `FormulaEvaluationFailed` for unsupported formulas
pub fn evaluate(formula: &Formula, ctx: &EffectContext) -> Result<u32, ActionError> {
    match formula {
        Formula::Constant(value) => Ok(*value),

        Formula::CasterStat { stat, percent } => {
            let actor = ctx
                .state
                .entities
                .actor(ctx.caster)
                .ok_or(ActionError::ActorNotFound)?;
            let stat_value = get_stat_value(&actor.snapshot().core, stat);
            Ok((stat_value as u32 * percent) / 100)
        }

        Formula::TargetStat { stat, percent } => {
            let actor = ctx
                .state
                .entities
                .actor(ctx.target)
                .ok_or(ActionError::TargetNotFound)?;
            let stat_value = get_stat_value(&actor.snapshot().core, stat);
            Ok((stat_value as u32 * percent) / 100)
        }

        Formula::WeaponDamage { percent } => {
            let actor = ctx
                .state
                .entities
                .actor(ctx.caster)
                .ok_or(ActionError::ActorNotFound)?;

            // Get weapon damage from equipment
            let weapon_damage = if let Some(weapon_handle) = actor.equipment.weapon {
                let items_oracle = ctx.env.items().map_err(|e| {
                    ActionError::FormulaEvaluationFailed(format!("ItemOracle unavailable: {}", e))
                })?;

                let item_def = items_oracle.definition(weapon_handle).ok_or_else(|| {
                    ActionError::FormulaEvaluationFailed(format!(
                        "Item definition not found for handle: {:?}",
                        weapon_handle
                    ))
                })?;

                match item_def.kind {
                    crate::env::ItemKind::Weapon(weapon_data) => weapon_data.damage,
                    _ => {
                        return Err(ActionError::FormulaEvaluationFailed(format!(
                            "Item is not a weapon: {:?}",
                            item_def.kind
                        )));
                    }
                }
            } else {
                5 // Unarmed damage
            };

            Ok((weapon_damage as u32) * percent / 100)
        }

        Formula::FromPreviousDamage { percent } => Ok(ctx.accumulated_damage * percent / 100),

        Formula::FromPreviousHealing { percent } => Ok(ctx.accumulated_healing * percent / 100),

        Formula::CasterResource { resource, percent } => {
            let actor = ctx
                .state
                .entities
                .actor(ctx.caster)
                .ok_or(ActionError::ActorNotFound)?;
            let current = get_resource_current(&actor.resources, resource);
            Ok(current * percent / 100)
        }

        Formula::TargetResource { resource, percent } => {
            let actor = ctx
                .state
                .entities
                .actor(ctx.target)
                .ok_or(ActionError::TargetNotFound)?;
            let current = get_resource_current(&actor.resources, resource);
            Ok(current * percent / 100)
        }

        Formula::TargetMissingResource { resource, percent } => {
            let actor = ctx
                .state
                .entities
                .actor(ctx.target)
                .ok_or(ActionError::TargetNotFound)?;
            let max = actor.snapshot().resource_max.get(*resource);
            let current = get_resource_current(&actor.resources, resource);
            let missing = max.saturating_sub(current);
            Ok(missing * percent / 100)
        }

        Formula::TargetMaxResource { resource, percent } => {
            let actor = ctx
                .state
                .entities
                .actor(ctx.target)
                .ok_or(ActionError::TargetNotFound)?;
            let max = actor.snapshot().resource_max.get(*resource);
            Ok(max * percent / 100)
        }

        Formula::Sum(formulas) => {
            let mut total = 0u32;
            for f in formulas {
                total = total.saturating_add(evaluate(f, ctx)?);
            }
            Ok(total)
        }

        Formula::Product(formulas) => {
            if formulas.is_empty() {
                return Ok(0);
            }
            let mut result = evaluate(&formulas[0], ctx)?;
            for f in &formulas[1..] {
                let value = evaluate(f, ctx)?;
                result = (result * value) / 100; // Percent multiplication
            }
            Ok(result)
        }

        Formula::Min(formulas) => formulas
            .iter()
            .map(|f| evaluate(f, ctx))
            .try_fold(u32::MAX, |min, res| res.map(|v| min.min(v))),

        Formula::Max(formulas) => formulas
            .iter()
            .map(|f| evaluate(f, ctx))
            .try_fold(0u32, |max, res| res.map(|v| max.max(v))),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get stat value from CoreEffective (final computed stats).
fn get_stat_value(stats: &CoreEffective, stat: &CoreStatKind) -> i32 {
    match stat {
        CoreStatKind::Str => stats.str,
        CoreStatKind::Int => stats.int,
        CoreStatKind::Con => stats.con,
        CoreStatKind::Dex => stats.dex,
        CoreStatKind::Wil => stats.wil,
        CoreStatKind::Ego => stats.ego,
    }
}

/// Get current resource value.
fn get_resource_current(resources: &ResourceCurrent, resource: &ResourceKind) -> u32 {
    match resource {
        ResourceKind::Hp => resources.hp,
        ResourceKind::Mp => resources.mp,
        ResourceKind::Lucidity => resources.lucidity,
    }
}
