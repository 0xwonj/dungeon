//! Application state for mode management and UI context.

use crate::cursor::CursorState;
use game_core::Position;

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
#[derive(Clone, Debug)]
pub struct AppState {
    /// Current application mode.
    pub mode: AppMode,
    /// Auto-target position (always present, computed each frame in Normal mode).
    pub auto_target_position: Option<Position>,
    /// Manual cursor (only present in ExamineManual or Targeting mode).
    pub manual_cursor: Option<CursorState>,
    /// Index of currently selected entity at cursor position (for cycling with Tab).
    pub entity_index: usize,
}

impl AppState {
    /// Returns the current examine position based on mode.
    pub fn examine_position(&self) -> Option<Position> {
        match self.mode {
            AppMode::Normal => self.auto_target_position,
            AppMode::ExamineManual | AppMode::Targeting { .. } => {
                self.manual_cursor.as_ref().map(|c| c.position)
            }
            AppMode::Inventory => None,
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
            auto_target_position: None,
            manual_cursor: None,
            entity_index: 0,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates auto-target position (called each frame in Normal mode).
    pub fn set_auto_target(&mut self, position: Option<Position>) {
        self.auto_target_position = position;
    }

    /// Enters manual Examine mode with cursor at the given position.
    pub fn enter_examine_manual(&mut self, position: Position) {
        self.mode = AppMode::ExamineManual;
        self.manual_cursor = Some(CursorState::new(position));
        self.entity_index = 0;
    }

    /// Toggles between Normal (auto-target) and ExamineManual mode.
    pub fn toggle_examine(&mut self, fallback_position: Position) {
        match self.mode {
            AppMode::Normal => {
                // Enter manual mode at current auto-target or fallback
                let pos = self.auto_target_position.unwrap_or(fallback_position);
                self.enter_examine_manual(pos);
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
        self.entity_index = 0;
    }

    /// Exits to Normal mode (auto-target).
    pub fn exit_to_normal(&mut self) {
        self.mode = AppMode::Normal;
        self.manual_cursor = None;
        self.entity_index = 0;
    }

    /// Returns true if currently in a modal mode requiring manual input.
    pub fn is_modal(&self) -> bool {
        !matches!(self.mode, AppMode::Normal)
    }

    /// Cycles to the next entity at the current cursor position.
    pub fn next_entity(&mut self) {
        self.entity_index = self.entity_index.wrapping_add(1);
    }

    /// Cycles to the previous entity at the current cursor position.
    pub fn prev_entity(&mut self) {
        self.entity_index = self.entity_index.wrapping_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_normal() {
        let state = AppState::new();
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.manual_cursor.is_none());
    }

    #[test]
    fn enter_examine_manual_sets_cursor() {
        let mut state = AppState::new();
        state.enter_examine_manual(Position::new(5, 10));
        assert_eq!(state.mode, AppMode::ExamineManual);
        assert!(state.manual_cursor.is_some());
        assert_eq!(state.manual_cursor.unwrap().position, Position::new(5, 10));
    }

    #[test]
    fn toggle_examine_switches_modes() {
        let mut state = AppState::new();
        state.set_auto_target(Some(Position::new(3, 4)));

        // Toggle to manual
        state.toggle_examine(Position::ORIGIN);
        assert_eq!(state.mode, AppMode::ExamineManual);
        assert_eq!(state.examine_position(), Some(Position::new(3, 4)));

        // Toggle back to auto
        state.toggle_examine(Position::ORIGIN);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.manual_cursor.is_none());
    }

    #[test]
    fn exit_to_normal_clears_manual_cursor() {
        let mut state = AppState::new();
        state.enter_examine_manual(Position::ORIGIN);
        state.exit_to_normal();
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.manual_cursor.is_none());
    }
}
