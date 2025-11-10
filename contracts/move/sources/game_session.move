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
/// - Events emitted for all state transitions
module dungeon::game_session {
    use sui::event;
    use dungeon::proof_verifier::{Self, VerifyingKey};

    // ===== Error Codes =====

    /// Caller is not the session owner
    const ENotOwner: u64 = 1;
    /// Session is already finalized
    const EAlreadyFinalized: u64 = 2;
    /// Session is not finalized yet
    const ENotFinalized: u64 = 3;

    // ===== Structs =====

    /// Represents an active game session
    ///
    /// Timestamps are not stored in the struct to minimize storage costs and
    /// avoid including non-deterministic data in ZK proofs. Time information
    /// is available in events for off-chain indexing and analytics.
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
        /// Accumulated actions root
        actions_root: vector<u8>,
        /// Action execution nonce (incremented after each action)
        nonce: u64,

        // Status
        /// Whether the session is finalized
        finalized: bool,
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
        new_actions_root: vector<u8>,
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

    // ===== Public Functions =====

    /// Create a new game session
    ///
    /// Initializes a new GameSession object with the provided commitments.
    /// The session is returned to the caller (use transfer::public_transfer to send to player).
    ///
    /// # Arguments
    /// * `oracle_root` - Content hash of oracle data (maps, items, NPCs, etc.)
    /// * `initial_state_root` - Merkle root of initial game state
    /// * `seed_commitment` - Commitment to RNG seed (hash of seed)
    /// * `ctx` - Transaction context
    ///
    /// # Returns
    /// A new GameSession object owned by the caller
    ///
    /// # Events
    /// Emits `SessionStartedEvent` with session details
    public fun create(
        oracle_root: vector<u8>,
        initial_state_root: vector<u8>,
        seed_commitment: vector<u8>,
        ctx: &mut TxContext,
    ): GameSession {
        let session_id = object::new(ctx);
        let player = tx_context::sender(ctx);

        let session = GameSession {
            id: session_id,
            player,
            oracle_root,
            initial_state_root,
            seed_commitment,
            state_root: initial_state_root,
            actions_root: vector::empty(),  // Empty actions root
            nonce: 0,
            finalized: false,
        };

        event::emit(SessionStartedEvent {
            session_id: object::uid_to_address(&session.id),
            player,
            oracle_root,
            started_at: tx_context::epoch(ctx),
        });

        session
    }

    /// Update session state with ZK proof
    ///
    /// Verifies a zero-knowledge proof of state transition and updates the session.
    /// Players can choose their own proof frequency (every turn, every 100 turns, etc.)
    ///
    /// The ZK proof must verify:
    /// - Transition from prev_state_root to new_state_root is valid
    /// - Transition from prev_actions_root to new_actions_root is valid
    /// - All game rules were correctly enforced
    /// - Oracle commitment matches the session's oracle_root
    ///
    /// # Arguments
    /// * `session` - Mutable reference to GameSession
    /// * `vk` - Verifying key for proof verification
    /// * `proof` - Groth16 proof bytes
    /// * `new_state_root` - New game state root after executing actions
    /// * `new_actions_root` - New accumulated actions root
    /// * `new_nonce` - Updated nonce (incremented after each action)
    /// * `ctx` - Transaction context (for sender verification)
    ///
    /// # Aborts
    /// * `ENotOwner` - If caller is not the session owner
    /// * `EAlreadyFinalized` - If session is already finalized
    ///
    /// # Events
    /// Emits `SessionUpdatedEvent` with new state commitments
    public fun update(
        session: &mut GameSession,
        vk: &VerifyingKey,
        proof: vector<u8>,
        new_state_root: vector<u8>,
        new_actions_root: vector<u8>,
        new_nonce: u64,
        ctx: &TxContext,
    ) {
        // Check ownership
        assert!(tx_context::sender(ctx) == session.player, ENotOwner);

        // Check not finalized
        assert!(!session.finalized, EAlreadyFinalized);

        // Construct public inputs for verification
        let public_inputs = proof_verifier::new_public_inputs(
            session.oracle_root,
            session.seed_commitment,
            session.state_root,
            session.actions_root,
            session.nonce,
            new_state_root,
            new_actions_root,
            new_nonce,
        );

        // Verify ZK proof
        proof_verifier::verify_game_proof(vk, &public_inputs, proof);

        // Update session state
        session.state_root = new_state_root;
        session.actions_root = new_actions_root;
        session.nonce = new_nonce;

        event::emit(SessionUpdatedEvent {
            session_id: object::uid_to_address(&session.id),
            new_state_root,
            new_actions_root,
            nonce: new_nonce,
            updated_at: tx_context::epoch(ctx),
        });
    }

    /// Finalize the game session
    ///
    /// Marks the session as finalized, preventing further updates.
    /// This is typically called when the game is complete (player died or won).
    ///
    /// # Arguments
    /// * `session` - Mutable reference to GameSession
    /// * `ctx` - Transaction context (for sender verification)
    ///
    /// # Aborts
    /// * `ENotOwner` - If caller is not the session owner
    ///
    /// # Events
    /// Emits `SessionFinalizedEvent` with final state and turn count
    public fun finalize(
        session: &mut GameSession,
        ctx: &TxContext,
    ) {
        // Check ownership
        assert!(tx_context::sender(ctx) == session.player, ENotOwner);

        // Mark as finalized
        session.finalized = true;

        event::emit(SessionFinalizedEvent {
            session_id: object::uid_to_address(&session.id),
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
    ///
    /// # Example
    /// ```
    /// // After completing a game
    /// game_session::finalize(&mut session, ctx);
    ///
    /// // Optionally delete to free storage
    /// game_session::delete(session, ctx);
    /// ```
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
            actions_root: _,
            nonce: _,
            finalized: _,
        } = session;

        object::delete(id);
    }

    // ===== View Functions =====

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

    /// Borrow the accumulated actions root
    public fun borrow_actions_root(session: &GameSession): &vector<u8> {
        &session.actions_root
    }

    /// Get the current nonce (action execution count)
    public fun nonce(session: &GameSession): u64 {
        session.nonce
    }

    /// Check if the session is finalized
    public fun is_finalized(session: &GameSession): bool {
        session.finalized
    }
}
