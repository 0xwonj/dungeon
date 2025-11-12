/// Game Session Module
///
/// Manages stateful on-chain game sessions with progressive ZK proof verification.
/// Each session tracks state commitments (oracle, state, actions) that are updated
/// via zero-knowledge proofs, enabling verifiable off-chain gameplay.
///
/// # Design
/// - Sessions are owned objects that track game state commitments
/// - Progressive updates via ZK proofs (players choose proof frequency)
/// - Content-addressed oracle data (deterministic replay)
/// - Optimistic gameplay with challenge period for verification
/// - Multiple action logs stored via Dynamic Object Fields during challenge period
/// - Events emitted for all state transitions
///
/// # Challenge-Based Verification
/// - Players submit ZK proofs with action logs published to Walrus
/// - Anyone can download actions and challenge invalid gameplay during challenge period
/// - Actions root in ZK proof cryptographically binds published actions to verified state
/// - Multiple updates can occur during challenge period (each stored separately)
/// - Old action logs can be cleaned up after challenge period expires
/// - Enables trustless verification without authority signatures
module dungeon::game_session {
    use sui::event;
    use sui::dynamic_object_field as dof;
    use dungeon::proof_verifier::{Self, VerifyingKey};
    use walrus::blob::Blob;

    // ===== Error Codes =====

    /// Caller is not the session owner
    const ENotOwner: u64 = 1;
    /// Session is not finalized yet
    const ENotFinalized: u64 = 2;
    /// Action log blob does not exist
    const EActionLogNotFound: u64 = 3;
    /// Challenge period has not expired yet
    const EChallengeNotExpired: u64 = 4;
    /// Action logs must be cleaned up before finalization
    const EActionLogsRemaining: u64 = 5;

    // ===== Constants =====

    /// Challenge period duration in epochs (e.g., 7 days worth of epochs)
    /// After this period, action logs can be cleaned up
    const CHALLENGE_PERIOD_EPOCHS: u64 = 7 * 24 * 60; // ~7 days (assuming 1 epoch = 1 minute)

    // ===== Structs =====

    /// Represents an active game session
    public struct GameSession has key, store {
        id: UID,
        /// Player address (session owner)
        player: address,

        // Immutable context (set at creation)
        /// Oracle data commitment (content hash)
        oracle_root: vector<u8>,
        /// Initial state root at game start
        initial_state_root: vector<u8>,
        /// Seed commitment for RNG fairness
        seed_commitment: vector<u8>,

        // Mutable state (updated by proofs)
        /// Current game state root
        state_root: vector<u8>,
        /// Action execution nonce (incremented after each action)
        nonce: u64,
        /// Number of pending action logs awaiting cleanup
        pending_action_logs: u64,

        // Status
        /// Whether the session is finalized
        finalized: bool,
    }

    /// Wrapper for action log blob with metadata
    /// Stored as Dynamic Object Field child of GameSession
    /// Key: nonce (u64)
    /// The blob_id serves as the cryptographic commitment to the actions
    public struct ActionLogBlob has key, store {
        id: UID,
        /// Walrus blob containing the full action sequence
        blob: Blob,
        /// Epoch when this action log was submitted
        submitted_at: u64,
        /// State root at the beginning of this action batch (for fraud proof verification)
        start_state_root: vector<u8>,
    }

    // ===== Events =====

    /// Emitted when a new game session is started
    public struct SessionStartedEvent has copy, drop {
        session_id: address,
        player: address,
        oracle_root: vector<u8>,
        started_at: u64,
    }

    /// Emitted when session state is updated via ZK proof
    public struct SessionUpdatedEvent has copy, drop {
        session_id: address,
        new_state_root: vector<u8>,
        nonce: u64,
        updated_at: u64,
    }

    /// Emitted when a game session is finalized
    public struct SessionFinalizedEvent has copy, drop {
        session_id: address,
        final_state_root: vector<u8>,
        final_nonce: u64,
        finalized_at: u64,
    }

    /// Emitted when an action log is published or updated
    public struct ActionLogPublishedEvent has copy, drop {
        session_id: address,
        actions_blob_id: u256,
        nonce: u64,
        published_at: u64,
    }

    // ===== Public Functions =====

