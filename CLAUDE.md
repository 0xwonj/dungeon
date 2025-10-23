# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A deterministic, ZK-provable, turn-based 2D dungeon RPG built with Rust. The game uses light ZK proofs with a single server authority for NPC actions, targeting EVM chains. The core principle is **functional core, imperative shell**: pure deterministic gameplay logic in `game-core`, with all I/O, crypto, and side effects isolated in `runtime`.

## Build & Test Commands

**Recommended: Use Just command runner for multi-backend workflows**

Install Just: `cargo install just`

### Quick Start with Just

```bash
# Fast development (stub backend - instant, no real proofs)
just build stub
just run stub
just test stub

# RISC0 backend (production, but skip guest builds for speed)
just build risc0-fast
just run risc0-fast

# RISC0 backend (full production build with real proofs)
just build risc0

# Set default backend via environment variable
export ZK_BACKEND=stub
just build   # automatically uses stub
just run
just test

# See all available commands
just --list
just help
```

### Common Just Commands

- `just build [backend]` - Build workspace with specified backend
- `just run [backend]` - Run CLI client
- `just test [backend]` - Run all tests
- `just lint [backend]` - Run clippy lints
- `just fmt` - Format all code
- `just check [backend]` - Run format check + lint + tests
- `just dev` - Fast development loop (format + lint + test with stub)
- `just pre-commit` - Pre-commit checks (recommended before committing)
- `just check-all` - Verify all backends compile
- `just tail-logs [session]` - Monitor client logs in real-time
- `just clean-data` - Clean save data and logs (with confirmation)

### Available ZK Backends

- `risc0` - RISC0 zkVM (production, real proofs, slow guest compilation)
- `risc0-fast` - RISC0 with `RISC0_SKIP_BUILD=1` (fast iteration, skip guest builds)
- `stub` - Stub prover (instant, no real proofs, testing only)
- `sp1` - SP1 zkVM (not implemented yet)
- `arkworks` - Arkworks circuits (not implemented yet)

### Direct Cargo Commands (without Just)

If you prefer not to use Just, you can use cargo directly:

```bash
# Stub backend (fast development)
cargo build --workspace --no-default-features --features stub
cargo run -p client-cli --no-default-features --features stub
cargo test --workspace --no-default-features --features stub

# RISC0 backend (default)
cargo build --workspace
RISC0_SKIP_BUILD=1 cargo build --workspace  # skip guest builds

# Lint and format
cargo lint  # uses default backend (risc0)
cargo fmt --all
```

### Environment Variables

- `ZK_BACKEND` - Set default backend for Just commands (risc0, risc0-fast, stub, sp1, arkworks)
- `RISC0_SKIP_BUILD=1` - Skip guest builds during cargo build (use for fast iteration)
- `RISC0_DEV_MODE=1` - Fast dev proofs (when running with real RISC0 backend)
- `RUST_LOG=info` - Logging level (use `info` or `warn` only - `debug` causes RISC0 to pollute TUI output)

## Architecture

### Core Crate Structure

```
crates/
├── game/
│   ├── core/        # Pure deterministic state machine (no I/O, crypto, or randomness)
│   └── content/     # Static content and fixtures exposed through oracle adapters
├── runtime/         # Public API (RuntimeHandle), orchestrator, workers, oracles, repositories
├── zk/              # Proving utilities reused by prover worker and off-chain services
├── client/
│   ├── bootstrap/   # Bootstrap utilities: configuration, oracle factories, runtime setup (crate: client-bootstrap)
│   └── frontend/
│       └── cli/     # Async terminal application, event loop, action provider
└── xtask/           # Development tools (cargo xtask pattern): tail-logs, clean-data
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

### client/bootstrap: Runtime Setup & Configuration

- **Crate name**: `client-bootstrap` (located at `crates/client/bootstrap/`)
- **Responsibility**: Bootstrap utilities for initializing runtime with proper configuration and oracles
- **Modules**:
  - `builder`: `RuntimeBuilder` builder pattern for assembling runtime with configuration
  - `config`: `CliConfig` and environment variable loading for client configuration
  - `oracles`: `OracleBundle`, `OracleFactory` trait, and `TestOracleFactory` implementation
- **Purpose**: Reusable setup code shared across CLI, UI, and other front-end crates
- **Exports**: `RuntimeBuilder`, `RuntimeSetup`, `CliConfig`, `OracleBundle`, `OracleFactory`

### client/frontend/cli: Terminal Interface

- **Responsibility**: Async terminal application with event loops and action providers
- **Architecture**: Consumes `client-bootstrap` for setup, subscribes to runtime events, renders state
- **Interaction**: Collects player commands, validates entity/turn alignment, forwards actions to runtime

## Code Organization Patterns

### Module Layout

- Use `mod.rs` for module re-exports or explicit module boundaries
- Export public API through crate root `lib.rs`
- NO inline unit tests in `#[cfg(test)]` modules - these slow down iteration and create maintenance overhead
- Integration tests only: Large-scale tests in `crates/<name>/tests/` directory that verify entire module behaviors

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

## Testing Policy

**IMPORTANT**: This project uses minimal testing to maximize development velocity.

### Testing Philosophy

- **Temporary unit tests OK during development**: You may write small `#[cfg(test)]` unit tests to verify logic while actively developing
- **Delete after verification**: Once the feature works and is verified, DELETE all small unit tests from the same commit/PR
- **No committed unit tests**: Small `#[cfg(test)]` modules create maintenance overhead and slow iteration - never commit them
- **Integration tests only in main branch**: Focus on high-level module behavior in `crates/<name>/tests/` directories
- **Test when complete**: Write integration tests for stable features after development is done

### Test Guidelines

- Integration tests verify entire workflows (runtime orchestration, action execution pipelines, event flows)
- Name tests after complete behaviors: `test_movement_workflow()`, `test_turn_scheduling_pipeline()`
- Use in-memory repositories for runtime tests to avoid I/O overhead
- Always run `cargo test --workspace` before pushing to catch integration issues
- Capture critical regression scenarios as focused integration tests

### When to Write Tests

**During Development (Temporary):**
- ✅ Write small unit tests to verify your logic while coding
- ✅ Use `#[cfg(test)]` modules to check edge cases during implementation
- ⚠️ **MUST DELETE** these temporary tests before committing/pushing

**For Permanent Tests (Integration Only):**
- ✅ After feature development is complete and API is stable
- ✅ For complex multi-crate integration scenarios
- ✅ To document critical edge cases or regression scenarios

**Never Write Tests For:**
- ❌ Individual functions or small helper methods (in committed code)
- ❌ Obvious logic or trivial getters/setters
- ❌ Code that is still actively changing

### Development Workflow

1. **Write feature code** + temporary unit tests for verification
2. **Verify** the feature works with `cargo test`
3. **Delete** all temporary `#[cfg(test)]` unit test modules
4. **Optionally add** integration tests in `tests/` directory for critical workflows
5. **Commit** only the feature code (and integration tests if added)

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

### What belongs in client/bootstrap (crate: client-bootstrap)

- Runtime configuration and bootstrap logic (`RuntimeBuilder`, `RuntimeSetup`)
- Configuration loading from environment variables (`CliConfig`)
- Oracle factory trait and implementations (`OracleFactory`, `TestOracleFactory`)
- Oracle bundle assembly (`OracleBundle`)
- Reusable setup utilities for all client front-ends

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
