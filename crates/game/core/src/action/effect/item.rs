//! Item-related effect implementations.

use crate::action::effect::ExecutionPhase;
use crate::action::error::ActionError;
use crate::action::execute::EffectContext;
use crate::action::types::{ActionInput, AppliedValue};

/// Acquire an item entity from the world and add it to the caster's inventory.
///
/// This effect:
/// 1. Gets the item EntityId from ActionInput::Target
/// 2. Adds it to the caster's inventory
/// 3. Removes the item entity from the world
///
/// This is a general-purpose effect for any scenario where an existing world item
/// should be transferred to inventory (picking up drops, looting containers, etc.).
///
/// **Important**: This effect does NOT validate position or range. Those checks
/// should be performed at the action validation level if needed.
///
/// For creating new items directly in inventory (quest rewards, generation, etc.),
/// use a separate effect (to be implemented).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AcquireItemEffect;

impl AcquireItemEffect {
    /// Create a new AcquireItem effect.
    pub fn new() -> Self {
        Self
    }

    /// Pre-validate: Check if item exists and caster has inventory space.
    pub fn pre_validate(&self, ctx: &EffectContext) -> Result<(), ActionError> {
        // Get target item ID from ActionInput
        let item_id = match ctx.action_input {
            ActionInput::Target(id) => *id,
            _ => {
                return Err(ActionError::EffectFailed(
                    "AcquireItemEffect requires Target input".to_string(),
                ));
            }
        };

        // Check item exists in world
        let _item = ctx
            .state
            .entities
            .item(item_id)
            .ok_or_else(|| ActionError::EffectFailed(format!("Item {} not found", item_id)))?;

        // Check caster has inventory space
        let caster = ctx
            .state
            .entities
            .actor(ctx.caster)
            .ok_or(ActionError::ActorNotFound)?;

        if caster.inventory.is_full() {
            return Err(ActionError::EffectFailed("Inventory is full".to_string()));
        }

        Ok(())
    }

    /// Apply item acquisition: transfer from world to inventory.
    pub fn apply(&self, ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // Get target item ID from ActionInput
        let item_id = match ctx.action_input {
            ActionInput::Target(id) => *id,
            _ => {
                return Err(ActionError::EffectFailed(
                    "AcquireItemEffect requires Target input".to_string(),
                ));
            }
        };

        // Get item (validate exists)
        let item = ctx
            .state
            .entities
            .item(item_id)
            .ok_or_else(|| ActionError::EffectFailed(format!("Item {} not found", item_id)))?;

        // Store item data before removing from world
        let handle = item.handle;
        let quantity = item.quantity;

        // Add to caster's inventory first (before removing from world)
        let caster = ctx
            .state
            .entities
            .actor_mut(ctx.caster)
            .ok_or(ActionError::ActorNotFound)?;

        caster
            .inventory
            .add_item(handle, quantity)
            .map_err(|e| ActionError::EffectFailed(format!("Failed to add to inventory: {}", e)))?;

        // Remove item entity from world
        let item_index = ctx
            .state
            .entities
            .items
            .iter()
            .position(|i| i.id == item_id)
            .ok_or_else(|| ActionError::EffectFailed("Item not found in world".to_string()))?;

        let _ = ctx.state.entities.items.remove(item_index);

        Ok(AppliedValue::ItemAcquired {
            item_id,
            handle,
            quantity,
        })
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for AcquireItem effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::Primary
    }
}

impl Default for AcquireItemEffect {
    fn default() -> Self {
        Self::new()
    }
}

/// Use a consumable item from inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UseConsumableEffect;

impl UseConsumableEffect {
    /// Create a new UseConsumable effect.
    pub fn new() -> Self {
        Self
    }

    /// Pre-validate: Check if inventory slot has a consumable.
    pub fn pre_validate(&self, ctx: &EffectContext) -> Result<(), ActionError> {
        // Get inventory slot from ActionInput
        let slot = match ctx.action_input {
            ActionInput::InventorySlot(s) => *s,
            _ => {
                return Err(ActionError::EffectFailed(
                    "UseConsumableEffect requires InventorySlot input".to_string(),
                ));
            }
        };

        // Get caster's inventory
        let caster = ctx
            .state
            .entities
            .actor(ctx.caster)
            .ok_or(ActionError::ActorNotFound)?;

        // Check slot exists and has an item
        let item_state = caster.inventory.get_slot(slot as usize).ok_or_else(|| {
            ActionError::EffectFailed(format!("Inventory slot {} is empty", slot))
        })?;

        // Get item definition from oracle
        let items_oracle = ctx
            .env
            .items()
            .map_err(|_| ActionError::ItemsNotAvailable)?;
        let item_def = items_oracle.definition(item_state.handle).ok_or_else(|| {
            ActionError::EffectFailed(format!(
                "Item definition not found for handle {:?}",
                item_state.handle
            ))
        })?;

        // Check it's a consumable
        if !matches!(item_def.kind, crate::env::ItemKind::Consumable(_)) {
            return Err(ActionError::EffectFailed(
                "Item is not consumable".to_string(),
            ));
        }

        Ok(())
    }

    /// Apply consumable use: execute effects and decrease quantity.
    pub fn apply(&self, ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // Get inventory slot
        let slot = match ctx.action_input {
            ActionInput::InventorySlot(s) => *s,
            _ => {
                return Err(ActionError::EffectFailed(
                    "UseConsumableEffect requires InventorySlot input".to_string(),
                ));
            }
        };

        // Get item from inventory
        let caster = ctx
            .state
            .entities
            .actor(ctx.caster)
            .ok_or(ActionError::ActorNotFound)?;

        let item_state = caster.inventory.get_slot(slot as usize).ok_or_else(|| {
            ActionError::EffectFailed(format!("Inventory slot {} is empty", slot))
        })?;

        let handle = item_state.handle;

        // Get item definition
        let items_oracle = ctx
            .env
            .items()
            .map_err(|_| ActionError::ItemsNotAvailable)?;
        let item_def = items_oracle.definition(handle).ok_or_else(|| {
            ActionError::EffectFailed(format!("Item definition not found for handle {:?}", handle))
        })?;

        // Get consumable data
        let consumable_data = match &item_def.kind {
            crate::env::ItemKind::Consumable(data) => data,
            _ => {
                return Err(ActionError::EffectFailed(
                    "Item is not consumable".to_string(),
                ));
            }
        };

        // Execute all consumable effects
        // Note: We're executing effects here, but they operate on the same EffectContext
        // This means the consumable effects will affect the caster (ctx.target = ctx.caster for consumables)
        for effect in &consumable_data.effects {
            // Apply each effect
            // Note: This is a simplified version. In a full implementation,
            // we would need to handle effect ordering, phases, etc.
            effect.kind.apply(ctx)?;
        }

        // Decrease quantity
        let caster_mut = ctx
            .state
            .entities
            .actor_mut(ctx.caster)
            .ok_or(ActionError::ActorNotFound)?;

        caster_mut
            .inventory
            .decrease_quantity(slot as usize, 1)
            .map_err(|e| {
                ActionError::EffectFailed(format!("Failed to decrease item quantity: {}", e))
            })?;

        Ok(AppliedValue::ItemUsed { slot, handle })
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for UseConsumable effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::Primary
    }
}

impl Default for UseConsumableEffect {
    fn default() -> Self {
        Self::new()
    }
}
