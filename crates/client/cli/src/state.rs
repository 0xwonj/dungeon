//! Application state for mode management and UI context.

use crate::cursor::CursorState;
use game_core::{EntityId, Position};

/// Top-level application mode determining input handling and UI layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppMode {
    /// Normal gameplay mode with auto-target tracking.
    Normal,
    /// Manual examine mode for inspecting tiles and entities.
    ExamineManual,
    /// Targeting mode for selecting attack/ability targets (future).
    #[allow(dead_code)]
    Targeting { action_type: TargetingAction },
    /// Inventory management mode (future).
    #[allow(dead_code)]
    Inventory,
}

/// Type of action requiring targeting (future).
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TargetingAction {
    Attack,
    UseItem { slot: u8 },
    Interact,
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
}

impl AppState {
    /// Returns the current examine position based on highlighted entity or cursor.
    ///
    /// This is used by examine panel to show tile information.
    /// Returns the position of the highlighted entity, or cursor position in manual mode.
    pub fn examine_position(&self) -> Option<Position> {
        match self.mode {
            AppMode::ExamineManual | AppMode::Targeting { .. } => {
                self.manual_cursor.as_ref().map(|c| c.position)
            }
            AppMode::Normal | AppMode::Inventory => None,
        }
    }

    /// Returns true if using manual cursor (not auto-target).
    pub fn is_manual_cursor(&self) -> bool {
        matches!(
            self.mode,
            AppMode::ExamineManual | AppMode::Targeting { .. }
        )
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Normal,
            highlighted_entity: None,
            manual_cursor: None,
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

    /// Enters Targeting mode with cursor and valid targets (future).
    #[allow(dead_code)]
    pub fn enter_targeting(&mut self, position: Position, action_type: TargetingAction) {
        self.mode = AppMode::Targeting { action_type };
        self.manual_cursor = Some(CursorState::new(position));
        self.highlighted_entity = None;
    }

    /// Exits to Normal mode (auto-target).
    pub fn exit_to_normal(&mut self) {
        self.mode = AppMode::Normal;
        self.manual_cursor = None;
        // Keep highlighted_entity - will be recomputed by auto-targeting
    }

    /// Returns true if currently in a modal mode requiring manual input.
    pub fn is_modal(&self) -> bool {
        !matches!(self.mode, AppMode::Normal)
    }
}
