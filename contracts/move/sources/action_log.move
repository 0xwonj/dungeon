/// Action Log Module
///
/// Stores references to off-chain action data for replay and challenge verification.
/// This module is optional - players can choose to publish action logs for competitive
/// play and challenges, or keep them private for casual sessions.
///
/// # Design
/// - One ActionLog per GameSession (1:1 relationship)
/// - References Walrus blob containing full action sequence
/// - Actions root is cryptographically verified by GameSession's ZK proof
/// - Challenge system uses this data to verify AI behavior validity
module dungeon::action_log {
    use sui::event;
    use dungeon::game_session::{Self, GameSession};

    // ===== Error Codes =====

    /// Caller is not the session owner
    const ENotOwner: u64 = 1;
    /// Session is already finalized
    const EAlreadyFinalized: u64 = 2;
    /// Actions root mismatch between log and session
    const EActionsRootMismatch: u64 = 3;

    // ===== Structs =====

    /// Represents an action log for a game session
    ///
    /// Stores references to off-chain action data in Walrus storage.
    /// The actions_root field must match the GameSession's actions_root
    /// to ensure cryptographic binding between logged actions and verified state.
    public struct ActionLog has key, store {
        id: UID,
        /// Reference to the associated GameSession
        session_id: address,
        /// Player address (must match session owner)
        player: address,
        /// Walrus blob ID containing full action sequence (JSON or CBOR)
        actions_blob_id: vector<u8>,
        /// Actions root commitment (must match GameSession.actions_root)
        actions_root: vector<u8>,
        /// Whether the log is finalized
        finalized: bool,
    }

    // ===== Events =====

    /// Emitted when an action log is published
    public struct ActionLogPublishedEvent has copy, drop {
        log_id: address,
        session_id: address,
        player: address,
        actions_blob_id: vector<u8>,
        actions_root: vector<u8>,
        published_at: u64,
    }

    /// Emitted when an action log is updated with new actions
    public struct ActionLogUpdatedEvent has copy, drop {
        log_id: address,
        session_id: address,
        actions_blob_id: vector<u8>,
        actions_root: vector<u8>,
        updated_at: u64,
    }

    /// Emitted when an action log is finalized
    public struct ActionLogFinalizedEvent has copy, drop {
        log_id: address,
        session_id: address,
        finalized_at: u64,
    }

    // ===== Public Functions =====

    /// Publish an action log for a game session
    ///
    /// Creates a new ActionLog linked to an existing GameSession.
    /// The actions_root must match the session's actions_root to ensure
    /// the logged actions correspond to the ZK-verified state transitions.
    ///
    /// # Arguments
    /// * `session` - Reference to GameSession (for validation)
    /// * `actions_blob_id` - Walrus blob ID containing action data
    /// * `actions_root` - Actions root commitment (must match session)
    /// * `ctx` - Transaction context
    ///
    /// # Returns
    /// A new ActionLog object owned by the caller
    ///
    /// # Aborts
    /// * `ENotOwner` - If caller is not the session owner
    /// * `EAlreadyFinalized` - If session is already finalized
    /// * `EActionsRootMismatch` - If actions_root doesn't match session
    ///
    /// # Events
    /// Emits `ActionLogPublishedEvent` with log details
    public fun publish(
        session: &GameSession,
        actions_blob_id: vector<u8>,
        actions_root: vector<u8>,
        ctx: &mut TxContext,
    ): ActionLog {
        let player = tx_context::sender(ctx);
        let session_id = object::id_to_address(&object::id(session));

        // Validate ownership
        assert!(game_session::player(session) == player, ENotOwner);

        // Validate session not finalized
        assert!(!game_session::is_finalized(session), EAlreadyFinalized);

        // Validate actions root matches session
        assert!(
            game_session::borrow_actions_root(session) == &actions_root,
            EActionsRootMismatch
        );

        let log_id = object::new(ctx);

        let log = ActionLog {
            id: log_id,
            session_id,
            player,
            actions_blob_id,
            actions_root,
            finalized: false,
        };

        event::emit(ActionLogPublishedEvent {
            log_id: object::uid_to_address(&log.id),
            session_id,
            player,
            actions_blob_id,
            actions_root,
            published_at: tx_context::epoch(ctx),
        });

        log
    }

