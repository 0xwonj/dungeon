//! Ratatui theme implementation of PresentationMapper.
//!
//! This module provides concrete styling for the terminal UI, implementing
//! the framework-agnostic PresentationMapper trait from client-core.

use client_frontend_core::{message::MessageLevel, view_model::PresentationMapper};
use game_core::{PropKind, TerrainKind, stats::StatsSnapshot};
use ratatui::style::{Color, Modifier, Style};

/// Ratatui-specific theme implementing PresentationMapper.
///
/// This provides consistent color schemes and styling rules for the CLI.
pub struct RatatuiTheme;

impl PresentationMapper for RatatuiTheme {
    type Style = Style;

    fn render_actor(
        &self,
        stats: &StatsSnapshot,
        is_player: bool,
        is_current: bool,
    ) -> (String, Self::Style) {
        let glyph = if is_player {
            "@".to_string()
        } else if is_current {
            "N".to_string()
        } else {
            "n".to_string()
        };

        let base_color = if is_player {
            Color::Yellow
        } else {
            Color::LightRed
        };

        let mut style = Style::default().fg(base_color);
        if is_current {
            style = self.emphasize_current(style);
        }

        // Add visual indicator for low health
        let (current_hp, max_hp) = stats.hp();
        if current_hp > 0 && current_hp < max_hp / 4 {
            // Less than 25% health: add dimming
            style = style.add_modifier(Modifier::DIM);
        }

        (glyph, style)
    }

    fn render_prop(&self, kind: &PropKind, is_active: bool) -> (String, Self::Style) {
        match kind {
            PropKind::Door => (
                if is_active { "/" } else { "+" }.to_string(),
                Style::default().fg(Color::Green),
            ),
            PropKind::Switch => ("^".to_string(), Style::default().fg(Color::LightBlue)),
            PropKind::Hazard => ("!".to_string(), Style::default().fg(Color::Magenta)),
            PropKind::Other => ("&".to_string(), Style::default().fg(Color::White)),
        }
    }

    fn render_terrain(&self, terrain: TerrainKind, has_items: bool) -> (String, Self::Style) {
        if has_items {
            return ("*".to_string(), Style::default().fg(Color::LightCyan));
        }

        let (glyph, color) = match terrain {
            TerrainKind::Floor => ('.', Color::DarkGray),
            TerrainKind::Wall => ('#', Color::Gray),
            TerrainKind::Void => (' ', Color::Reset),
            TerrainKind::Water => ('~', Color::Blue),
            TerrainKind::Custom(_) => ('?', Color::LightMagenta),
        };

        (glyph.to_string(), Style::default().fg(color))
    }

    fn style_health(&self, current: u32, maximum: u32) -> Self::Style {
        if maximum == 0 {
            return Style::default().fg(Color::Gray);
        }

        let percent = (current * 100) / maximum;
        let color = match percent {
            75..=100 => Color::Green,
            50..=74 => Color::Yellow,
            25..=49 => Color::LightRed,
            _ => Color::Red,
        };

        Style::default().fg(color)
    }

    fn style_energy(&self, current: u32, maximum: u32) -> Self::Style {
        if maximum == 0 {
            return Style::default().fg(Color::Gray);
        }

        let percent = (current * 100) / maximum;
        let color = match percent {
            75..=100 => Color::Cyan,
            50..=74 => Color::Blue,
            25..=49 => Color::LightBlue,
            _ => Color::DarkGray,
        };

        Style::default().fg(color)
    }

    fn style_message(&self, level: MessageLevel) -> Self::Style {
        match level {
            MessageLevel::Info => Style::default().fg(Color::White),
            MessageLevel::Warning => Style::default().fg(Color::Yellow),
            MessageLevel::Error => Style::default().fg(Color::LightRed),
        }
    }

    fn emphasize_current(&self, base_style: Self::Style) -> Self::Style {
        base_style.add_modifier(Modifier::BOLD)
    }
}

impl RatatuiTheme {
    /// Create a new RatatuiTheme instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RatatuiTheme {
    fn default() -> Self {
        Self::new()
    }
}
