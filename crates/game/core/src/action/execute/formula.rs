//! Formula evaluation system for dynamic value calculation.
//!
//! Formulas allow action effects to scale based on:
//! - Character stats (STR, INT, etc.)
//! - Current/max resources (HP, MP, Lucidity)
//! - Previous effects in the same action (damage chains)
//! - Weapon damage
//! - Arithmetic combinations (sum, product, min, max)
//!
//! ## Design Principles
//!
//! - **Composable**: Formulas can be nested arbitrarily
//! - **Pure**: No side effects, deterministic evaluation
//! - **Safe**: Uses saturating arithmetic to prevent overflow
//! - **Extensible**: Easy to add new formula types
//!
//! ## Examples
//!
//! ```ignore
//! // 150% weapon damage
//! Formula::WeaponDamage { percent: 150 }
//!
//! // 50% caster STR + 10 flat
//! Formula::Sum(vec![
//!     Formula::CasterStat { stat: CoreStatKind::Str, percent: 50 },
//!     Formula::Constant(10),
//! ])
//!
//! // 30% of previous damage (damage chain)
//! Formula::FromPreviousDamage { percent: 30 }
//! ```

use crate::action::effect::Formula;
use crate::stats::{CoreEffective, CoreStatKind, ResourceCurrent, ResourceKind};

use super::effects::EffectContext;
use super::validation::ActionError;

// ============================================================================
// Formula Evaluation
// ============================================================================

/// Evaluate a formula to get a numeric value.
///
/// ## Supported Formulas
/// - `Constant`: Fixed value
/// - `CasterStat`: Percentage of caster's stat
/// - `TargetStat`: Percentage of target's stat
/// - `WeaponDamage`: Percentage of weapon damage (TODO: implement actual weapon lookup)
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
pub(super) fn evaluate_formula(formula: &Formula, ctx: &EffectContext) -> Result<u32, ActionError> {
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
                // Get weapon definition from ItemOracle
                match ctx.env.items() {
                    Ok(items_oracle) => {
                        if let Some(item_def) = items_oracle.definition(weapon_handle) {
                            // Extract damage from WeaponData
                            if let crate::env::ItemKind::Weapon(weapon_data) = item_def.kind {
                                weapon_data.damage
                            } else {
                                5 // Not a weapon (shouldn't happen)
                            }
                        } else {
                            5 // Item definition not found
                        }
                    }
                    Err(_) => 5, // Oracle unavailable
                }
            } else {
                5 // Unarmed damage
            };

            Ok((weapon_damage as u32) * percent / 100)
        }

        Formula::FromPreviousDamage { percent } => Ok(ctx.accumulated_damage * percent / 100),

        Formula::FromPreviousHealing { percent } => Ok(ctx.accumulated_healing * percent / 100),

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
                total = total.saturating_add(evaluate_formula(f, ctx)?);
            }
            Ok(total)
        }

        Formula::Product(formulas) => {
            if formulas.is_empty() {
                return Ok(0);
            }
            let mut result = evaluate_formula(&formulas[0], ctx)?;
            for f in &formulas[1..] {
                let value = evaluate_formula(f, ctx)?;
                result = (result * value) / 100; // Percent multiplication
            }
            Ok(result)
        }

        Formula::Min(formulas) => formulas
            .iter()
            .map(|f| evaluate_formula(f, ctx))
            .try_fold(u32::MAX, |min, res| res.map(|v| min.min(v))),

        Formula::Max(formulas) => formulas
            .iter()
            .map(|f| evaluate_formula(f, ctx))
            .try_fold(0u32, |max, res| res.map(|v| max.max(v))),

        _ => Err(ActionError::FormulaEvaluationFailed(format!(
            "Formula {:?} not yet implemented",
            formula
        ))),
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
