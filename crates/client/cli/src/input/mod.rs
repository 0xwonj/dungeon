//! Input processing for the CLI client.
//!
//! This module owns the keyboard-to-command mapping so the rest of the
//! application can remain agnostic about concrete key bindings or the
//! specifics of `crossterm` events.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use game_core::{Action, ActionInput, ActionKind, CardinalDirection, CharacterAction, EntityId};

pub mod provider;
pub use provider::CliActionProvider;

/// High-level outcome of processing a keyboard event.
#[derive(Debug)]
pub enum KeyAction {
    /// Exit the application.
    Quit,
    /// Submit the decoded game action to the runtime.
    Submit(Action),
    /// Toggle between Normal (auto-target) and ExamineManual mode.
    ToggleExamine,
    /// Exit current modal mode back to Normal.
    ExitModal,
    /// Move cursor in modal mode.
    MoveCursor(CardinalDirection),
    /// Cycle to next entity at cursor.
    NextEntity,
    /// Cycle to previous entity at cursor.
    PrevEntity,
    /// Directional input in Normal mode (bump-to-attack or move).
    DirectionalInput(CardinalDirection),
    /// Use action slot (0-8, corresponding to keys 1-9).
    UseSlot(usize),
    /// Open ability menu to view/assign actions.
    OpenAbilityMenu,
    /// Select ability from menu (ability list index, not slot).
    SelectAbilityForSlot(usize),
    /// Confirm target selection in targeting mode.
    ConfirmTarget,
    /// No meaningful command was produced.
    None,
}

/// Translates `KeyEvent`s into game commands using a configurable key map.
///
/// **Design:**
/// - Mode-aware input handling (Normal, Examine, AbilityMenu, Targeting)
/// - Directional input generates `DirectionalInput` in Normal mode (not Move)
/// - Bump-to-attack logic is handled in event loop, not here
pub struct InputHandler {
    player_entity: EntityId,
}

impl InputHandler {
    pub fn new(player_entity: EntityId) -> Self {
        Self { player_entity }
    }

    /// Updates the entity the handler should bind actions to.
    pub fn set_player_entity(&mut self, player_entity: EntityId) {
        self.player_entity = player_entity;
    }

    /// Converts a raw key event into a higher-level command.
    ///
    /// Mode is provided externally (from AppState) for cleaner separation.
    pub fn handle_key(&self, key: KeyEvent, mode: &crate::state::AppMode) -> KeyAction {
        use crate::state::AppMode;

        match mode {
            AppMode::Normal => self.handle_normal_mode(key),
            AppMode::ExamineManual => self.handle_examine_mode(key),
            AppMode::AbilityMenu => self.handle_ability_menu(key),
            AppMode::Targeting(targeting_state) => self.handle_targeting_mode(key, targeting_state),
            AppMode::Inventory => KeyAction::None, // TODO: Future
        }
    }

    /// Handle input in Normal mode (movement, action slots, menu).
    fn handle_normal_mode(&self, key: KeyEvent) -> KeyAction {
        match key.code {
            // Directional input: Bump-to-attack or Move (decided by event loop)
            KeyCode::Left => KeyAction::DirectionalInput(CardinalDirection::West),
            KeyCode::Right => KeyAction::DirectionalInput(CardinalDirection::East),
            KeyCode::Up => KeyAction::DirectionalInput(CardinalDirection::North),
            KeyCode::Down => KeyAction::DirectionalInput(CardinalDirection::South),

            // Vi keys for diagonal movement
            KeyCode::Char('h') => KeyAction::DirectionalInput(CardinalDirection::West),
            KeyCode::Char('l') => KeyAction::DirectionalInput(CardinalDirection::East),
            KeyCode::Char('k') => KeyAction::DirectionalInput(CardinalDirection::North),
            KeyCode::Char('j') => KeyAction::DirectionalInput(CardinalDirection::South),
            KeyCode::Char('y') => KeyAction::DirectionalInput(CardinalDirection::NorthWest),
            KeyCode::Char('u') => KeyAction::DirectionalInput(CardinalDirection::NorthEast),
            KeyCode::Char('b') => KeyAction::DirectionalInput(CardinalDirection::SouthWest),
            KeyCode::Char('n') => KeyAction::DirectionalInput(CardinalDirection::SouthEast),

            // Action slots (1-9)
            KeyCode::Char('1') => KeyAction::UseSlot(0),
            KeyCode::Char('2') => KeyAction::UseSlot(1),
            KeyCode::Char('3') => KeyAction::UseSlot(2),
            KeyCode::Char('4') => KeyAction::UseSlot(3),
            KeyCode::Char('5') => KeyAction::UseSlot(4),
            KeyCode::Char('6') => KeyAction::UseSlot(5),
            KeyCode::Char('7') => KeyAction::UseSlot(6),
            KeyCode::Char('8') => KeyAction::UseSlot(7),
            KeyCode::Char('9') => KeyAction::UseSlot(8),

            // Commands
            KeyCode::Char('a') => KeyAction::OpenAbilityMenu,
            KeyCode::Char('x') => KeyAction::ToggleExamine,
            KeyCode::Char(' ') | KeyCode::Char('.') => self.wait(),
            KeyCode::Char('q') => KeyAction::Quit,

            // Tab cycling (for auto-target in Normal mode)
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    KeyAction::PrevEntity
                } else {
                    KeyAction::NextEntity
                }
            }
            KeyCode::BackTab => KeyAction::PrevEntity,

