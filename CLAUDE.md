# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A deterministic, ZK-provable, turn-based 2D dungeon RPG built with Rust. The game uses light ZK proofs with a single server authority for NPC actions, targeting EVM chains. The core principle is **functional core, imperative shell**: pure deterministic gameplay logic in `game-core`, with all I/O, crypto, and side effects isolated in `runtime`.

## Build & Test Commands

- Full workspace build: `cargo build --workspace`
- Run CLI client: `cargo run -p client-cli`
- Run UI client: `cargo run -p client-ui`
- All tests: `cargo test --workspace`
- Single test: `cargo test --workspace <test_name>`
- Specific crate tests: `cargo test -p game-core`
- Format code: `cargo fmt`
- Lint: `cargo clippy --workspace --all-targets`
- API documentation: `cargo doc --no-deps --open`

## Architecture

### Core Crate Structure

```
crates/
├── game-core/       # Pure deterministic state machine (no I/O, crypto, or randomness)
├── types/           # Shared data structures and commitments (minimal dependencies)
├── proofs/          # Proof system facade (zkVM or Plonkish backends)
├── server/          # NPC authority services
└── client/
    ├── runtime/     # Authoritative runtime (lib): oracles, witnesses, proofs, EVM submission
    ├── cli/         # Headless client (bin)
    └── ui/          # Bevy visualization (bin)
```

**Dependency flow**: `client`, `server`, `proofs` → depend on `game-core` only. Never the reverse.

### game-core: Pure State Machine

- **Responsibility**: Given `State`, `Env` (read-only oracles), and `Action`, compute next `State`
- **Entry point**: `reducer::step(env, state, action) -> Result<State, StepError>`
- **Action system**: All actions implement `ActionTransition` trait with `pre_validate`, `apply`, `post_validate` hooks
- **Command layer**: High-level `ActionCommand` trait converts ergonomic commands to canonical `Action` variants
- **Environment**: Oracles for map data (`MapOracle`), item definitions (`ItemOracle`), and game tables (`TablesOracle`) - core reads these but never implements them
- **Constraints**: No I/O, no randomness, no floating point, no time/clocks, no crypto operations
- **Exports**: All public types re-exported through `lib.rs`

### runtime: Imperative Shell

- **Responsibility**: Implements oracles, collects witnesses, generates proofs, submits to EVM, manages persistence and secrets
- **API**: Transport-agnostic ports (`RuntimeControl`, `RuntimeQuery`, `RuntimeEvents`)
- **Queues**: `sim_queue` → `proof_queue` → `submit_queue` with bounded backpressure
- **Workers**: Simulation worker (uses `GameEngine`), proof workers, submit worker (EVM transactions)
- **Repositories**: All storage behind traits (`StateRepo`, `MapRepo`, `NpcRepo`, `ProofRepo`, etc.)
- **Security boundary**: Secrets (wallet keys, proving keys, RPC tokens) never leave runtime

### Turn System

- Discrete shared turns with fixed phases: Player Action → NPC Actions → End-of-Turn (EoT) ticks
- Actions are atomic (all-or-nothing)
- System uses deterministic entity scheduling with tie-breaking rules
- See `docs/turn_system.md` and `docs/game_design.md` for detailed mechanics

## Code Organization Patterns

### Module Layout

- Use `mod.rs` for module re-exports or explicit module boundaries
- Export public API through crate root `lib.rs`
- Co-locate tests in `#[cfg(test)]` modules next to implementation
- Integration tests go in `crates/<name>/tests/` directory

### Naming

- Functions/modules/files: `snake_case`
- Structs/enums/traits: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- 4-space indentation, trailing commas

### State & Actions

- All state mutations flow through `GameEngine::execute`
- Each `ActionKind` has a corresponding implementation of `ActionTransition`
- Commands provide ergonomic builder interface but always materialize to canonical `Action` for determinism
- Witness deltas track which state/env fields were accessed for proof generation
- `GameEngine` also manages turn scheduling through integrated `TurnSystem` access

## Testing

- Unit tests: Fast, isolated, in `#[cfg(test)]` modules
- Name tests after observable behavior: `handles_empty_party()`, `rejects_invalid_move()`
- Integration tests: Cross-crate behavior in `tests/` subdirectories
- Always run `cargo test --workspace` before pushing
- Property-based tests for randomized action sequences ensure determinism
- Capture regression scenarios from bugs as new test cases

## Commits

Use Conventional Commits format:
- `feat: add turn scheduling system`
- `fix: correct movement validation`
- `refactor: extract action command builder`
- `test: add movement edge cases`
- `docs: update runtime architecture`

Keep commits scoped to single concerns. Include doc updates when behavior changes.

## Important Design Boundaries

### What belongs in game-core

- State data structures
- Action validation and application logic
- Turn/phase mechanics
- Combat, movement, status, inventory rules
- Deterministic entity behavior
- Field order for commitments (but not hashing)

### What belongs in runtime

- Oracle implementations (backed by repos/caches)
- Witness transcript assembly
- Commitment hashing (using core's field order)
- Proof generation and verification
- NPC order fetching and signature validation
- EVM transaction submission
- State persistence and journaling
- Configuration and secrets management

### What belongs in client/ui

- Bevy rendering and ECS systems
- User input handling
- HUD and visual feedback
- Snapshot consumption and display

### What belongs in client/cli

- Headless subcommands: `play`, `prove`, `submit`, `inspect`, `bench`
- Automation and scripting interface
- JSON logging for CI/CD

## Security Notes

- Validate all inputs at oracle ingestion (Merkle proofs, signatures) before data enters repos
- Never include floating point or nondeterministic operations in game-core
- Replay protection via monotonic turn index + nullifier registry
- All cross-component data exchange uses commitments; plaintext is untrusted
- Record/replay capability for determinism verification
