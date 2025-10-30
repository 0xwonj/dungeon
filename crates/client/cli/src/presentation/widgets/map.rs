//! Map widget rendering the 2D game grid with entities.
//!
//! This widget fully leverages PresentationMapper for framework-independent styling.

use client_core::view_model::{PresentationMapper, ViewModel};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::AppState;

/// Render the map panel showing the 2D grid with terrain and entities.
///
/// **NEW ARCHITECTURE**: This widget demonstrates full PresentationMapper usage:
/// - All actors styled via theme.render_actor()
/// - All props styled via theme.render_prop()
/// - All terrain styled via theme.render_terrain()
/// - Framework-independent color logic
pub fn render<T: PresentationMapper<Style = Style>>(
    frame: &mut Frame,
    area: Rect,
    view_model: &ViewModel,
    app_state: &AppState,
    theme: &T,
) {
    // Get highlighted position from highlighted entity or cursor
    let highlighted_pos = if let Some(entity_id) = app_state.highlighted_entity {
        // Find the position of the highlighted entity
        view_model
            .actors
            .iter()
            .find(|a| a.id == entity_id)
            .map(|a| a.position)
    } else {
        // Fallback: cursor position in manual mode
        app_state.examine_position()
    };

    let mut rows = Vec::with_capacity(view_model.map.tiles.len());

    for row in &view_model.map.tiles {
        let spans: Vec<Span> = row
            .iter()
            .map(|tile| {
                let position = tile.position;
                let is_highlighted = Some(position) == highlighted_pos;

                // Priority: Actor > Prop > Item > Terrain
                // Check for actors at this position
                if let Some(actor) = view_model.actors.iter().find(|a| a.position == position) {
                    let is_current = view_model.turn.current_actor == actor.id;
                    let (glyph, mut style) =
                        theme.render_actor(&actor.stats, actor.is_player, is_current);

                    // Highlight the position if this is the highlighted entity
                    if is_highlighted {
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    }

                    return Span::styled(glyph, style);
                }

                // Check for props at this position
                if let Some(prop) = view_model.props.iter().find(|p| p.position == position) {
                    let (glyph, mut style) = theme.render_prop(&prop.kind, prop.is_active);

                    if is_highlighted {
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    }

                    return Span::styled(glyph, style);
                }

                // Check for items at this position (computed dynamically from ViewModel.items)
                let has_items = view_model.items.iter().any(|i| i.position == position);

                // Render terrain (with item indicator if present)
                let (glyph, mut style) = theme.render_terrain(tile.terrain, has_items);

                // Highlight position
                if is_highlighted {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }

                Span::styled(glyph, style)
            })
            .collect();

        rows.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(rows).block(Block::default().borders(Borders::ALL).title("Map"));

    frame.render_widget(paragraph, area);
}