            _ => KeyAction::None,
        }
    }

    /// Handle input in Examine mode (cursor movement, entity cycling).
    fn handle_examine_mode(&self, key: KeyEvent) -> KeyAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('x') => KeyAction::ExitModal,
            KeyCode::Left => KeyAction::MoveCursor(CardinalDirection::West),
            KeyCode::Right => KeyAction::MoveCursor(CardinalDirection::East),
            KeyCode::Up => KeyAction::MoveCursor(CardinalDirection::North),
            KeyCode::Down => KeyAction::MoveCursor(CardinalDirection::South),
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    KeyAction::PrevEntity
                } else {
                    KeyAction::NextEntity
                }
            }
            KeyCode::BackTab => KeyAction::PrevEntity,
            _ => KeyAction::None,
        }
    }

    /// Handle input in Ability Menu mode (select action to assign).
    fn handle_ability_menu(&self, key: KeyEvent) -> KeyAction {
        match key.code {
            KeyCode::Char('1') => KeyAction::SelectAbilityForSlot(0),
            KeyCode::Char('2') => KeyAction::SelectAbilityForSlot(1),
            KeyCode::Char('3') => KeyAction::SelectAbilityForSlot(2),
            KeyCode::Char('4') => KeyAction::SelectAbilityForSlot(3),
            KeyCode::Char('5') => KeyAction::SelectAbilityForSlot(4),
            KeyCode::Char('6') => KeyAction::SelectAbilityForSlot(5),
            KeyCode::Char('7') => KeyAction::SelectAbilityForSlot(6),
            KeyCode::Char('8') => KeyAction::SelectAbilityForSlot(7),
            KeyCode::Char('9') => KeyAction::SelectAbilityForSlot(8),
            KeyCode::Esc => KeyAction::ExitModal,
            _ => KeyAction::None,
        }
    }

    /// Handle input in Targeting mode (mode-specific input).
    fn handle_targeting_mode(
        &self,
        key: KeyEvent,
        targeting_state: &crate::state::TargetingState,
    ) -> KeyAction {
        use crate::state::TargetingInputMode;

        match &targeting_state.input_mode {
            TargetingInputMode::Position { .. } => {
                // Arrow keys to move cursor, Tab to cycle entities at cursor, Enter to confirm
                match key.code {
                    KeyCode::Left => KeyAction::MoveCursor(CardinalDirection::West),
                    KeyCode::Right => KeyAction::MoveCursor(CardinalDirection::East),
                    KeyCode::Up => KeyAction::MoveCursor(CardinalDirection::North),
                    KeyCode::Down => KeyAction::MoveCursor(CardinalDirection::South),
                    KeyCode::Tab => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            KeyAction::PrevEntity
                        } else {
                            KeyAction::NextEntity
                        }
                    }
                    KeyCode::BackTab => KeyAction::PrevEntity,
                    KeyCode::Enter => KeyAction::ConfirmTarget,
                    KeyCode::Esc => KeyAction::ExitModal,
                    _ => KeyAction::None,
                }
            }

            TargetingInputMode::Direction { .. } => {
                // Arrow keys select direction, Enter confirms
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        KeyAction::MoveCursor(CardinalDirection::West)
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        KeyAction::MoveCursor(CardinalDirection::East)
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        KeyAction::MoveCursor(CardinalDirection::North)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        KeyAction::MoveCursor(CardinalDirection::South)
                    }
                    KeyCode::Char('y') => KeyAction::MoveCursor(CardinalDirection::NorthWest),
                    KeyCode::Char('u') => KeyAction::MoveCursor(CardinalDirection::NorthEast),
                    KeyCode::Char('b') => KeyAction::MoveCursor(CardinalDirection::SouthWest),
                    KeyCode::Char('n') => KeyAction::MoveCursor(CardinalDirection::SouthEast),
                    KeyCode::Enter => KeyAction::ConfirmTarget,
                    KeyCode::Esc => KeyAction::ExitModal,
                    _ => KeyAction::None,
                }
            }
        }
    }

    fn wait(&self) -> KeyAction {
        let character_action =
            CharacterAction::new(self.player_entity, ActionKind::Wait, ActionInput::None);
        KeyAction::Submit(Action::Character(character_action))
    }
}
