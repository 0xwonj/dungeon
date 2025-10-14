//! UI rendering with Ratatui built on top of a view-model layer.
//!
//! This module contains all presentation logic for the CLI client, including:
//! - Frame layout and panel composition
//! - Entity and tile rendering with color coding
//! - Examine panel with auto-target and manual cursor support
//! - Message log formatting
use anyhow::Result;
use game_core::{GameState, Position, PropKind, TerrainKind, env::MapOracle};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListDirection, ListItem, Paragraph},
};

use crate::{
    presentation::terminal::Tui,
    state::{AppMode, AppState},
};
use frontend_core::{
    message::{MessageEntry, MessageLevel, MessageLog},
    view_model::{
        ActorStatsSnapshot, EntityDetailView, MapSnapshot, MapTile, OccupantView, PlayerSnapshot,
        ResourceSnapshot, TileInfoSnapshot, TurnSummary, UiFrame, WorldSnapshot,
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

pub fn render(
    terminal: &mut Tui,
    frame: &UiFrame,
    app_state: &AppState,
    game_state: &GameState,
    map: &dyn MapOracle,
) -> Result<()> {
    terminal.draw(|ctx| render_frame(ctx, frame, app_state, game_state, map))?;
    Ok(())
}

fn render_frame(
    frame: &mut Frame,
    view: &UiFrame,
    app_state: &AppState,
    game_state: &GameState,
    map: &dyn MapOracle,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(MESSAGE_PANEL_HEIGHT),
            Constraint::Length(2),
        ])
        .split(frame.area());

    render_header(frame, chunks[0], &view.turn, app_state);
    render_game(
        frame,
        chunks[1],
        GameRenderContext {
            map: &view.map,
            player: &view.player,
            world: &view.world,
            app_state,
            game_state,
            map_oracle: map,
        },
    );
    render_messages(frame, chunks[2], &view.messages);
    render_footer(frame, chunks[3], app_state);
}

/// Context for rendering the game panel.
struct GameRenderContext<'a> {
    map: &'a MapSnapshot,
    player: &'a PlayerSnapshot,
    world: &'a WorldSnapshot,
    app_state: &'a AppState,
    game_state: &'a GameState,
    map_oracle: &'a dyn MapOracle,
}

fn render_game(frame: &mut Frame, area: ratatui::layout::Rect, ctx: GameRenderContext) {
    // Always show Examine panel with 6:2:2 layout
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(area);

    render_map(frame, chunks[0], ctx.map, ctx.app_state);
    render_player_stats(frame, chunks[1], ctx.player, ctx.world);

    // Always render Examine panel (auto-target or manual cursor)
    if let Some(examine_pos) = ctx.app_state.examine_position() {
        render_examine_panel(
            frame,
            chunks[2],
            examine_pos,
            ctx.app_state.entity_index,
            ctx.app_state.is_manual_cursor(),
            ctx.game_state,
            ctx.map_oracle,
        );
    }
}

