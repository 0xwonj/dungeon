//! SP1 zkVM guest program for game state verification.
//!
//! This program runs inside the SP1 zkVM and proves that executing
//! an action on a given state produces a specific result deterministically.
//!
//! Similar to RISC0 guest program but uses SP1 SDK APIs.

#![no_main]
#![no_std]

extern crate alloc;

sp1_zkvm::entrypoint!(main);

use game_core::{OracleSnapshot, SnapshotOracleBundle};

pub fn main() {
    // Read inputs from host in order
    // 1. Oracle snapshot (static game content)
    let oracle_snapshot: OracleSnapshot = sp1_zkvm::io::read();

    // 2. Game state before action execution
    let mut state: game_core::GameState = sp1_zkvm::io::read();

    // 3. Action to execute
    let action: game_core::Action = sp1_zkvm::io::read();

    // Create oracle bundle from snapshot
    let oracle_bundle = SnapshotOracleBundle::new(&oracle_snapshot);

    // Execute action deterministically using game-core logic
    // This is the CORE PROOF: GameEngine::execute() runs inside zkVM
    let mut engine = game_core::GameEngine::new(&mut state);

    engine
        .execute(oracle_bundle.as_env().into_game_env(), &action)
        .unwrap_or_else(|e| {
            // In zkVM guest, panics are converted to proof failures
            // Provide detailed error context for debugging proof generation issues
            panic!("Action execution failed in SP1 zkVM guest: {:?}", e)
        });

    // Commit public outputs to journal
    sp1_zkvm::io::commit(&state); // Resulting state after action execution
    sp1_zkvm::io::commit(&action); // Action that was executed
}

