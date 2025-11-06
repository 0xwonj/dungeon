//! Examine widget for detailed entity and tile inspection.

use client_core::view_model::{PresentationMapper, ViewModel, entities::ActorView};
use game_core::{EntityId, Position, env::MapOracle};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Context for examine panel rendering.
///
/// Groups related parameters to reduce function argument count.
#[derive(Clone, Copy, Debug)]
pub struct ExamineContext {
    /// Currently highlighted entity (if any)
    pub highlighted_entity: Option<EntityId>,
    /// Cursor position in manual mode (if any)
    pub cursor_position: Option<Position>,
    /// Whether manual cursor mode is active
    pub is_manual: bool,
}

/// Render the examine panel showing tile and entity details.
///
/// Displays information about the highlighted entity and its tile.
/// - **Normal mode**: Shows highlighted entity from auto-targeting or Tab cycling
/// - **Manual mode**: Shows entity at cursor position (if any)
pub fn render<T: PresentationMapper<Style = Style>>(
    frame: &mut Frame,
    area: Rect,
    ctx: &ExamineContext,
    view_model: &ViewModel,
    map_oracle: &dyn MapOracle,
    theme: &T,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    // Determine tile position to display
    let tile_position = if let Some(entity_id) = ctx.highlighted_entity {
        // Show tile info for highlighted entity's position
        view_model
            .actors
            .iter()
            .find(|a| a.id == entity_id)
            .and_then(|a| a.position)
            .or(ctx.cursor_position)
            .or(view_model.player.position)
            .unwrap_or_else(|| game_core::Position::new(0, 0))
    } else {
        // No highlighted entity - use cursor or player position
        ctx.cursor_position
            .or(view_model.player.position)
            .unwrap_or_else(|| game_core::Position::new(0, 0))
    };

    // Top section: Tile info
    render_tile_info(
        frame,
        chunks[0],
        tile_position,
        view_model,
        map_oracle,
        ctx.is_manual,
    );

    // Bottom section: Entity details
    render_entity_details(frame, chunks[1], ctx.highlighted_entity, view_model, theme);
}

