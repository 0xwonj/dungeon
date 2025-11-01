//! Input processing for the CLI client.
//!
//! This module owns the keyboard-to-command mapping so the rest of the
//! application can remain agnostic about concrete key bindings or the
//! specifics of `crossterm` events.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use game_core::{Action, CardinalDirection, CharacterActionKind, EntityId, MoveAction, WaitAction};

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
    /// No meaningful command was produced.
    None,
}

/// Translates `KeyEvent`s into game commands using a configurable key map.
pub struct InputHandler {
    player_entity: EntityId,
    is_modal: bool,
}

impl InputHandler {
    pub fn new(player_entity: EntityId) -> Self {
        Self {
            player_entity,
            is_modal: false,
        }
    }

    /// Updates the entity the handler should bind actions to.
    pub fn set_player_entity(&mut self, player_entity: EntityId) {
        self.player_entity = player_entity;
    }

    /// Updates whether we're in a modal mode (affects input interpretation).
    pub fn set_modal(&mut self, is_modal: bool) {
        self.is_modal = is_modal;
    }

    /// Converts a raw key event into a higher-level command.
    pub fn handle_key(&self, key: KeyEvent) -> KeyAction {
        // Modal mode inputs
        if self.is_modal {
            return self.handle_modal_key(key);
        }

        // Normal mode inputs
        match key.code {
            KeyCode::Char(ch) => self.handle_char(ch),
            KeyCode::Left => self.movement(CardinalDirection::West),
            KeyCode::Right => self.movement(CardinalDirection::East),
            KeyCode::Up => self.movement(CardinalDirection::North),
            KeyCode::Down => self.movement(CardinalDirection::South),
            KeyCode::Enter => self.wait(),
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

    fn handle_char(&self, raw: char) -> KeyAction {
        let ch = raw.to_ascii_lowercase();
        match ch {
            'q' => KeyAction::Quit,
            '.' | ' ' => self.wait(),
            'x' => KeyAction::ToggleExamine,
            _ => KeyAction::None,
        }
    }

    fn handle_modal_key(&self, key: KeyEvent) -> KeyAction {
        match key.code {
            KeyCode::Esc => KeyAction::ExitModal,
            KeyCode::Char('x') => KeyAction::ToggleExamine, // x also toggles in manual mode
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    KeyAction::PrevEntity
                } else {
                    KeyAction::NextEntity
                }
            }
            KeyCode::BackTab => KeyAction::PrevEntity,
            KeyCode::Char(_ch) => KeyAction::None,
            KeyCode::Left => KeyAction::MoveCursor(CardinalDirection::West),
            KeyCode::Right => KeyAction::MoveCursor(CardinalDirection::East),
            KeyCode::Up => KeyAction::MoveCursor(CardinalDirection::North),
            KeyCode::Down => KeyAction::MoveCursor(CardinalDirection::South),
            _ => KeyAction::None,
        }
    }

    fn movement(&self, direction: CardinalDirection) -> KeyAction {
        let action = MoveAction::new(self.player_entity, direction);
        KeyAction::Submit(Action::character(
            self.player_entity,
            CharacterActionKind::Move(action),
        ))
    }

    fn wait(&self) -> KeyAction {
        KeyAction::Submit(Action::character(
            self.player_entity,
            CharacterActionKind::Wait(WaitAction::new(self.player_entity)),
        ))
    }
}
