//! Input handling (keyboard and directional input).

use anyhow::Result;
use client_frontend_core::EventConsumer;
use crossterm::event::{self as term_event, Event as TermEvent, KeyEvent, KeyEventKind};
use game_core::{Action, EntityId, env::MapOracle};
use tokio::time::Duration;

use super::super::EventLoop;
use crate::{
    cursor::CursorMovement,
    input::KeyAction,
    presentation::terminal::Tui,
    state::{AppMode, TargetingInputMode},
};

impl<C> EventLoop<C>
where
    C: EventConsumer,
{
    /// Poll for keyboard input and handle UI interactions.
    pub(in crate::event) async fn handle_input_tick(&mut self, terminal: &mut Tui) -> Result<bool> {
        if !term_event::poll(Duration::from_millis(0))? {
            return Ok(false);
        }

        match term_event::read()? {
            TermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key_press(key, terminal).await
            }
            TermEvent::Resize(_, _) => {
                self.render(terminal)?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Handle key press and dispatch to appropriate handler.
    pub(in crate::event) async fn handle_key_press(
        &mut self,
        key: KeyEvent,
        terminal: &mut Tui,
    ) -> Result<bool> {
        match self.input.handle_key(key, &self.app_state.mode) {
            KeyAction::Quit => {
                self.consumer
                    .message_log_mut()
                    .push_text(format!("[{}] Quitting...", self.view_model.turn.clock));

                // Just render the quit message
                self.render(terminal)?;
                Ok(true)
            }
            KeyAction::Submit(action) => {
                if self.tx_action.send(action).await.is_err() {
                    tracing::error!("Action channel closed");
                    return Ok(true);
                }
                Ok(false)
            }
            KeyAction::ToggleExamine => {
                let cursor_pos = if let Some(entity_id) = self.app_state.highlighted_entity {
                    // Place cursor at highlighted entity's position
                    self.view_model
                        .actors
                        .iter()
                        .find(|a| a.id == entity_id)
                        .and_then(|a| a.position)
                        .or(self.view_model.player.position)
                        .unwrap_or_else(|| game_core::Position::new(0, 0))
                } else {
                    // No highlighted entity - default to player
                    self.view_model
                        .player
                        .position
                        .unwrap_or_else(|| game_core::Position::new(0, 0))
                };

                self.app_state.toggle_examine(cursor_pos);
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::ExitModal => {
                self.app_state.exit_to_normal();
                self.compute_auto_target();
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::MoveCursor(direction) => {
                // Check if in SelectDirection targeting mode
                if let AppMode::Targeting(targeting_state) = &mut self.app_state.mode
                    && let TargetingInputMode::Direction { selected } =
                        &mut targeting_state.input_mode
                {
                    // Update selected direction
                    *selected = Some(direction);
                    self.render(terminal)?;
                    return Ok(false);
                }

                // Normal cursor movement (ExamineManual mode or SelectPosition targeting)
                if let Some(cursor) = &mut self.app_state.manual_cursor {
                    let (dx, dy) = direction.to_delta();
                    let dimensions = self.oracles.map.dimensions();
                    cursor.move_by(dx, dy, dimensions.width, dimensions.height);

                    // Update highlighted entity to first entity at new cursor position
                    self.update_highlighted_at_cursor();
                    self.render(terminal)?;
                }
                Ok(false)
            }
            KeyAction::NextEntity => {
                if self.app_state.mode == AppMode::Normal {
                    // Normal mode: cycle through all NPCs
                    self.cycle_highlighted_entity(1);
                } else {
                    // Manual mode: cycle through entities at cursor position
                    self.cycle_entities_at_cursor(1);
                }
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::PrevEntity => {
                if self.app_state.mode == AppMode::Normal {
                    // Normal mode: cycle through all NPCs (backwards)
                    self.cycle_highlighted_entity(-1);
                } else {
                    // Manual mode: cycle through entities at cursor position (backwards)
                    self.cycle_entities_at_cursor(-1);
                }
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::DirectionalInput(direction) => {
                self.handle_directional_input(direction).await?;
                Ok(false)
            }
            KeyAction::UseSlot(slot) => {
                self.handle_use_slot(slot).await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::OpenAbilityMenu => {
                self.handle_open_ability_menu().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::SelectAbilityForSlot(ability_idx) => {
                self.handle_select_ability(ability_idx)?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::ConfirmTarget => {
                self.handle_confirm_target().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::PickupItem => {
                self.handle_pickup_item().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::SaveGame => {
                self.handle_save_game().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::OpenStartScreen => {
                self.handle_open_start_screen().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::OpenSaveMenu => {
                self.handle_open_save_menu().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::MenuUp => {
                self.handle_menu_up();
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::MenuDown => {
                self.handle_menu_down();
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::MenuConfirm => {
                self.handle_menu_confirm().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::UploadToWalrus => {
                self.handle_upload_to_walrus().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::SubmitProof => {
                self.handle_submit_proof().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::CreateSession => {
                self.handle_create_session().await?;
                self.render(terminal)?;
                Ok(false)
            }
            KeyAction::None => Ok(false),
        }
    }

    /// Handle directional input: Bump-to-attack or Move.
    pub(in crate::event) async fn handle_directional_input(
        &mut self,
        direction: game_core::CardinalDirection,
    ) -> Result<()> {
        use game_core::{ActionInput, ActionKind, CharacterAction};

        let Some(player_pos) = self.view_model.player.position else {
            return Ok(());
        };
        let (dx, dy) = direction.offset();
        let target_pos = game_core::Position::new(player_pos.x + dx, player_pos.y + dy);

        // Check if there's an enemy at target position
        let enemy_at_target = self.view_model.actors.iter().find(|actor| {
            actor.id != EntityId::PLAYER
                && actor.position == Some(target_pos)
                && actor.stats.resource_current.hp > 0
        });

        let action = if let Some(enemy) = enemy_at_target {
            // Bump-to-attack: Attack the enemy
            CharacterAction::new(
                EntityId::PLAYER,
                ActionKind::MeleeAttack,
                ActionInput::Target(enemy.id),
            )
        } else {
            // No enemy: Just move
            CharacterAction::new(
                EntityId::PLAYER,
                ActionKind::Move,
                ActionInput::Direction(direction),
            )
        };

        self.tx_action.send(Action::Character(action)).await?;
        Ok(())
    }

    /// Handle picking up an item at the player's position.
    pub(in crate::event) async fn handle_pickup_item(&mut self) -> Result<()> {
        use game_core::{ActionInput, ActionKind, CharacterAction};

        let Some(player_pos) = self.view_model.player.position else {
            return Ok(());
        };

        // Find item at player's position
        let item_at_pos = self
            .view_model
            .items
            .iter()
            .find(|item| item.position == player_pos);

        if let Some(item) = item_at_pos {
            // Create PickupItem action with Target input (item entity ID)
            let action = CharacterAction::new(
                EntityId::PLAYER,
                ActionKind::PickupItem,
                ActionInput::Target(item.id),
            );

            self.tx_action.send(Action::Character(action)).await?;
        } else {
            // No item at player's position - optionally show a message
            self.consumer
                .message_log_mut()
                .push_text("No item here to pick up.".to_string());
        }

        Ok(())
    }

    /// Handle save game (Ctrl+S) - create manual checkpoint.
    pub(in crate::event) async fn handle_save_game(&mut self) -> Result<()> {
        self.consumer
            .message_log_mut()
            .push_text("Saving game...".to_string());

        match self.runtime_handle.create_checkpoint().await {
            Ok(nonce) => {
                self.consumer
                    .message_log_mut()
                    .push_text(format!("Game saved at nonce {} (Ctrl+O to load)", nonce));
            }
            Err(e) => {
                self.consumer
                    .message_log_mut()
                    .push_text(format!("Failed to save: {}", e));
            }
        }

        Ok(())
    }

    /// Handle open save menu (Ctrl+O) - enter full-screen save menu mode.
    pub(in crate::event) async fn handle_open_start_screen(&mut self) -> Result<()> {
        use client_bootstrap::list_sessions;

        // Query runtime for save data directory
        // For now, use environment variable or default fallback
        let save_dir = std::env::var("SAVE_DATA_DIR")
            .ok()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("./save_data"));

        // List all available sessions
        match list_sessions(&save_dir) {
            Ok(sessions) => {
                // Enter start screen mode
                self.app_state.enter_start_screen(sessions);
            }
            Err(e) => {
                self.consumer
                    .message_log_mut()
                    .push_text(format!("Failed to list sessions: {}", e));
            }
        }

        Ok(())
    }

    pub(in crate::event) async fn handle_open_save_menu(&mut self) -> Result<()> {
        use runtime::ActionBatchStatus;

        // List all checkpoints and filter out InProgress
        match self.runtime_handle.list_all_checkpoints().await {
            Ok(all_batches) => {
                // Filter out InProgress batches (end_nonce not finalized yet)
                let batches: Vec<_> = all_batches
                    .into_iter()
                    .filter(|b| !matches!(b.status, ActionBatchStatus::InProgress))
                    .collect();

                // Fetch blockchain session info (if available)
                #[cfg(feature = "sui")]
                let session_info = match self.runtime_handle.get_blockchain_session_info().await {
                    Ok(info) => info,
                    Err(e) => {
                        self.consumer
                            .message_log_mut()
                            .push_text(format!("Failed to fetch session info: {}", e));
                        None
                    }
                };

                // Enter save menu mode with finalized checkpoints
                self.app_state.enter_save_menu(
                    batches,
                    #[cfg(feature = "sui")]
                    session_info,
                );
            }
            Err(e) => {
                self.consumer
                    .message_log_mut()
                    .push_text(format!("Failed to list checkpoints: {}", e));
            }
        }

        Ok(())
    }

    /// Handle menu navigation up (StartScreen, SaveMenu).
    pub(in crate::event) fn handle_menu_up(&mut self) {
        use crate::state::AppMode;

        match &mut self.app_state.mode {
            AppMode::StartScreen(start_state) => {
                if start_state.selected > 0 {
                    start_state.selected -= 1;
                }
            }
            AppMode::SaveMenu(menu_state) => {
                if menu_state.selected_index > 0 {
                    menu_state.selected_index -= 1;
                }
            }
            _ => {}
        }
    }

    /// Handle menu navigation down (StartScreen, SaveMenu).
    pub(in crate::event) fn handle_menu_down(&mut self) {
        use crate::state::AppMode;

        match &mut self.app_state.mode {
            AppMode::StartScreen(start_state) => {
                let max_index = start_state.sessions.len(); // 0 = New Game, 1+ = sessions
                if start_state.selected < max_index {
                    start_state.selected += 1;
                }
            }
            AppMode::SaveMenu(menu_state) => {
                if menu_state.selected_index < menu_state.saved_states.len().saturating_sub(1) {
                    menu_state.selected_index += 1;
                }
            }
            _ => {}
        }
    }

    /// Handle menu confirm (StartScreen session selection, SaveMenu load state).
    pub(in crate::event) async fn handle_menu_confirm(&mut self) -> Result<()> {
        use crate::state::AppMode;

        if let AppMode::StartScreen(start_state) = &self.app_state.mode {
            if start_state.selected == 0 {
                // New Game selected
                self.consumer.message_log_mut().push_text(
                    "New Game not yet implemented. Please restart the client to start a new game."
                        .to_string(),
                );
                self.app_state.exit_to_normal();
            } else {
                // Continue from selected session
                let session_idx = start_state.selected - 1;
                if let Some(session) = start_state.sessions.get(session_idx) {
                    self.consumer.message_log_mut().push_text(format!(
                        "Session resumption not yet implemented. Please restart the client to continue session: {}",
                        session.session_id
                    ));
                    self.app_state.exit_to_normal();
                }
            }
            return Ok(());
        }

        if let AppMode::SaveMenu(menu_state) = &self.app_state.mode {
            if menu_state.saved_states.is_empty() {
                // No saved states - just exit
                self.app_state.exit_to_normal();
                return Ok(());
            }

            // Get selected state
            let selected_state = &menu_state.saved_states[menu_state.selected_index];
            let target_nonce = selected_state.nonce;

            self.consumer
                .message_log_mut()
                .push_text(format!("Restoring state at nonce {}...", target_nonce));

            // Use restore_state to actually replace the simulation worker's state
            match self.runtime_handle.restore_state(target_nonce).await {
                Ok(()) => {
                    self.consumer.message_log_mut().push_text(format!(
                        "Successfully restored state at nonce {}",
                        target_nonce
                    ));

                    // Prepare next turn to refresh the ViewModel
                    match self.runtime_handle.prepare_next_turn().await {
                        Ok((_, restored_state)) => {
                            // Update ViewModel with restored state
                            self.view_model =
                                client_frontend_core::view_model::ViewModel::from_initial_state(
                                    &restored_state,
                                    self.oracles.map.as_ref(),
                                );

                            self.consumer.message_log_mut().push_text(format!(
                                "State restored (turn {})",
                                restored_state.turn.clock
                            ));

                            // Recompute auto-target for new state
                            self.compute_auto_target();
                        }
                        Err(e) => {
                            self.consumer
                                .message_log_mut()
                                .push_text(format!("Failed to refresh view: {}", e));
                        }
                    }

                    // Exit back to normal mode
                    self.app_state.exit_to_normal();
                }
                Err(e) => {
                    self.consumer
                        .message_log_mut()
                        .push_text(format!("Failed to restore state: {}", e));

                    // Stay in menu on error
                }
            }
        }

        Ok(())
    }

    /// Handle upload to Walrus (W key in SaveMenu).
    pub(in crate::event) async fn handle_upload_to_walrus(&mut self) -> Result<()> {
        #[cfg(not(feature = "sui"))]
        {
            Ok(())
        }

        #[cfg(feature = "sui")]
        {
            use crate::state::AppMode;
            use runtime::ActionBatchStatus;

            if let AppMode::SaveMenu(menu_state) = &self.app_state.mode {
                if menu_state.saved_states.is_empty() {
                    return Ok(());
                }

                // Get selected state and its associated batch
                let selected_state = &menu_state.saved_states[menu_state.selected_index];

                if let Some(batch_idx) = selected_state.batch_index {
                    if let Some(batch) = menu_state.action_batches.get(batch_idx) {
                        // Check if upload is available (must be Proven)
                        let can_upload = matches!(batch.status, ActionBatchStatus::Proven { .. });

                        if !can_upload {
                            self.app_state.save_menu_log.push_text(format!(
                                "Cannot upload to Walrus for batch at nonce {} (status: {:?})",
                                batch.start_nonce, batch.status
                            ));
                            return Ok(());
                        }

                        // Upload to Walrus
                        self.app_state.save_menu_log.push_text(format!(
                            "Uploading action log for batch {} → {} to Walrus...",
                            batch.start_nonce, batch.end_nonce
                        ));

                        #[cfg(feature = "sui")]
                        {
                            match self
                                .runtime_handle
                                .upload_to_walrus(batch.start_nonce)
                                .await
                            {
                                Ok((blob_object_id, walrus_blob_id)) => {
                                    self.app_state.save_menu_log.push_text(format!(
                                    "✓ Uploaded to Walrus successfully!\nBlob Object: {}\nWalrus ID: {}",
                                    blob_object_id,
                                    walrus_blob_id
                                ));

                                    // Refresh menu state to show updated batch status
                                    if let Err(e) = self.refresh_save_menu().await {
                                        tracing::warn!(
                                            "Failed to refresh save menu after upload: {}",
                                            e
                                        );
                                    }
                                }
                                Err(e) => {
                                    self.app_state
                                        .save_menu_log
                                        .push_text(format!("✗ Failed to upload to Walrus: {}", e));
                                }
                            }
                        }
                    } else {
                        self.app_state
                            .save_menu_log
                            .push_text("No associated proof batch found".to_string());
                    }
                } else {
                    self.app_state
                        .save_menu_log
                        .push_text("No associated proof batch for this state".to_string());
                }
            }

            Ok(())
        }
    }

    /// Handle submit proof (S key in SaveMenu).
    pub(in crate::event) async fn handle_submit_proof(&mut self) -> Result<()> {
        #[cfg(not(feature = "sui"))]
        {
            Ok(())
        }

        #[cfg(feature = "sui")]
        {
            use crate::state::AppMode;
            use runtime::ActionBatchStatus;

            if let AppMode::SaveMenu(menu_state) = &self.app_state.mode {
                if menu_state.saved_states.is_empty() {
                    return Ok(());
                }

                // Get selected state and its associated batch
                let selected_state = &menu_state.saved_states[menu_state.selected_index];

                if let Some(batch_idx) = selected_state.batch_index {
                    if let Some(batch) = menu_state.action_batches.get(batch_idx) {
                        // Check if submission is available (must be BlobUploaded)
                        let can_submit =
                            matches!(batch.status, ActionBatchStatus::BlobUploaded { .. });

                        if !can_submit {
                            self.app_state.save_menu_log.push_text(format!(
                                "Cannot submit proof for batch at nonce {} (status: {:?})",
                                batch.start_nonce, batch.status
                            ));
                            return Ok(());
                        }

                        // Submit to blockchain
                        self.app_state.save_menu_log.push_text(format!(
                            "Submitting proof for batch {} → {} to blockchain...",
                            batch.start_nonce, batch.end_nonce
                        ));

                        #[cfg(feature = "sui")]
                        {
                            match self
                                .runtime_handle
                                .submit_to_blockchain(batch.start_nonce)
                                .await
                            {
                                Ok(tx_digest) => {
                                    self.app_state.save_menu_log.push_text(format!(
                                        "✓ Submitted to blockchain successfully!\nTx Digest: {}",
                                        tx_digest
                                    ));

                                    // Refresh menu state to show updated batch status
                                    if let Err(e) = self.refresh_save_menu().await {
                                        tracing::warn!(
                                            "Failed to refresh save menu after submit: {}",
                                            e
                                        );
                                    }
                                }
                                Err(e) => {
                                    self.app_state.save_menu_log.push_text(format!(
                                        "✗ Failed to submit to blockchain: {}",
                                        e
                                    ));
                                }
                            }
                        }
                    } else {
                        self.app_state
                            .save_menu_log
                            .push_text("No associated proof batch found".to_string());
                    }
                } else {
                    self.app_state
                        .save_menu_log
                        .push_text("No associated proof batch for this state".to_string());
                }
            }

            Ok(())
        }
    }

    /// Handle create session on blockchain (C key in SaveMenu).
    pub(in crate::event) async fn handle_create_session(&mut self) -> Result<()> {
        #[cfg(not(feature = "sui"))]
        {
            Ok(())
        }

        #[cfg(feature = "sui")]
        {
            use crate::state::AppMode;

            if !matches!(self.app_state.mode, AppMode::SaveMenu(_)) {
                return Ok(());
            }

            self.app_state
                .save_menu_log
                .push_text("Creating session on blockchain using state 0...".to_string());

            match self.runtime_handle.create_session_on_blockchain().await {
                Ok(session_object_id) => {
                    self.app_state.save_menu_log.push_text(format!(
                        "✓ Session created successfully!\nSession Object ID: {}",
                        session_object_id
                    ));

                    // Refresh menu state to show updated session info
                    if let Err(e) = self.refresh_save_menu().await {
                        tracing::warn!("Failed to refresh save menu after session creation: {}", e);
                    }
                }
                Err(e) => {
                    self.app_state
                        .save_menu_log
                        .push_text(format!("✗ Failed to create session: {}", e));
                }
            }

            Ok(())
        }
    }

    /// Refresh Save Menu state with latest batch data from runtime.
    ///
    /// Used after blockchain operations (upload/submit) to show updated status immediately.
    pub(in crate::event) async fn refresh_save_menu(&mut self) -> Result<()> {
        use crate::state::AppMode;
        use runtime::ActionBatchStatus;

        // Only refresh if we're in Save Menu mode
        if let AppMode::SaveMenu(current_state) = &self.app_state.mode {
            let selected_index = current_state.selected_index;

            match self.runtime_handle.list_all_checkpoints().await {
                Ok(all_batches) => {
                    // Filter out InProgress batches
                    let batches: Vec<_> = all_batches
                        .into_iter()
                        .filter(|b| !matches!(b.status, ActionBatchStatus::InProgress))
                        .collect();

                    // Fetch blockchain session info (if available)
                    #[cfg(feature = "sui")]
                    let session_info = match self.runtime_handle.get_blockchain_session_info().await
                    {
                        Ok(info) => info,
                        Err(e) => {
                            self.app_state
                                .save_menu_log
                                .push_text(format!("Failed to fetch session info: {}", e));
                            None
                        }
                    };

                    // Re-enter save menu with updated batches, preserving selection
                    self.app_state.enter_save_menu(
                        batches,
                        #[cfg(feature = "sui")]
                        session_info,
                    );

                    // Restore selected index (capped at new list length)
                    if let AppMode::SaveMenu(new_state) = &mut self.app_state.mode {
                        new_state.selected_index =
                            selected_index.min(new_state.saved_states.len().saturating_sub(1));
                    }
                }
                Err(e) => {
                    self.app_state
                        .save_menu_log
                        .push_text(format!("Failed to refresh menu: {}", e));
                }
            }
        }

        Ok(())
    }
}