    /// Create a new game session
    ///
    /// Initializes a new GameSession object with the provided commitments
    /// and transfers it to the transaction sender.
    ///
    /// # Arguments
    /// * `oracle_root` - Content hash of oracle data (maps, items, NPCs, etc.)
    /// * `initial_state_root` - Merkle root of initial game state
    /// * `seed_commitment` - Commitment to RNG seed (hash of seed)
    /// * `ctx` - Transaction context
    ///
    /// # Events
    /// Emits `SessionStartedEvent` with session details
    entry fun create(
        oracle_root: vector<u8>,
        initial_state_root: vector<u8>,
        seed_commitment: vector<u8>,
        ctx: &mut TxContext,
    ) {
        let session_id = object::new(ctx);
        let player = tx_context::sender(ctx);

        let session = GameSession {
            id: session_id,
            player,
            oracle_root,
            initial_state_root,
            seed_commitment,
            state_root: initial_state_root,
            nonce: 0,
            pending_action_logs: 0,
            finalized: false,
        };

        event::emit(SessionStartedEvent {
            session_id: session_id(&session),
            player,
            oracle_root,
            started_at: tx_context::epoch(ctx),
        });

        transfer::public_transfer(session, player);
    }

    /// Update session state with ZK proof and action log
    ///
    /// The ZK proof must verify:
    /// - Transition from prev_state_root to new_state_root is valid
    /// - Actions executed match the Walrus blob (blob_id == actions_root)
    /// - All game rules were correctly enforced
    /// - Oracle commitment matches the session's oracle_root
    ///
    /// If the session was previously finalized, this will unfinalize it
    /// (since new action logs require a new challenge period).
    ///
    /// # Arguments
    /// * `session` - Mutable reference to GameSession
    /// * `vk` - Verifying key for proof verification
    /// * `proof` - Groth16 proof bytes
    /// * `new_state_root` - New game state root after executing actions
    /// * `new_nonce` - Updated nonce (incremented after each action)
    /// * `actions_blob` - Walrus blob containing full action sequence
    /// * `ctx` - Transaction context (for sender and epoch)
    ///
    /// # Aborts
    /// * `ENotOwner` - If caller is not the session owner
    /// * `EActionsRootMismatch` - If blob_id doesn't match actions_root from proof
    ///
    /// # Events
    /// Emits `SessionUpdatedEvent` and `ActionLogPublishedEvent`
    public fun update(
        session: &mut GameSession,
        vk: &VerifyingKey,
        proof: vector<u8>,
        new_state_root: vector<u8>,
        new_nonce: u64,
        actions_blob: Blob,
        ctx: &mut TxContext,
    ) {
        // Validate ownership
        assert!(tx_context::sender(ctx) == session.player, ENotOwner);

        // Get blob_id which serves as actions_root
        let blob_id = walrus::blob::blob_id(&actions_blob);
        let actions_root = u256_to_bytes(blob_id);

        // Construct public inputs for verification
        let public_inputs = proof_verifier::new_public_inputs(
            session.oracle_root,
            session.seed_commitment,
            session.state_root,
            actions_root,
            new_state_root,
            new_nonce,
        );

        // Verify ZK proof (aborts if invalid)
        // The proof verifies that blob_id (actions_root) matches the actions used in state transition
        proof_verifier::verify_game_proof(vk, &public_inputs, proof);

        // Create action log blob wrapper with metadata
        let action_log = ActionLogBlob {
            id: object::new(ctx),
            blob: actions_blob,
            submitted_at: tx_context::epoch(ctx),
            start_state_root: session.state_root,
        };

        // Store action log as Dynamic Object Field using nonce as key
        dof::add(
            &mut session.id,
            new_nonce,
            action_log
        );

        // Update session state
        session.state_root = new_state_root;
        session.nonce = new_nonce;
        session.pending_action_logs = session.pending_action_logs + 1;

        // Unfinalize if previously finalized (new action logs require challenge period)
        if (session.finalized) {
            session.finalized = false;
        };

        // Emit events
        event::emit(ActionLogPublishedEvent {
            session_id: session_id(session),
            actions_blob_id: blob_id,
            nonce: new_nonce,
            published_at: tx_context::epoch(ctx),
        });

        event::emit(SessionUpdatedEvent {
            session_id: session_id(session),
            new_state_root,
            nonce: new_nonce,
            updated_at: tx_context::epoch(ctx),
        });
    }