    /// Update action log with new actions
    ///
    /// Updates the ActionLog to reference a new Walrus blob containing
    /// additional actions. The new actions_root must match the session's
    /// current actions_root (verified by ZK proof updates).
    ///
    /// # Arguments
    /// * `log` - Mutable reference to ActionLog
    /// * `session` - Reference to GameSession (for validation)
    /// * `new_actions_blob_id` - New Walrus blob ID
    /// * `new_actions_root` - New actions root (must match session)
    /// * `ctx` - Transaction context
    ///
    /// # Aborts
    /// * `ENotOwner` - If caller is not the log owner
    /// * `EAlreadyFinalized` - If log is already finalized
    /// * `EActionsRootMismatch` - If actions_root doesn't match session
    ///
    /// # Events
    /// Emits `ActionLogUpdatedEvent` with new blob reference
    public fun update(
        log: &mut ActionLog,
        session: &GameSession,
        new_actions_blob_id: vector<u8>,
        new_actions_root: vector<u8>,
        ctx: &TxContext,
    ) {
        // Validate ownership
        assert!(tx_context::sender(ctx) == log.player, ENotOwner);

        // Validate not finalized
        assert!(!log.finalized, EAlreadyFinalized);

        // Validate session reference matches
        let session_id = object::id_to_address(&object::id(session));
        assert!(log.session_id == session_id, EActionsRootMismatch);

        // Validate actions root matches session
        assert!(
            game_session::borrow_actions_root(session) == &new_actions_root,
            EActionsRootMismatch
        );

        // Update log
        log.actions_blob_id = new_actions_blob_id;
        log.actions_root = new_actions_root;

        event::emit(ActionLogUpdatedEvent {
            log_id: object::uid_to_address(&log.id),
            session_id: log.session_id,
            actions_blob_id: new_actions_blob_id,
            actions_root: new_actions_root,
            updated_at: tx_context::epoch(ctx),
        });
    }

    /// Finalize the action log
    ///
    /// Marks the log as finalized, preventing further updates.
    /// Typically called after the associated GameSession is finalized.
    ///
    /// # Arguments
    /// * `log` - Mutable reference to ActionLog
    /// * `ctx` - Transaction context
    ///
    /// # Aborts
    /// * `ENotOwner` - If caller is not the log owner
    ///
    /// # Events
    /// Emits `ActionLogFinalizedEvent`
    public fun finalize(
        log: &mut ActionLog,
        ctx: &TxContext,
    ) {
        // Validate ownership
        assert!(tx_context::sender(ctx) == log.player, ENotOwner);

        // Mark as finalized
        log.finalized = true;

        event::emit(ActionLogFinalizedEvent {
            log_id: object::uid_to_address(&log.id),
            session_id: log.session_id,
            finalized_at: tx_context::epoch(ctx),
        });
    }

    /// Delete a finalized action log
    ///
    /// Removes the log from blockchain storage, freeing resources.
    /// Only finalized logs can be deleted, and only by their owner.
    ///
    /// # Arguments
    /// * `log` - The log to delete (ownership transferred, will be consumed)
    /// * `ctx` - Transaction context
    ///
    /// # Aborts
    /// * `EAlreadyFinalized` - Used as ENotFinalized - if log is not finalized yet
    /// * `ENotOwner` - If caller is not the log owner
    public fun delete(log: ActionLog, ctx: &TxContext) {
        // Validate finalized
        assert!(log.finalized, EAlreadyFinalized);

        // Validate ownership
        assert!(tx_context::sender(ctx) == log.player, ENotOwner);

        // Destructure and delete
        let ActionLog {
            id,
            session_id: _,
            player: _,
            actions_blob_id: _,
            actions_root: _,
            finalized: _,
        } = log;

        object::delete(id);
    }

    // ===== View Functions =====

    /// Get the associated session ID
    public fun session_id(log: &ActionLog): address {
        log.session_id
    }

    /// Get the log owner's address
    public fun player(log: &ActionLog): address {
        log.player
    }

    /// Borrow the actions blob ID
    public fun borrow_actions_blob_id(log: &ActionLog): &vector<u8> {
        &log.actions_blob_id
    }

    /// Borrow the actions root
    public fun borrow_actions_root(log: &ActionLog): &vector<u8> {
        &log.actions_root
    }

    /// Check if the log is finalized
    public fun is_finalized(log: &ActionLog): bool {
        log.finalized
    }
}
