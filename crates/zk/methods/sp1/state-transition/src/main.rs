//! SP1 zkVM guest program for batch state transition verification.
//!
//! This program runs inside the SP1 zkVM and proves that executing
//! a batch of actions on a given state produces a specific final state deterministically.
//!
//! # Public Values Structure (SP1 Compatible)
//!
//! The guest commits 6 fields to public values in a specific order:
//!
//! ```text
//! 1. oracle_root       (32 bytes) - Commitment to static game content
//! 2. seed_commitment   (32 bytes) - Commitment to RNG seed
//! 3. prev_state_root   (32 bytes) - State hash before execution
//! 4. actions_root      (32 bytes) - Commitment to action sequence (Walrus blob_id)
//! 5. new_state_root    (32 bytes) - State hash after execution
//! 6. new_nonce         (8 bytes)  - Action counter after execution
//!
//! Total: 168 bytes
//! Public Values Digest: SHA256(public_values_bytes)
//! ```
//!
//! # Two-Stage Verification
//!
//! **Stage 1 (On-chain):** Groth16/PLONK proof verification with public_values_digest
//! - Verifies proof is valid
//! - Verifies public_values_digest matches proof
//! - This proves: "Some valid execution produced this digest"
//!
//! **Stage 2 (On-chain):** Public values content verification
//! - Verifies provided public values data hashes to digest
//! - Extracts and validates individual fields from public values
//! - This proves: "The public values contain these specific committed values"
//!
//! # Verification Model
//!
//! ```text
//! Host → zkVM Guest:
//!   - OracleSnapshot (static game content)
//!   - seed_commitment (RNG seed commitment)
//!   - start_state (GameState at batch start)
//!   - actions (Vec<Action> to execute sequentially)
//!
//! Guest executes:
//!   1. Compute oracle_root from OracleSnapshot
//!   2. Compute prev_state_root from start_state
//!   3. Compute actions_root from actions
//!   4. Execute each action sequentially
//!   5. Compute new_state_root from final state
//!   6. Get new_nonce from final state
//!   7. Commit all 6 fields to public values in order
//!
//! zkVM → Host:
//!   - Proof (with public values):
//!     - oracle_root
//!     - seed_commitment
//!     - prev_state_root
//!     - actions_root
//!     - new_state_root
//!     - new_nonce
//!
//! Host verifies:
//!   - Proof is valid (cryptographic verification)
//!   - Public values match expected values
//!
//! # Design Rationale
//!
//! Batch proving is more efficient than individual action proofs:
//! - Single proof for N actions instead of N proofs
//! - Amortizes proof generation overhead across all actions
//! - Reduces on-chain verification cost (one proof to verify)
//! - Maintains same security guarantees as individual proofs
//!
//! # Delta Optimization
//!
//! Delta computation is disabled in zkVM mode (via `zkvm` feature flag):
//! - Eliminates clone() overhead per action
//! - Skips state comparison pass
//! - Reduces computation inside zkVM
//! - Host can compute deltas if needed by comparing before/after states
//! ```

#![no_main]
sp1_zkvm::entrypoint!(main);

use game_core::{
    compute_actions_root, Action, GameEngine, GameState, OracleSnapshot, SnapshotOracleBundle,
};

pub fn main() {
    // ========================================================================
    // READ INPUTS FROM HOST
    // ========================================================================

    // 1. Oracle snapshot (static game content)
    let oracle_snapshot: OracleSnapshot = sp1_zkvm::io::read();

    // 2. Seed commitment (RNG seed commitment)
    let seed_commitment: [u8; 32] = sp1_zkvm::io::read();

    // 3. Game state at batch start
    let mut state: GameState = sp1_zkvm::io::read();

    // 4. Batch of actions to execute sequentially
    let actions: Vec<Action> = sp1_zkvm::io::read();

    // ========================================================================
    // COMPUTE ROOTS BEFORE EXECUTION
    // ========================================================================

    // Compute oracle root (commitment to static game content)
    let oracle_root = oracle_snapshot.compute_oracle_root();

    // Compute previous state root (state before execution)
    let prev_state_root = state.compute_state_root();

    // Compute actions root (commitment to action sequence - simulates Walrus blob_id)
    let actions_root = compute_actions_root(&actions);

    // ========================================================================
    // EXECUTE ACTIONS
    // ========================================================================

    // Create oracle bundle from snapshot
    let oracle_bundle = SnapshotOracleBundle::new(&oracle_snapshot);
    let env = oracle_bundle.as_env();

    // Execute all actions sequentially using game-core logic
    // This is the CORE PROOF: GameEngine::execute() runs N times inside zkVM
    let mut engine = GameEngine::new(&mut state);

    let action_count = actions.len();
    for (index, action) in actions.iter().enumerate() {
        // Convert to GameEnv on each iteration (minimal overhead: 6 pointer copies)
        engine.execute(env.as_game_env(), action).unwrap_or_else(|e| {
            // In zkVM guest, panics are converted to proof failures
            // Provide detailed error context for debugging proof generation issues
            panic!(
                "Action execution failed in zkVM guest (action {}/{}): {:?}",
                index + 1,
                action_count,
                e
            )
        });
    }

    // ========================================================================
    // COMPUTE ROOTS AFTER EXECUTION
    // ========================================================================

    // Compute new state root (state after execution)
    let new_state_root = state.compute_state_root();

    // Get new nonce (action counter after execution)
    let new_nonce = state.nonce();

    // ========================================================================
    // COMMIT TO PUBLIC VALUES (CRITICAL: Must maintain exact order!)
    // ========================================================================
    //
    // Public values format (168 bytes total):
    // - 5 × 32 bytes (roots) = 160 bytes
    // - 1 × 8 bytes (nonce) = 8 bytes
    //
    // This order MUST match:
    // - Host-side public values parsing in crates/zk/src/sp1/prover.rs
    // - On-chain verification in contracts/move/sources/proof_verifier.move
    //
    // IMPORTANT: We manually concatenate all fields into a single 168-byte array
    // and commit it as raw bytes to avoid bincode serialization overhead.
    // SP1's commit_slice() adds no metadata, maintaining exact byte layout.
    // ========================================================================

    let mut public_values = [0u8; 168];

    // Copy all fields into public values buffer in exact order
    public_values[0..32].copy_from_slice(&oracle_root); // offset 0..32
    public_values[32..64].copy_from_slice(&seed_commitment); // offset 32..64
    public_values[64..96].copy_from_slice(&prev_state_root); // offset 64..96
    public_values[96..128].copy_from_slice(&actions_root); // offset 96..128
    public_values[128..160].copy_from_slice(&new_state_root); // offset 128..160
    public_values[160..168].copy_from_slice(&new_nonce.to_le_bytes()); // offset 160..168

    // Commit the entire public values as raw bytes (exactly 168 bytes, no metadata)
    sp1_zkvm::io::commit_slice(&public_values);
}