    /// Remove expired action logs after challenge period
    ///
    /// Removes action log blobs that have passed the challenge period, freeing up
    /// storage and providing storage rebate. Can be called by anyone (not just owner)
    /// to incentivize cleanup.
    ///
    /// The challenge period is defined by CHALLENGE_PERIOD_EPOCHS constant.
    /// This function removes the ActionLogBlob wrappers and returns the Walrus Blobs.
    ///
    /// For single nonce removal, pass a single-element vector: `vector[nonce]`.
    /// If any action log hasn't expired or doesn't exist, the entire transaction aborts.
    ///
    /// # Arguments
    /// * `session` - Mutable reference to GameSession
    /// * `nonces` - Vector of nonces to remove (action logs must be expired)
    /// * `ctx` - Transaction context (for current epoch)
    ///
    /// # Returns
    /// Vector of Walrus Blob objects (in same order as input nonces)
    ///
    /// # Aborts
    /// * `EActionLogNotFound` - If any action log doesn't exist
    /// * `EChallengeNotExpired` - If any action log hasn't expired yet
    public fun remove_expired_action_logs(
        session: &mut GameSession,
        nonces: vector<u64>,
        ctx: &TxContext,
    ): vector<Blob> {
        let mut results = vector::empty<Blob>();
        let mut i = 0;
        let len = vector::length(&nonces);

        while (i < len) {
            let nonce = *vector::borrow(&nonces, i);

            // Check if action log exists
            assert!(dof::exists_<u64>(&session.id, nonce), EActionLogNotFound);

            // Borrow to check expiration
            let action_log = dof::borrow<u64, ActionLogBlob>(&session.id, nonce);
            let current_epoch = tx_context::epoch(ctx);
            let challenge_expiry = action_log.submitted_at + CHALLENGE_PERIOD_EPOCHS;

            assert!(current_epoch >= challenge_expiry, EChallengeNotExpired);

            // Remove and unwrap
            let action_log = dof::remove<u64, ActionLogBlob>(&mut session.id, nonce);
            let ActionLogBlob { id, blob, submitted_at: _, start_state_root: _ } = action_log;
            object::delete(id);

            vector::push_back(&mut results, blob);
            i = i + 1;
        };

        // Decrement counter by the number of removed logs
        session.pending_action_logs = session.pending_action_logs - len;

        results
    }

    /// Finalize the game session
    ///
    /// Marks the session as finalized, preventing further updates.
    /// This is typically called when the game is complete (player died or won).
    ///
    /// All action logs must be cleaned up before finalization to ensure
    /// all challenge periods have expired and verification is complete.
    ///
    /// # Arguments
    /// * `session` - Mutable reference to GameSession
    /// * `ctx` - Transaction context (for sender verification)
    ///
    /// # Aborts
    /// * `ENotOwner` - If caller is not the session owner
    /// * `EActionLogsRemaining` - If there are pending action logs that haven't been cleaned up
    ///
    /// # Events
    /// Emits `SessionFinalizedEvent` with final state and turn count
    public fun finalize(
        session: &mut GameSession,
        ctx: &TxContext,
    ) {
        // Check ownership
        assert!(tx_context::sender(ctx) == session.player, ENotOwner);

        // Ensure all action logs have been cleaned up
        assert!(session.pending_action_logs == 0, EActionLogsRemaining);

        // Mark as finalized
        session.finalized = true;

        event::emit(SessionFinalizedEvent {
            session_id: session_id(session),
            final_state_root: session.state_root,
            final_nonce: session.nonce,
            finalized_at: tx_context::epoch(ctx),
        });
    }

    /// Delete a finalized game session
    ///
    /// Removes the session from blockchain storage, freeing resources and providing
    /// storage rebate to the caller. Only finalized sessions can be deleted, and only
    /// by their owner.
    ///
    /// This is useful for cleaning up completed game sessions that are no longer needed,
    /// reducing storage costs. Important sessions (e.g., high scores) can be kept on-chain
    /// for historical records and leaderboards.
    ///
    /// # Arguments
    /// * `session` - The session to delete (ownership transferred, will be consumed)
    /// * `ctx` - Transaction context (for sender verification)
    ///
    /// # Aborts
    /// * `ENotFinalized` - If session is not finalized yet
    /// * `ENotOwner` - If caller is not the session owner
    public fun delete(session: GameSession, ctx: &TxContext) {
        // Validate session is finalized
        assert!(session.finalized, ENotFinalized);

        // Validate ownership
        assert!(tx_context::sender(ctx) == session.player, ENotOwner);

        // Destructure session and delete
        let GameSession {
            id,
            player: _,
            oracle_root: _,
            initial_state_root: _,
            seed_commitment: _,
            state_root: _,
            nonce: _,
            pending_action_logs: _,
            finalized: _,
        } = session;

        object::delete(id);
    }

