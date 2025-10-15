//! RISC0 zkVM guest program for game state verification.
//!
//! This program runs inside the RISC0 zkVM and proves that executing
//! an action on a given state produces a specific result deterministically.
//!
//! # Verification Model
//!
//! ```text
//! Host → zkVM Guest:
//!   - OracleSnapshot (static game content)
//!   - before_state (GameState)
//!   - action (Action)
//!
//! Guest executes:
//!   1. GameEngine::execute(env, action) on mutable before_state
//!   2. Computes result state and delta deterministically
//!   3. Commits public outputs to journal
//!
//! zkVM → Host:
//!   - Receipt (proof + journal):
//!     - after_state (computed result)
//!     - delta (state changes)
//!     - action (what was executed)
//!
//! Host verifies:
//!   - Receipt is valid (cryptographic verification)
//!   - Journal's after_state matches simulation worker's expected state
//!
//! # Design Rationale
//!
//! Consistency verification happens on the host side rather than inside zkVM:
//! - Reduces zkVM computational overhead (state comparison is expensive)
//! - Reduces input size (no need to pass expected state into zkVM)
//! - Maintains security: journal data is cryptographically committed in the proof
//! - Host can efficiently verify journal contents match expected results
//! ```

#![no_main]
#![no_std]

extern crate alloc;

use game_core::{OracleSnapshot, SnapshotOracleBundle};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // Read inputs from host in order
    // 1. Oracle snapshot (static game content)
    let oracle_snapshot: OracleSnapshot = env::read();

    // 2. Game state before action execution
    let mut state: game_core::GameState = env::read();

    // 3. Action to execute
    let action: game_core::Action = env::read();

    // Create oracle bundle from snapshot
    let oracle_bundle = SnapshotOracleBundle::new(&oracle_snapshot);

    // Execute action deterministically using game-core logic
    // This is the CORE PROOF: GameEngine::execute() runs inside zkVM
    let mut engine = game_core::GameEngine::new(&mut state);

    let delta = engine
        .execute(oracle_bundle.as_env().into_game_env(), &action)
        .unwrap_or_else(|e| {
            // In zkVM guest, panics are converted to proof failures
            // Provide detailed error context for debugging proof generation issues
            panic!("Action execution failed in zkVM guest: {:?}", e)
        });

    // Commit public outputs to journal
    // The host will verify these outputs match the simulation worker's results
    // This approach keeps zkVM overhead minimal while maintaining security
    env::commit(&state); // Resulting state after action execution
    env::commit(&delta); // State changes
    env::commit(&action); // Action that was executed
}
