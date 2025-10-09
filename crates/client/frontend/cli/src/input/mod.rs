//! Input processing for the CLI client.
//!
//! This module owns the keyboard-to-command mapping so the rest of the
//! application can remain agnostic about concrete key bindings or the
//! specifics of `crossterm` events.

use crossterm::event::{KeyCode, KeyEvent};
use game_core::{Action, ActionKind, CardinalDirection, EntityId, MoveAction};

pub mod provider;
pub use provider::CliActionProvider;

/// High-level outcome of processing a keyboard event.
#[derive(Debug)]
pub enum KeyAction {
    /// Exit the application.
    Quit,
    /// Submit the decoded game action to the runtime.
    Submit(Action),
    /// No meaningful command was produced.
    None,
}

/// Translates `KeyEvent`s into game commands using a configurable key map.
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
    pub fn handle_key(&self, key: KeyEvent) -> KeyAction {
        match key.code {
            KeyCode::Char(ch) => self.handle_char(ch),
            KeyCode::Left => self.movement(CardinalDirection::West),
            KeyCode::Right => self.movement(CardinalDirection::East),
            KeyCode::Up => self.movement(CardinalDirection::North),
            KeyCode::Down => self.movement(CardinalDirection::South),
            KeyCode::Enter => self.wait(),
            _ => KeyAction::None,
        }
    }

    fn handle_char(&self, raw: char) -> KeyAction {
        let ch = raw.to_ascii_lowercase();
        match ch {
            'q' => KeyAction::Quit,
            'h' | 'a' => self.movement(CardinalDirection::West),
            'j' | 's' => self.movement(CardinalDirection::South),
            'k' | 'w' => self.movement(CardinalDirection::North),
            'l' | 'd' => self.movement(CardinalDirection::East),
            '.' | ' ' => self.wait(),
            _ => KeyAction::None,
        }
    }

    fn movement(&self, direction: CardinalDirection) -> KeyAction {
        let action = MoveAction::new(self.player_entity, direction, 1);
        KeyAction::Submit(Action::new(self.player_entity, ActionKind::Move(action)))
    }

    fn wait(&self) -> KeyAction {
        KeyAction::Submit(Action::new(self.player_entity, ActionKind::Wait))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn maps_movement_keys() {
        let handler = InputHandler::new(EntityId::PLAYER);
        assert!(matches!(
            handler.handle_key(key(KeyCode::Char('h'))),
            KeyAction::Submit(_)
        ));
        assert!(matches!(
            handler.handle_key(key(KeyCode::Char('W'))),
            KeyAction::Submit(_)
        ));
        assert!(matches!(
            handler.handle_key(key(KeyCode::Left)),
            KeyAction::Submit(_)
        ));
    }

    #[test]
    fn maps_wait_and_quit() {
        let handler = InputHandler::new(EntityId::PLAYER);
        assert!(matches!(
            handler.handle_key(key(KeyCode::Char('.'))),
            KeyAction::Submit(_)
        ));
        assert!(matches!(
            handler.handle_key(key(KeyCode::Char('q'))),
            KeyAction::Quit
        ));
    }

    #[test]
    fn ignores_unknown_keys() {
        let handler = InputHandler::new(EntityId::PLAYER);
        assert!(matches!(
            handler.handle_key(key(KeyCode::Char('x'))),
            KeyAction::None
        ));
    }
}
