//! RISC0 zkVM guest program for batch state transition verification.
//!
//! This program runs inside the RISC0 zkVM and proves that executing
//! a batch of actions on a given state produces a specific final state deterministically.
//!
//! # Verification Model
//!
//! ```text
//! Host → zkVM Guest:
//!   - OracleSnapshot (static game content)
//!   - start_state (GameState at batch start)
//!   - actions (Vec<Action> to execute sequentially)
//!
//! Guest executes:
//!   1. Create GameEngine with start_state
//!   2. Execute each action sequentially using GameEngine::execute()
//!   3. Compute final state deterministically
//!   4. Commit public outputs to journal
//!
//! zkVM → Host:
//!   - Receipt (proof + journal):
//!     - end_state (computed final state)
//!     - action_count (number of actions executed)
//!
//! Host verifies:
//!   - Receipt is valid (cryptographic verification)
//!   - Journal's end_state matches expected final state
//!   - Action count matches expected batch size
//!
//! # Design Rationale
//!
//! Batch proving is more efficient than individual action proofs:
//! - Single proof for N actions instead of N proofs
//! - Amortizes proof generation overhead across all actions
//! - Reduces on-chain verification cost (one proof to verify)
//! - Maintains same security guarantees as individual proofs
//!
//! Consistency verification happens on the host side:
//! - Reduces zkVM computational overhead
//! - Maintains security: journal data is cryptographically committed
//! - Host can efficiently verify journal contents match expected results
//!
//! # Delta Optimization
//!
//! Delta computation is disabled in zkVM mode (via `zkvm` feature flag):
//! - Eliminates clone() overhead per action
//! - Skips state comparison pass
//! - Reduces journal size (no delta commitment needed)
//! - Host can compute deltas if needed by comparing before/after states
//! ```

#![no_main]
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use game_core::{Action, GameEngine, OracleSnapshot, SnapshotOracleBundle};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // Read inputs from host in order
    // 1. Oracle snapshot (static game content)
    let oracle_snapshot: OracleSnapshot = env::read();

    // 2. Game state at batch start
    let mut state: game_core::GameState = env::read();

    // 3. Batch of actions to execute sequentially
    let actions: Vec<Action> = env::read();

    // Create oracle bundle from snapshot
    let oracle_bundle = SnapshotOracleBundle::new(&oracle_snapshot);
    let game_env = oracle_bundle.as_env().into_game_env();

    // Execute all actions sequentially using game-core logic
    // This is the CORE PROOF: GameEngine::execute() runs N times inside zkVM
    let mut engine = GameEngine::new(&mut state);

    let action_count = actions.len();
    for (index, action) in actions.iter().enumerate() {
        engine.execute(game_env, action).unwrap_or_else(|e| {
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

    // Commit public outputs to journal
    env::commit(&state); // Final state after executing all actions
    env::commit(&(action_count as u64)); // Number of actions executed
}
