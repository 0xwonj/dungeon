//! Framework-agnostic presentation mapping traits.
//!
//! This module defines abstractions for mapping game entities to visual representations
//! (glyphs, colors, styles) without depending on any specific UI framework.
//! Each frontend (TUI, GUI, Web) implements these traits with their own styling system.

use crate::message::MessageLevel;
use game_core::{PropKind, TerrainKind, stats::StatsSnapshot};

/// Framework-agnostic presentation mapper for game entities.
///
/// This trait allows each frontend to define its own visual style
/// (colors, fonts, icons) while sharing the same view model logic.
///
/// # Example
///
/// ```ignore
/// // TUI implementation with Ratatui
/// impl PresentationMapper for RatatuiTheme {
///     type Style = ratatui::style::Style;
///
///     fn render_actor(&self, stats: &StatsSnapshot, is_player: bool, is_current: bool) -> (String, Self::Style) {
///         let (glyph, color) = if is_player {
///             ("@", Color::Yellow)
///         } else {
///             ("n", Color::LightRed)
///         };
///         let mut style = Style::default().fg(color);
///         if is_current {
///             style = style.add_modifier(Modifier::BOLD);
///         }
///         (glyph.to_string(), style)
///     }
/// }
/// ```
pub trait PresentationMapper {
    /// Style type for this frontend (e.g., `ratatui::style::Style`).
    type Style: Clone;

    /// Render an actor (player or NPC) to glyph and style.
    ///
    /// Uses `game_core::StatsSnapshot` directly - the same type used in ZK proofs.
    fn render_actor(
        &self,
        stats: &StatsSnapshot,
        is_player: bool,
        is_current: bool,
    ) -> (String, Self::Style);

    /// Render a prop to glyph and style.
    fn render_prop(&self, kind: &PropKind, is_active: bool) -> (String, Self::Style);

    /// Render terrain to glyph and style.
    fn render_terrain(&self, terrain: TerrainKind, has_items: bool) -> (String, Self::Style);

    /// Style for health display.
    ///
    /// Takes current/max values directly from `StatsSnapshot.hp()`.
    fn style_health(&self, current: u32, maximum: u32) -> Self::Style;

    /// Style for energy display.
    ///
    /// Takes current/max values directly from `StatsSnapshot.mp()`.
    fn style_energy(&self, current: u32, maximum: u32) -> Self::Style;

    /// Style for message log entries based on level.
    fn style_message(&self, level: MessageLevel) -> Self::Style;

    /// Emphasize current actor (adds bold, glow, etc).
    fn emphasize_current(&self, base_style: Self::Style) -> Self::Style;
}
