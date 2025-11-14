# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Arkworks Circuit Backend (Phase 2 + Phase 3 - In Progress)**:
  - **GameTransitionCircuit**: Complete circuit architecture for proving game actions with R1CS constraints
    - 650+ lines of comprehensive circuit implementation (`crates/zk/src/circuit/game_transition.rs`)
    - Action type encoding and polymorphic selector pattern
    - Public inputs: before_root, after_root, action_type, actor_id
    - Private witnesses: entity states, action parameters, Merkle proofs
    - Modular constraint structure supporting Move, MeleeAttack, and Wait actions
  - **R1CS Gadget Library** (`crates/zk/src/circuit/gadgets.rs`):
    - Poseidon hash gadgets for circuit-friendly hashing
    - Merkle path verification gadgets with conditional selection
    - Range check and bounds validation gadgets
    - Position validation (adjacency, bounds checking)
    - Safe arithmetic (add/subtract with overflow/underflow protection)
    - 350+ lines of reusable constraint gadgets
  - **StateTransition::from_delta()**: Fully implemented witness generation pipeline
    - Connects StateDelta → Merkle witnesses → circuit proofs
    - Computes before/after state roots
    - Generates Merkle proofs for changed entities only (sparse representation)
  - **Action Constraints** (detailed validation logic):
    - **Move**: Position delta validation (8 directions), bounds checking, adjacency verification
    - **MeleeAttack**: Actor liveness, stamina validation, range checking, damage calculation skeleton
    - **Wait**: Trivial no-op validation (state consistency via Merkle proofs)
  - **Circuit Design Documentation**: 120+ lines of inline documentation explaining:
    - Architecture rationale (field element encoding, action polymorphism, efficiency optimizations)
    - Implementation phases (Core Infrastructure → Action Constraints → Effect Constraints)
    - Public/private input structure and constraint flow

- **Terminal TTY Detection**: Improved terminal initialization with proper error handling
  - Uses `std::io::IsTerminal` (stable since Rust 1.70) instead of deprecated `atty` crate
  - Proper error logging and cleanup on failure
  - Warning messages when running without TTY
- **Comprehensive Codebase Documentation**: Added `docs/CODEBASE_SUMMARY.md` with detailed architecture overview

### Changed
- Enhanced terminal initialization in `crates/client/cli/src/presentation/terminal.rs`
  - Replaced deprecated `atty` crate with `std::io::IsTerminal`
  - Improved error handling with proper logging and cleanup
  - Fixed silent error swallowing with explicit error propagation

### Fixed
- **Removed deprecated `atty` dependency**: Replaced with stable `std::io::IsTerminal` trait
- **Improved error handling**: Terminal initialization now properly logs errors and cleans up on failure instead of silently continuing
- **File organization**: Moved `CODEBASE_SUMMARY.md` to `docs/` directory for better organization

### Implementation Notes

**Arkworks Circuit Implementation Status (Phase 2 → Phase 3 Transition)**:
- **Completed**:
  - Full circuit architecture and constraint design
  - Witness generation pipeline (StateDelta → Merkle proofs)
  - Gadget library for common operations
  - Action-specific constraint logic for all 3 action types
  - Comprehensive documentation and code comments
- **In Progress** (requires Arkworks R1CS API fixes):
  - Boolean gadget API (`.and()`, `.or()`, `.not()` → use trait methods)
  - FpVar API (`.value()`, `.cs()` → use correct accessors)
  - MerklePath struct field access (`.directions` → `.path_bits`)
  - BigInt conversion methods (`.to_bytes_le()` → use correct API)
- **TODO** (Phase 3 completion):
  - Full Poseidon gadget constraints (currently witness-only)
  - Target witness integration for attack validation
  - Occupancy and passability witnesses for movement
  - ArkworksProver integration with GameTransitionCircuit
  - Integration tests and performance benchmarks

## Previous Releases

See git history for previous changes.