    // ===== View Functions =====

    /// Get the session's object ID as address
    ///
    /// Returns the unique identifier for this session as an address.
    /// Useful for event tracking and referencing sessions.
    ///
    /// # Arguments
    /// * `session` - Reference to GameSession
    ///
    /// # Returns
    /// Address representation of the session's UID
    public fun session_id(session: &GameSession): address {
        object::uid_to_address(&session.id)
    }

    /// Get the session owner's address
    public fun player(session: &GameSession): address {
        session.player
    }

    /// Borrow the oracle root commitment
    public fun borrow_oracle_root(session: &GameSession): &vector<u8> {
        &session.oracle_root
    }

    /// Borrow the current state root
    public fun borrow_state_root(session: &GameSession): &vector<u8> {
        &session.state_root
    }

    /// Get the current nonce (action execution count)
    public fun nonce(session: &GameSession): u64 {
        session.nonce
    }

    /// Get the number of pending action logs awaiting cleanup
    public fun pending_action_logs(session: &GameSession): u64 {
        session.pending_action_logs
    }

    /// Check if the session is finalized
    public fun is_finalized(session: &GameSession): bool {
        session.finalized
    }

    /// Check if an action log exists for a given nonce
    ///
    /// Use this to verify if a specific action log is available for challenge verification.
    ///
    /// # Arguments
    /// * `session` - Reference to GameSession
    /// * `nonce` - Nonce of the action log to check
    ///
    /// # Returns
    /// True if action log exists, false otherwise
    public fun has_action_log(session: &GameSession, nonce: u64): bool {
        dof::exists_<u64>(&session.id, nonce)
    }

    /// Borrow an action log blob for a specific nonce
    ///
    /// Returns a reference to the ActionLogBlob containing the Walrus blob ID and metadata.
    /// Use this for challenge validation and verification.
    ///
    /// # Arguments
    /// * `session` - Reference to GameSession
    /// * `nonce` - Nonce of the action log to borrow
    ///
    /// # Returns
    /// Reference to ActionLogBlob
    ///
    /// # Aborts
    /// * `EActionLogNotFound` - If action log with given nonce doesn't exist
    public fun borrow_action_log(session: &GameSession, nonce: u64): &ActionLogBlob {
        assert!(dof::exists_<u64>(&session.id, nonce), EActionLogNotFound);
        dof::borrow<u64, ActionLogBlob>(&session.id, nonce)
    }

    /// Get the blob ID from an ActionLogBlob
    ///
    /// Extracts the Walrus blob ID for downloading action sequence.
    ///
    /// # Arguments
    /// * `action_log` - Reference to ActionLogBlob
    ///
    /// # Returns
    /// Blob ID as u256
    public fun action_log_blob_id(action_log: &ActionLogBlob): u256 {
        walrus::blob::blob_id(&action_log.blob)
    }

    /// Get submission epoch from action log
    public fun action_log_submitted_at(action_log: &ActionLogBlob): u64 {
        action_log.submitted_at
    }

    /// Get the start state root from action log (for fraud proof verification)
    public fun action_log_start_state_root(action_log: &ActionLogBlob): &vector<u8> {
        &action_log.start_state_root
    }

    /// Get the initial state root (for replay verification)
    public fun borrow_initial_state_root(session: &GameSession): &vector<u8> {
        &session.initial_state_root
    }

    /// Get the seed commitment (for RNG verification)
    public fun borrow_seed_commitment(session: &GameSession): &vector<u8> {
        &session.seed_commitment
    }

    /// Get the challenge period duration in epochs
    public fun challenge_period_epochs(): u64 {
        CHALLENGE_PERIOD_EPOCHS
    }

    // ===== Helper Functions =====

    /// Convert u256 to 32-byte vector (big-endian)
    /// Used to convert Walrus blob_id to actions_root format for ZK proof
    fun u256_to_bytes(value: u256): vector<u8> {
        let mut bytes = vector::empty<u8>();
        let mut v = value;
        let mut i = 0;

        // Extract 32 bytes (big-endian)
        while (i < 32) {
            vector::push_back(&mut bytes, ((v & 0xFF) as u8));
            v = v >> 8;
            i = i + 1;
        };

        // Reverse for big-endian
        vector::reverse(&mut bytes);
        bytes
    }
}
