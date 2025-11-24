//! Player stats widget displaying health, energy, and other player information.

use client_frontend_core::view_model::{PresentationMapper, ViewModel};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Render the player stats panel.
///
/// **NEW ARCHITECTURE**: This widget uses PresentationMapper for styling,
/// demonstrating framework-independent theming!
pub fn render<T: PresentationMapper<Style = Style>>(
    frame: &mut Frame,
    area: Rect,
    view_model: &ViewModel,
    theme: &T,
) {
    let player = &view_model.player;
    let world = &view_model.world;

    let (hp_cur, hp_max) = player.stats.hp();
    let (mp_cur, mp_max) = player.stats.mp();

    // ✅ PresentationMapper for health/energy styling
    let hp_style = theme.style_health(hp_cur, hp_max);
    let mp_style = theme.style_energy(mp_cur, mp_max);

    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Health: ", Style::default().fg(Color::White)),
        Span::styled(format!("{}/{}", hp_cur, hp_max), hp_style),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Energy: ", Style::default().fg(Color::White)),
        Span::styled(format!("{}/{}", mp_cur, mp_max), mp_style),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Speed (Phys): ", Style::default().fg(Color::White)),
        Span::raw(player.stats.speed.physical.to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Position: ", Style::default().fg(Color::White)),
        Span::raw(match player.position {
            Some(pos) => format!("({}, {})", pos.x, pos.y),
            None => "Not on map".to_string(),
        }),
    ]));

    // World statistics
    lines.push(Line::from(vec![
        Span::styled("NPCs: ", Style::default().fg(Color::White)),
        Span::raw(world.npc_count.to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Props: ", Style::default().fg(Color::White)),
        Span::raw(world.prop_count.to_string()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Items: ", Style::default().fg(Color::White)),
        Span::raw(world.loose_item_count.to_string()),
    ]));

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Player"));

    frame.render_widget(paragraph, area);
}
