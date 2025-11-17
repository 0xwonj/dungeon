//! Application state for mode management and UI context.

use crate::cursor::CursorState;
use game_core::{ActionKind, EntityId, Position};

/// Top-level application mode determining input handling and UI layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppMode {
    /// Normal gameplay mode with auto-target tracking.
    Normal,
    /// Manual examine mode for inspecting tiles and entities.
    ExamineManual,
    /// Ability menu for assigning actions to hotkey slots.
    AbilityMenu,
    /// Targeting mode for selecting attack/ability targets.
    Targeting(TargetingState),
    /// Inventory management mode (future).
    #[allow(dead_code)]
    Inventory,
}

/// State for targeting mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetingState {
    /// The action being executed.
    pub action_kind: ActionKind,
    /// The targeting input mode based on action's targeting requirements.
    pub input_mode: TargetingInputMode,
}

/// Targeting input mode based on action's TargetingMode.
///
/// **Design:**
/// This is the UI-layer representation of how to collect targeting input.
/// Uses unified position-based cursor targeting for consistency with examine mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetingInputMode {
    /// Position-based targeting (unified for entity and position targets).
    ///
    /// Use arrow keys to move cursor, Tab to cycle entities at cursor position, Enter to confirm.
    /// The examine panel automatically shows details of entities at cursor position.
    Position {
        /// If true, require an entity at cursor position to confirm (SingleTarget actions).
        /// If false, allow empty tiles (AOE, teleportation, etc).
        require_entity: bool,
        /// Optional max range from player (for range-limited abilities).
        max_range: Option<u32>,
    },

    /// Direction selection (directional actions like Move, Dash).
    ///
    /// Arrow key press immediately confirms direction.
    Direction {
        /// The selected direction (None until key pressed).
        selected: Option<game_core::CardinalDirection>,
    },
}

impl TargetingInputMode {
    /// Creates a TargetingInputMode from game-core's TargetingMode.
    ///
    /// **Important:** This should only be called for targeting modes that
    /// require user input. `None` and `SelfOnly` should execute immediately
    /// without entering targeting mode.
    ///
    /// # Arguments
    /// * `mode` - The targeting mode from action profile
    ///
    /// # Returns
    /// * `Some(input_mode)` if user input is required
    /// * `None` if action should execute immediately (None/SelfOnly)
    pub fn from_targeting_mode(mode: &game_core::TargetingMode) -> Option<Self> {
        match mode {
            // No targeting needed - execute immediately
            game_core::TargetingMode::None | game_core::TargetingMode::SelfOnly => None,

            // Entity targeting - use cursor with entity requirement
            game_core::TargetingMode::SingleTarget { range, .. } => Some(Self::Position {
                require_entity: true,
                max_range: Some(*range),
            }),

            // Direction targeting - arrow keys
            game_core::TargetingMode::Directional { .. } => {
                Some(Self::Direction { selected: None })
            }
        }
    }
}

/// Mutable application state tracking current mode and cursor.
///
/// **Design Philosophy:**
/// - Single source of truth: `highlighted_entity` drives all highlighting
/// - EntityId-based tracking: survives entity movement and state changes
/// - Mode-specific behavior: Normal (auto-target) vs Manual (cursor-based)
#[derive(Clone, Debug)]
pub struct AppState {
    /// Current application mode.
    pub mode: AppMode,
    /// Currently highlighted entity (synchronized with map highlight and examine panel).
    ///
    /// - **Normal mode**: Set by auto-targeting logic, cycled with Tab key
    /// - **Manual mode**: Set by cursor position, cycled with Tab for entities at same position
    pub highlighted_entity: Option<EntityId>,
    /// Manual cursor (only present in ExamineManual or Targeting mode).
    pub manual_cursor: Option<CursorState>,
    /// Action hotkey slots (1-9 keys) - user-configurable
    pub action_slots: ActionSlots,
}

