# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A deterministic, ZK-provable, turn-based 2D dungeon RPG built with Rust. The game uses light ZK proofs with a single server authority for NPC actions, targeting EVM chains. The core principle is **functional core, imperative shell**: pure deterministic gameplay logic in `game-core`, with all I/O, crypto, and side effects isolated in `runtime`.

## Build & Test Commands

- Full workspace build: `cargo build --workspace`
- Run CLI client: `cargo run -p client-frontend-cli`
- All tests: `cargo test --workspace`
- Single test: `cargo test --workspace <test_name>`
- Specific crate tests: `cargo test -p runtime`
- Format code: `cargo fmt`
- Lint: `cargo clippy --workspace --all-targets --all-features`
- API documentation: `cargo doc --no-deps --open`

## Architecture

### Core Crate Structure

```
crates/
├── game/
│   ├── core/        # Pure deterministic state machine (no I/O, crypto, or randomness)
│   └── content/     # Static content and fixtures exposed through oracle adapters
├── runtime/         # Public API (RuntimeHandle), orchestrator, workers, oracles, repositories
├── zk/              # Proving utilities reused by prover worker and off-chain services
└── client/
    ├── core/        # Shared UX glue: config, messages, view models, oracle factories
    └── frontend/
        └── cli/     # Async terminal application, event loop, action provider
```

**Dependency flow**: `client`, `runtime`, `zk` → depend on `game/core` only. Never the reverse.

### game/core: Pure State Machine

- **Responsibility**: Deterministic rules engine, domain models, and validation schema
- **Entry point**: `GameEngine::execute(action) -> Result<State, Error>` and `GameEngine::prepare_next_turn()`
- **Action system**: All actions implement validation and application logic with deterministic state transitions
- **Environment**: Oracles for map data, item definitions, and game tables - core reads these but never implements them
- **Constraints**: No I/O, no randomness, no floating point, no time/clocks, no crypto operations
- **Exports**: All public types re-exported through `lib.rs`

### runtime: Imperative Shell

- **Responsibility**: Orchestrates game loop, implements oracles, manages persistence, coordinates workers, and emits game events
- **API**: Public surface consumed by clients (`RuntimeHandle`, `GameEvent`, `ActionProvider`)
- **Workers**: `SimulationWorker` (owns canonical `GameState`, processes turns and actions), `ProverWorker` (planned), `SubmitWorker` (planned)
- **Oracles**: Adapters exposing static game content (maps, NPC templates, loot tables) compatible with `game/core`
- **Repositories**: All storage behind traits (`StateRepository`, etc.) with in-memory implementations for testing
- **Message-driven**: Workers communicate via `tokio` channels, enabling concurrent pipelines

### client/core: UX Glue

- **Responsibility**: Shared client logic for configuration, message passing, view models, and oracle factories
- **Bootstrap**: Provides `RuntimeConfig` and `OracleBundle` construction for runtime initialization
- **Providers**: Implements `ActionProvider` for human input, AI/NPC scripts, or deterministic replay
- **Message-driven**: Translates front-end messages into runtime-facing actions

### client/frontend/cli: Terminal Interface

- **Responsibility**: Async terminal application with event loops and action providers
- **Architecture**: Consumes `client/core` abstractions, subscribes to runtime events, renders state
- **Interaction**: Collects player commands, validates entity/turn alignment, forwards actions to runtime

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

- All state mutations flow through the runtime's `SimulationWorker`, which delegates to `game/core`
- Actions are validated and executed deterministically within `game/core`
- Runtime emits `GameEvent` broadcasts for all state transitions (turn completion, action execution, failures)
- Clients consume events via `RuntimeHandle::subscribe_events()` for UI updates and feedback
- Turn scheduling is managed by the simulation worker via `prepare_next_turn()` calls

## Testing

- Unit tests: Fast, isolated, in `#[cfg(test)]` modules
- Name tests after observable behavior: `handles_turn_preparation()`, `validates_action_execution()`
- Integration tests: Cross-crate behavior in `tests/` subdirectories
- Always run `cargo test --workspace` before pushing
- Test runtime event flows and worker coordination with in-memory repositories
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

### What belongs in game/core

- State data structures and domain models
- Action validation and execution logic
- Deterministic state transitions
- Game rules (combat, movement, inventory, etc.)
- Pure functional operations (no I/O, no side effects)

### What belongs in game/content

- Static content: maps, NPC templates, loot tables
- Fixtures and test data
- Content exposed through oracle adapters

### What belongs in runtime

- Runtime orchestration and worker coordination
- Oracle implementations (backed by repositories)
- State persistence and checkpoint management
- Event broadcasting and subscription
- `RuntimeHandle` API for client interaction
- Proof generation coordination (ProverWorker - planned)
- Blockchain submission coordination (SubmitWorker - planned)

### What belongs in client/core

- Runtime configuration and bootstrap logic
- Oracle factory implementations
- `ActionProvider` implementations
- Message types and view models
- Client-side coordination logic

### What belongs in client/frontend/cli

- Async terminal application and event loop
- User input collection and validation
- Event consumption and display
- Player action provider implementation

## Security & Determinism Notes

- All randomness and side-effects are injected at the edges (providers, repositories)
- `game/core` must remain pure and deterministic (no I/O, no floating point, no time/clocks)
- Runtime workers communicate via message passing to maintain isolation
- State transitions are reproducible given the same action sequence
- Future: Proof generation will validate action sequences without re-running full engine in-circuit
- Future: Blockchain integration will verify proofs on-chain with minimal gas costs