fn render_header(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    summary: &TurnSummary,
    app_state: &AppState,
) {
    let mode_text = match app_state.mode {
        AppMode::Normal => "",
        AppMode::ExamineManual => " [EXAMINE - MANUAL]",
        AppMode::Targeting { .. } => " [TARGETING]",
        AppMode::Inventory => " [INVENTORY]",
    };

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
        Span::styled(
            mode_text,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Game"));

    frame.render_widget(paragraph, area);
}

fn render_map(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    map: &MapSnapshot,
    app_state: &AppState,
) {
    let cursor_pos = app_state.examine_position();

    let mut rows = Vec::with_capacity(map.tiles.len());
    for row in &map.tiles {
        let spans = row
            .iter()
            .map(|tile| {
                let (glyph, mut style) = glyph_for_tile(tile);

                // Highlight cursor position
                if cursor_pos.is_some_and(|cursor| tile.position == cursor) {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }

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
        Span::styled("Speed (Phys): ", Style::default().fg(Color::White)),
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

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, app_state: &AppState) {
    let text = match app_state.mode {
        AppMode::Normal => vec![Line::from(vec![
            Span::raw("[hjkl/WASD/Arrows] Move | "),
            Span::raw("[Space/Enter/.] Wait | "),
            Span::raw("[x] Manual examine | "),
            Span::raw("[Tab] Cycle | "),
            Span::raw("[q] Quit"),
        ])],
        AppMode::ExamineManual => vec![Line::from(vec![
            Span::raw("[hjkl/Arrows] Move cursor | "),
            Span::raw("[Tab] Next entity | "),
            Span::raw("[Shift+Tab] Prev | "),
            Span::raw("[x/ESC] Back"),
        ])],
        _ => vec![Line::from(vec![Span::raw("[ESC] Exit mode")])],
    };

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

/// Formats a resource (health/energy) as a colored span.
fn render_resource<'a>(resource: &ResourceSnapshot, color: Color) -> Span<'a> {
    Span::styled(
        format!("{}/{}", resource.current, resource.maximum),
        Style::default().fg(color),
    )
}

/// Formats a resource as a colored string for entity details.
fn format_resource_str(resource: &ResourceSnapshot) -> String {
    format!("{}/{}", resource.current, resource.maximum)
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

/// Renders the Examine panel showing detailed tile and entity information.
fn render_examine_panel(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    cursor_position: Position,
    entity_index: usize,
    is_manual: bool,
    game_state: &GameState,
    map_oracle: &dyn MapOracle,
) {
    let tile_info = TileInfoSnapshot::from_state(map_oracle, game_state, cursor_position);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    render_tile_info(frame, chunks[0], &tile_info, is_manual);
    render_entity_details(frame, chunks[1], &tile_info, entity_index);
}

/// Renders tile-level information (terrain, passability, overlays).
fn render_tile_info(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    tile_info: &TileInfoSnapshot,
    is_manual: bool,
) {
    let terrain_name = format!("{:?}", tile_info.terrain);
    let passable = if tile_info.is_passable { "Yes" } else { "No" };
    let occupied = if tile_info.is_occupied { "Yes" } else { "No" };
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
            Span::raw(format!(
                "({}, {})",
                tile_info.position.x, tile_info.position.y
            )),
        ]),
        Line::from(vec![
            Span::styled("Terrain: ", Style::default().fg(Color::White)),
            Span::raw(terrain_name),
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
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Examine"));

    frame.render_widget(paragraph, area);
}

/// Renders entity details with cycling support.
fn render_entity_details(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    tile_info: &TileInfoSnapshot,
    entity_index: usize,
) {
    if tile_info.entities.is_empty() {
        let paragraph = Paragraph::new(vec![Line::from("No entities here")])
            .block(Block::default().borders(Borders::ALL).title("Entities"));
        frame.render_widget(paragraph, area);
        return;
    }

    let index = entity_index % tile_info.entities.len();
    let entity = &tile_info.entities[index];

    let title = format!("Entities ({}/{})", index + 1, tile_info.entities.len());
    let lines = match entity {
        EntityDetailView::Player { id, stats } => render_actor_details("Player", *id, stats),
        EntityDetailView::Npc {
            id,
            template_id,
            stats,
        } => {
            let mut lines = render_actor_details("NPC", *id, stats);
            lines.insert(
                1,
                Line::from(vec![
                    Span::styled("Template: ", Style::default().fg(Color::White)),
                    Span::raw(format!("#{}", template_id)),
                ]),
            );
            lines
        }
        EntityDetailView::Prop {
            id,
            kind,
            is_active,
        } => {
            vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(Color::White)),
                    Span::raw("Prop"),
                ]),
                Line::from(vec![
                    Span::styled("ID: ", Style::default().fg(Color::White)),
                    Span::raw(format!("{}", id)),
                ]),
                Line::from(vec![
                    Span::styled("Kind: ", Style::default().fg(Color::White)),
                    Span::raw(format!("{:?}", kind)),
                ]),
                Line::from(vec![
                    Span::styled("Active: ", Style::default().fg(Color::White)),
                    Span::raw(if *is_active { "Yes" } else { "No" }),
                ]),
            ]
        }
        EntityDetailView::Item { id, handle } => {
            vec![
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(Color::White)),
                    Span::raw("Item"),
                ]),
                Line::from(vec![
                    Span::styled("ID: ", Style::default().fg(Color::White)),
                    Span::raw(format!("{}", id)),
                ]),
                Line::from(vec![
                    Span::styled("Handle: ", Style::default().fg(Color::White)),
                    Span::raw(format!("{}", handle.0)),
                ]),
            ]
        }
    };

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(paragraph, area);
}

/// Helper to render actor (Player/NPC) stat lines.
fn render_actor_details<'a>(
    type_name: &'a str,
    id: game_core::EntityId,
    stats: &'a ActorStatsSnapshot,
) -> Vec<Line<'a>> {
    vec![
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::White)),
            Span::raw(type_name),
        ]),
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::White)),
            Span::raw(format!("{}", id)),
        ]),
        Line::from(vec![
            Span::styled("Health: ", Style::default().fg(Color::White)),
            Span::styled(
                format_resource_str(&stats.health),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::styled("Energy: ", Style::default().fg(Color::White)),
            Span::styled(
                format_resource_str(&stats.energy),
                Style::default().fg(Color::Blue),
            ),
        ]),
        Line::from(vec![
            Span::styled("Speed: ", Style::default().fg(Color::White)),
            Span::raw(stats.speed.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Ready at: ", Style::default().fg(Color::White)),
            Span::raw(
                stats
                    .ready_at
                    .map_or("Inactive".to_string(), |t| t.to_string()),
            ),
        ]),
    ]
}
