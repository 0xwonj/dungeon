//! Rendering handlers.

use anyhow::Result;
use client_frontend_core::EventConsumer;

use super::super::EventLoop;
use crate::presentation::{terminal::Tui, ui};

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    /// Render current state using ViewModel.
    pub(in crate::event) fn render(&mut self, terminal: &mut Tui) -> Result<()> {
        self.input.set_player_entity(self.view_model.player.id);

        // Extract available actions from ViewModel
        let available_actions: Vec<_> = self
            .view_model
            .player
            .actions
            .iter()
            .map(|ability| ability.kind)
            .collect();

        let ctx = ui::RenderContext {
            view_model: &self.view_model,
            messages: self.consumer.message_log(),
            app_state: &self.app_state,
            action_slots: &self.app_state.action_slots,
            available_actions: &available_actions,
            message_panel_height: self.cli_config.ui.message_panel_height,
            map: self.oracles.map.as_ref(),
        };

        ui::render_with_view_model(terminal, &ctx)
    }
}
