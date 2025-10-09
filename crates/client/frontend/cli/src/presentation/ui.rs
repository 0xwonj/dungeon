//! UI rendering with ratatui built on top of a view-model layer.
use anyhow::Result;
use game_core::{GameState, PropKind, TerrainKind, env::MapOracle};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListDirection, ListItem, Paragraph},
};

use crate::presentation::terminal::Tui;
use frontend_core::{
    message::{MessageEntry, MessageLevel, MessageLog},
    view_model::{
        MapSnapshot, MapTile, OccupantView, PlayerSnapshot, ResourceSnapshot, TurnSummary, UiFrame,
        WorldSnapshot,
    },
};

pub const MESSAGE_PANEL_HEIGHT: u16 = 5;

pub fn build_frame<M: MapOracle + ?Sized>(
    map: &M,
    state: &GameState,
    messages: &MessageLog,
) -> UiFrame {
    UiFrame::from_state(map, state, messages, MESSAGE_PANEL_HEIGHT as usize)
}

pub fn render(terminal: &mut Tui, frame: &UiFrame) -> Result<()> {
    terminal.draw(|ctx| render_frame(ctx, frame))?;
    Ok(())
}

fn render_frame(frame: &mut Frame, view: &UiFrame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(MESSAGE_PANEL_HEIGHT),
            Constraint::Length(2),
        ])
        .split(frame.area());

    render_header(frame, chunks[0], &view.turn);
    render_game(frame, chunks[1], &view.map, &view.player, &view.world);
    render_messages(frame, chunks[2], &view.messages);
    render_footer(frame, chunks[3]);
}

fn render_game(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    map: &MapSnapshot,
    player: &PlayerSnapshot,
    world: &WorldSnapshot,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    render_map(frame, chunks[0], map);
    render_player_stats(frame, chunks[1], player, world);
}

fn render_header(frame: &mut Frame, area: ratatui::layout::Rect, summary: &TurnSummary) {
    let text = vec![Line::from(vec![
        Span::raw("Time: "),
        Span::styled(
            summary.clock.to_string(),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | Actor: "),
        Span::styled(
            format!("{:?}", summary.current_actor),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | Active: "),
        Span::styled(
            summary.active_actors.len().to_string(),
            Style::default().fg(Color::LightGreen),
        ),
    ])];

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Game"));

    frame.render_widget(paragraph, area);
}

fn render_map(frame: &mut Frame, area: ratatui::layout::Rect, map: &MapSnapshot) {
    let mut rows = Vec::with_capacity(map.tiles.len());
    for row in &map.tiles {
        let spans = row
            .iter()
            .map(|tile| {
                let (glyph, style) = glyph_for_tile(tile);
                Span::styled(glyph, style)
            })
            .collect::<Vec<_>>();
        rows.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(rows).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Map ({}×{})", map.width, map.height)),
    );

    frame.render_widget(paragraph, area);
}

fn render_player_stats(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    player: &PlayerSnapshot,
    world: &WorldSnapshot,
) {
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Health: ", Style::default().fg(Color::White)),
        render_resource(&player.stats.health, Color::Red),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Energy: ", Style::default().fg(Color::White)),
        render_resource(&player.stats.energy, Color::Blue),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Speed: ", Style::default().fg(Color::White)),
        Span::raw(player.stats.speed.to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Position: ", Style::default().fg(Color::White)),
        Span::raw(format!("({}, {})", player.position.x, player.position.y)),
    ]));

    match player.stats.ready_at {
        Some(tick) => {
            lines.push(Line::from(vec![
                Span::styled("Ready at: ", Style::default().fg(Color::White)),
                Span::styled(tick.to_string(), Style::default().fg(Color::Yellow)),
            ]));
        }
        None => {
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::White)),
                Span::styled("Inactive", Style::default().fg(Color::Gray)),
            ]));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("Inventory items: ", Style::default().fg(Color::White)),
        Span::raw(player.inventory_items.to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("NPCs tracked: ", Style::default().fg(Color::White)),
        Span::raw(world.npc_count.to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Props: ", Style::default().fg(Color::White)),
        Span::raw(world.prop_count.to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Loose items: ", Style::default().fg(Color::White)),
        Span::raw(world.loose_item_count.to_string()),
    ]));

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Player"));

    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect) {
    let text = vec![Line::from(vec![
        Span::raw("[hjkl / WASD / Arrows] Move | "),
        Span::raw("[Space][Enter][.] Wait | "),
        Span::raw("[q] Quit"),
    ])];

    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn render_messages(frame: &mut Frame, area: ratatui::layout::Rect, messages: &[MessageEntry]) {
    let mut items: Vec<ListItem> = messages
        .iter()
        .map(|entry| ListItem::new(format_message(entry)).style(style_for_level(entry.level)))
        .collect();

    while items.len() < MESSAGE_PANEL_HEIGHT as usize {
        items.push(ListItem::new(""));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .direction(ListDirection::BottomToTop);

    frame.render_widget(list, area);
}

fn render_resource(resource: &ResourceSnapshot, color: Color) -> Span<'static> {
    Span::styled(
        format!("{}/{}", resource.current, resource.maximum),
        Style::default().fg(color),
    )
}

fn glyph_for_tile(tile: &MapTile) -> (String, Style) {
    if let Some(glyph) = tile
        .occupants
        .iter()
        .map(|occupant| match occupant {
            OccupantView::Player { is_current, .. } => {
                ("@".to_string(), actor_style(*is_current, Color::Yellow))
            }
            OccupantView::Npc { is_current, .. } => (
                if *is_current { "N" } else { "n" }.to_string(),
                actor_style(*is_current, Color::LightRed),
            ),
            OccupantView::Prop {
                kind, is_active, ..
            } => prop_visual(kind, *is_active),
        })
        .next()
    {
        return glyph;
    }

    if tile.loose_items > 0 {
        return ("*".to_string(), Style::default().fg(Color::LightCyan));
    }

    if tile.overlays > 0 {
        return ("!".to_string(), Style::default().fg(Color::Magenta));
    }

    let (glyph, color) = match tile.terrain {
        TerrainKind::Floor => ('.', Color::DarkGray),
        TerrainKind::Wall => ('#', Color::Gray),
        TerrainKind::Void => (' ', Color::Reset),
        TerrainKind::Water => ('~', Color::Blue),
        TerrainKind::Custom(_) => ('?', Color::LightMagenta),
    };

    (glyph.to_string(), Style::default().fg(color))
}

fn actor_style(is_current: bool, color: Color) -> Style {
    let mut style = Style::default().fg(color);
    if is_current {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

fn prop_visual(kind: &PropKind, is_active: bool) -> (String, Style) {
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

fn format_message(entry: &MessageEntry) -> String {
    match entry.timestamp {
        Some(ts) => format!("[{}] {}", ts, entry.text),
        None => entry.text.clone(),
    }
}

fn style_for_level(level: MessageLevel) -> Style {
    match level {
        MessageLevel::Info => Style::default().fg(Color::White),
        MessageLevel::Warning => Style::default().fg(Color::Yellow),
        MessageLevel::Error => Style::default().fg(Color::LightRed),
    }
}