impl AppState {
    /// Returns the current examine position based on highlighted entity or cursor.
    ///
    /// This is used by examine panel to show tile information.
    /// Returns the position of the highlighted entity, or cursor position in manual mode.
    pub fn examine_position(&self) -> Option<Position> {
        match self.mode {
            AppMode::ExamineManual | AppMode::Targeting(_) => {
                self.manual_cursor.as_ref().map(|c| c.position)
            }
            AppMode::Normal | AppMode::AbilityMenu | AppMode::Inventory => None,
        }
    }

    /// Returns true if using manual cursor (not auto-target).
    pub fn is_manual_cursor(&self) -> bool {
        matches!(self.mode, AppMode::ExamineManual | AppMode::Targeting(_))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Normal,
            highlighted_entity: None,
            manual_cursor: None,
            action_slots: ActionSlots::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the highlighted entity (used by auto-targeting and entity cycling).
    pub fn set_highlighted_entity(&mut self, entity_id: Option<EntityId>) {
        self.highlighted_entity = entity_id;
    }

    /// Enters manual Examine mode with cursor at the given position.
    pub fn enter_examine_manual(&mut self, position: Position, entity_at_cursor: Option<EntityId>) {
        self.mode = AppMode::ExamineManual;
        self.manual_cursor = Some(CursorState::new(position));
        self.highlighted_entity = entity_at_cursor;
    }

    /// Toggles between Normal (auto-target) and ExamineManual mode.
    ///
    /// When entering manual mode, cursor is placed at highlighted entity's position (or fallback).
    pub fn toggle_examine(&mut self, fallback_position: Position) {
        match self.mode {
            AppMode::Normal => {
                // Enter manual mode at current highlighted entity's position or fallback
                let pos = fallback_position; // Will be set properly by caller
                self.enter_examine_manual(pos, self.highlighted_entity);
            }
            AppMode::ExamineManual => {
                // Return to auto-target mode
                self.exit_to_normal();
            }
            _ => {}
        }
    }

    /// Enters ability menu mode.
    pub fn enter_ability_menu(&mut self) {
        self.mode = AppMode::AbilityMenu;
        self.manual_cursor = None;
    }

    /// Enters targeting mode with the specified targeting state.
    pub fn enter_targeting(&mut self, targeting_state: TargetingState, cursor_position: Position) {
        self.mode = AppMode::Targeting(targeting_state);
        self.manual_cursor = Some(CursorState::new(cursor_position));
        // highlighted_entity will be set by targeting logic
    }

    /// Exits to Normal mode (auto-target).
    pub fn exit_to_normal(&mut self) {
        self.mode = AppMode::Normal;
        self.manual_cursor = None;
        // Keep highlighted_entity - will be recomputed by auto-targeting
    }
}

// ============================================================================
// Action Slots
// ============================================================================

/// Action hotkey slots (1-9 keys).
///
/// **Design:**
/// - Fixed 9 slots mapped to keys 1-9
/// - Slot 0 (key '1') defaults to MeleeAttack for bump-to-attack
/// - Future: Serialization for persistent configuration
#[derive(Clone, Debug)]
pub struct ActionSlots {
    /// 9 slots: index 0 = key '1', index 8 = key '9'
    slots: [Option<ActionKind>; 9],
}

impl ActionSlots {
    /// Creates default action slots with MeleeAttack in slot 1.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the action assigned to the given slot (0-8).
    pub fn get(&self, slot: usize) -> Option<ActionKind> {
        self.slots.get(slot).copied().flatten()
    }

    /// Assigns an action to the given slot (0-8).
    pub fn set(&mut self, slot: usize, action: Option<ActionKind>) {
        if let Some(s) = self.slots.get_mut(slot) {
            *s = action;
        }
    }
}

impl Default for ActionSlots {
    fn default() -> Self {
        let mut slots = [None; 9];
        // Slot 0 (key '1'): Default melee attack for bump-to-attack
        slots[0] = Some(ActionKind::MeleeAttack);
        Self { slots }
    }
}
