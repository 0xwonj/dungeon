//! Map widget rendering the 2D game grid with entities.
//!
//! This widget fully leverages PresentationMapper for framework-independent styling.

use client_core::view_model::{PresentationMapper, ViewModel};
use game_core::{CardinalDirection, Position};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::{AppMode, AppState, TargetingInputMode};

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

    // Compute targeting visualization data
    let targeting_info = compute_targeting_visualization(view_model, app_state);

    let mut rows = Vec::with_capacity(view_model.map.tiles.len());

    for row in &view_model.map.tiles {
        let spans: Vec<Span> = row
            .iter()
            .map(|tile| {
                let position = tile.position;
                let is_highlighted = Some(position) == highlighted_pos;

                // Check targeting visualization
                let in_range = targeting_info.range_positions.contains(&position);
                let in_directional_path = targeting_info.directional_path.contains(&position);
                let is_valid_target = targeting_info.valid_target_positions.contains(&position);

                // Priority: Actor > Prop > Item > Terrain
                // Check for actors at this position
                if let Some(actor) = view_model.actors.iter().find(|a| a.position == position) {
                    let is_current = view_model.turn.current_actor == actor.id;
                    let (glyph, mut style) =
                        theme.render_actor(&actor.stats, actor.is_player, is_current);

                    // Apply targeting visualization
                    if is_highlighted && targeting_info.is_targeting {
                        // Current target: bright highlight
                        style = style.bg(Color::Yellow).add_modifier(Modifier::BOLD);
                    } else if is_valid_target {
                        // Valid target in range: subtle highlight
                        style = style.bg(Color::Rgb(40, 40, 0)).add_modifier(Modifier::BOLD);
                    } else if is_highlighted {
                        // Normal highlight (non-targeting mode)
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    }

                    return Span::styled(glyph, style);
                }

                // Check for props at this position
                if let Some(prop) = view_model.props.iter().find(|p| p.position == position) {
                    let (glyph, mut style) = theme.render_prop(&prop.kind, prop.is_active);

                    if is_highlighted {
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    } else if in_directional_path {
                        // Directional path highlight
                        style = style.bg(Color::Rgb(0, 30, 30));
                    } else if in_range {
                        // Range indicator
                        style = style.bg(Color::Rgb(20, 20, 30));
                    }

                    return Span::styled(glyph, style);
                }

                // Check for items at this position (computed dynamically from ViewModel.items)
                let has_items = view_model.items.iter().any(|i| i.position == position);

                // Render terrain (with item indicator if present)
                let (glyph, mut style) = theme.render_terrain(tile.terrain, has_items);

                // Apply targeting visualization to terrain
                if is_highlighted {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                } else if in_directional_path {
                    // Directional path: cyan tint
                    style = style.bg(Color::Rgb(0, 30, 30));
                } else if in_range {
                    // Range indicator: subtle blue tint
                    style = style.bg(Color::Rgb(20, 20, 30));
                }

                Span::styled(glyph, style)
            })
            .collect();

        rows.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(rows).block(Block::default().borders(Borders::ALL).title("Map"));

    frame.render_widget(paragraph, area);
}

/// Targeting visualization data computed from app state.
#[derive(Default)]
struct TargetingVisualization {
    is_targeting: bool,
    range_positions: Vec<Position>,
    directional_path: Vec<Position>,
    valid_target_positions: Vec<Position>,
}

/// Compute targeting visualization based on current targeting mode.
fn compute_targeting_visualization(
    view_model: &ViewModel,
    app_state: &AppState,
) -> TargetingVisualization {
    let AppMode::Targeting(targeting_state) = &app_state.mode else {
        return TargetingVisualization::default();
    };

    let player_pos = view_model.player.position;

    match &targeting_state.input_mode {
        TargetingInputMode::Position {
            max_range,
            require_entity,
        } => {
            // If require_entity, highlight all valid entities within range
            let valid_target_positions: Vec<Position> = if *require_entity {
                view_model
                    .actors
                    .iter()
                    .filter(|actor| {
                        actor.id != game_core::EntityId::PLAYER
                            && actor.stats.resource_current.hp > 0
                            && max_range
                                .map(|r| chebyshev_distance(player_pos, actor.position) <= r)
                                .unwrap_or(true)
                    })
                    .map(|a| a.position)
                    .collect()
            } else {
                vec![]
            };

            TargetingVisualization {
                is_targeting: true,
                range_positions: vec![], // TODO: Visualize range indicator
                directional_path: vec![],
                valid_target_positions,
            }
        }

        TargetingInputMode::Direction { selected } => {
            // Show directional path if direction is selected
            let directional_path = if let Some(direction) = selected {
                compute_directional_path(player_pos, *direction, 5) // Assume max range 5
            } else {
                vec![]
            };

            TargetingVisualization {
                is_targeting: true,
                range_positions: vec![],
                directional_path,
                valid_target_positions: vec![],
            }
        }
    }
}

/// Compute positions along a directional path.
fn compute_directional_path(
    start: Position,
    direction: CardinalDirection,
    max_range: u32,
) -> Vec<Position> {
    let (dx, dy) = direction.offset();
    (1..=max_range)
        .map(|i| Position::new(start.x + dx * i as i32, start.y + dy * i as i32))
        .collect()
}

/// Calculate Chebyshev distance (chessboard distance) for range checks.
#[allow(dead_code)]
fn chebyshev_distance(from: Position, to: Position) -> u32 {
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();
    dx.max(dy) as u32
}
