//! RISC0 zkVM guest program for batch state transition verification.
//!
//! This program runs inside the RISC0 zkVM and proves that executing
//! a batch of actions on a given state produces a specific final state deterministically.
//!
//! # Journal Structure (RISC0 Groth16 Compatible)
//!
//! The guest commits 6 fields to the journal in a specific order:
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
//! Journal Digest: SHA256(journal_bytes)
//! ```
//!
//! # Two-Stage Verification
//!
//! **Stage 1 (On-chain):** Groth16 proof verification with journal_digest
//! - Verifies proof is valid
//! - Verifies journal_digest matches proof
//! - This proves: "Some valid execution produced this journal digest"
//!
//! **Stage 2 (On-chain):** Journal content verification
//! - Verifies provided journal data hashes to journal_digest
//! - Extracts and validates individual fields from journal
//! - This proves: "The journal contains these specific values"
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
//!   7. Commit all 6 fields to journal in order
//!
//! zkVM → Host:
//!   - Receipt (proof + journal):
//!     - oracle_root
//!     - seed_commitment
//!     - prev_state_root
//!     - actions_root
//!     - new_state_root
//!     - new_nonce
//!
//! Host verifies:
//!   - Receipt is valid (cryptographic verification)
//!   - Journal fields match expected values
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
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use game_core::{compute_actions_root, Action, GameEngine, GameState, OracleSnapshot, SnapshotOracleBundle};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // Read inputs from host in order
    // 1. Oracle snapshot (static game content)
    let oracle_snapshot: OracleSnapshot = env::read();

    // 2. Seed commitment (RNG seed commitment)
    let seed_commitment: [u8; 32] = env::read();

    // 3. Game state at batch start
    let mut state: GameState = env::read();

    // 4. Batch of actions to execute sequentially
    let actions: Vec<Action> = env::read();

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
    // COMMIT TO JOURNAL (CRITICAL: Must maintain exact order!)
    // ========================================================================
    //
    // Journal format (168 bytes total):
    // - 5 × 32 bytes (roots) = 160 bytes
    // - 1 × 8 bytes (nonce) = 8 bytes
    //
    // This order MUST match:
    // - Host-side journal parsing in crates/zk/src/zkvm/risc0.rs
    // - On-chain verification in contracts/move/sources/proof_verifier.move
    //
    // IMPORTANT: We manually concatenate all fields into a single 168-byte array
    // and commit it as raw bytes to avoid bincode serialization overhead.
    // Each env::commit(&value) call adds type metadata, causing journal bloat.
    // ========================================================================

    let mut journal = [0u8; 168];

    // Copy all fields into journal buffer in exact order
    journal[0..32].copy_from_slice(&oracle_root);        // offset 0..32
    journal[32..64].copy_from_slice(&seed_commitment);   // offset 32..64
    journal[64..96].copy_from_slice(&prev_state_root);   // offset 64..96
    journal[96..128].copy_from_slice(&actions_root);     // offset 96..128
    journal[128..160].copy_from_slice(&new_state_root);  // offset 128..160
    journal[160..168].copy_from_slice(&new_nonce.to_le_bytes()); // offset 160..168

    // Commit the entire journal as raw bytes (exactly 168 bytes, no metadata)
    env::commit_slice(&journal);
}