/// Render tile information section.
fn render_tile_info(
    frame: &mut Frame,
    area: Rect,
    position: Position,
    view_model: &ViewModel,
    map_oracle: &dyn MapOracle,
    is_manual: bool,
) {
    // Get terrain from map oracle
    let terrain = map_oracle
        .tile(position)
        .map(|t| format!("{:?}", t.terrain()))
        .unwrap_or_else(|| "Void".to_string());

    let passable = if map_oracle
        .tile(position)
        .as_ref()
        .is_some_and(|v| v.is_passable())
    {
        "Yes"
    } else {
        "No"
    };

    let occupied = if view_model
        .actors
        .iter()
        .any(|a| a.position == Some(position))
        || view_model.props.iter().any(|p| p.position == position)
    {
        "Yes"
    } else {
        "No"
    };

    let mode_indicator = if is_manual { "MANUAL" } else { "AUTO" };

    let lines = vec![
        Line::from(vec![
            Span::styled("Mode: ", Style::default().fg(Color::White)),
            Span::styled(
                mode_indicator,
                Style::default()
                    .fg(if is_manual {
                        Color::Cyan
                    } else {
                        Color::Yellow
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(Color::White)),
            Span::raw(format!("({}, {})", position.x, position.y)),
        ]),
        Line::from(vec![
            Span::styled("Terrain: ", Style::default().fg(Color::White)),
            Span::raw(terrain),
        ]),
        Line::from(vec![
            Span::styled("Passable: ", Style::default().fg(Color::White)),
            Span::raw(passable),
        ]),
        Line::from(vec![
            Span::styled("Occupied: ", Style::default().fg(Color::White)),
            Span::raw(occupied),
        ]),
    ];

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Tile Info"));

    frame.render_widget(paragraph, area);
}

/// Render entity details for the highlighted entity.
///
/// Shows information about the currently highlighted entity (NPC, prop, or item).
/// If no entity is highlighted, shows a placeholder message.
fn render_entity_details<T: PresentationMapper<Style = Style>>(
    frame: &mut Frame,
    area: Rect,
    highlighted_entity: Option<game_core::EntityId>,
    view_model: &ViewModel,
    theme: &T,
) {
    let Some(entity_id) = highlighted_entity else {
        // No entity highlighted
        let paragraph = Paragraph::new(vec![Line::from("No entity selected")])
            .block(Block::default().borders(Borders::ALL).title("Entity"));
        frame.render_widget(paragraph, area);
        return;
    };

    // Find the highlighted entity (prioritize actors, then props, then items)
    if let Some(actor) = view_model.actors.iter().find(|a| a.id == entity_id) {
        let entity_type = if actor.is_player { "Player" } else { "NPC" };
        let lines = render_actor_details(entity_type, actor, theme);

        let paragraph =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(entity_type));
        frame.render_widget(paragraph, area);
        return;
    }

    if let Some(prop) = view_model.props.iter().find(|p| p.id == entity_id) {
        let lines = render_prop_details(prop);

        let paragraph =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Prop"));
        frame.render_widget(paragraph, area);
        return;
    }

    if let Some(item) = view_model.items.iter().find(|i| i.id == entity_id) {
        let lines = render_item_details(item);

        let paragraph =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Item"));
        frame.render_widget(paragraph, area);
        return;
    }

    // Entity ID not found in view model
    let paragraph = Paragraph::new(vec![Line::from(format!(
        "Entity {:?} not found",
        entity_id
    ))])
    .block(Block::default().borders(Borders::ALL).title("Entity"));
    frame.render_widget(paragraph, area);
}

/// Render actor (NPC) details.
fn render_actor_details<'a, T: PresentationMapper<Style = Style>>(
    type_name: &'a str,
    actor: &'a ActorView,
    theme: &T,
) -> Vec<Line<'a>> {
    let (hp_cur, hp_max) = actor.stats.hp();
    let (mp_cur, mp_max) = actor.stats.mp();

    vec![
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::White)),
            Span::raw(type_name),
        ]),
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::White)),
            Span::raw(format!("{:?}", actor.id)),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(Color::White)),
            Span::raw(match actor.position {
                Some(pos) => format!("({}, {})", pos.x, pos.y),
                None => "Not on map".to_string(),
            }),
        ]),
        Line::from(vec![
            Span::styled("HP: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{}/{}", hp_cur, hp_max),
                theme.style_health(hp_cur, hp_max),
            ),
        ]),
        Line::from(vec![
            Span::styled("MP: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{}/{}", mp_cur, mp_max),
                theme.style_energy(mp_cur, mp_max),
            ),
        ]),
        Line::from(vec![
            Span::styled("Speed: ", Style::default().fg(Color::White)),
            Span::raw(actor.stats.speed.physical.to_string()),
        ]),
    ]
}

/// Render prop details.
fn render_prop_details<'a>(prop: &'a client_core::view_model::entities::PropView) -> Vec<Line<'a>> {
    vec![
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::White)),
            Span::raw("Prop"),
        ]),
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::White)),
            Span::raw(format!("{:?}", prop.id)),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(Color::White)),
            Span::raw(format!("({}, {})", prop.position.x, prop.position.y)),
        ]),
        Line::from(vec![
            Span::styled("Kind: ", Style::default().fg(Color::White)),
            Span::raw(format!("{:?}", prop.kind)),
        ]),
        Line::from(vec![
            Span::styled("Active: ", Style::default().fg(Color::White)),
            Span::raw(if prop.is_active { "Yes" } else { "No" }),
        ]),
    ]
}

/// Render item details.
fn render_item_details<'a>(item: &'a client_core::view_model::entities::ItemView) -> Vec<Line<'a>> {
    vec![
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::White)),
            Span::raw("Item"),
        ]),
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::White)),
            Span::raw(format!("{:?}", item.id)),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(Color::White)),
            Span::raw(format!("({}, {})", item.position.x, item.position.y)),
        ]),
        Line::from(vec![
            Span::styled("Handle: ", Style::default().fg(Color::White)),
            Span::raw(format!("{}", item.handle.0)),
        ]),
    ]
}
