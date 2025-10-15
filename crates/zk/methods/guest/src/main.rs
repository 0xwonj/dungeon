//! RISC0 zkVM guest program for game state verification.
//!
//! This program runs inside the RISC0 zkVM and proves that executing
//! an action on a given state produces the expected result.
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
//!   1. GameEngine::execute(env, action) on mutable state
//!   2. Computes after_state and delta deterministically
//!   3. Commits public outputs to journal
//!
//! zkVM → Host:
//!   - Receipt (proof + journal):
//!     - after_state (computed result)
//!     - delta (state changes)
//!     - action (what was executed)
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

    // After execute(), state has been mutated to after_state
    // Commit public outputs to journal for verification
    // These outputs prove that executing the action produced this specific result
    env::commit(&state); // Complete resulting state (after execution)
    env::commit(&delta); // What changed
    env::commit(&action); // Which action was executed
}
