# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Dungeon** is a deterministic, ZK-provable, turn-based 2D roguelike RPG built with Rust. The game demonstrates how zero-knowledge proofs enable **fairness without authority** and **secrecy without deceit** — every action, roll, and AI move can be cryptographically proven valid without revealing hidden information.

### Core Philosophy

- **Verifiable Computation**: ZK proofs ensure honest gameplay while preserving mystery (hidden maps, enemy intent, RNG seeds)
- **Systems Over Scripts**: Emergent gameplay from interacting systems (AI, factions, procedural generation) rather than authored narratives
- **Functional Core, Imperative Shell**: Pure deterministic logic in `game-core`, all I/O and side effects isolated in `runtime`
- **Off-chain Play, On-chain Trust**: Rich local gameplay with succinct proof verification for legitimacy

### Technical Architecture

**Multi-Backend ZK System**: Supports RISC0 zkVM (production), SP1 zkVM (production), and stub prover (testing), with planned Arkworks support

**Three-Layer Design**:
1. **game-core**: Pure state machine with 3-phase action pipeline (pre_validate → apply → post_validate)
   - 5-layer stat system, oracle pattern, delta tracking, available actions query
2. **runtime**: Orchestration layer with workers, event bus, hooks, AI, and persistence
   - Topic-based events, utility AI (Intent → Tactic → Action), post-execution hooks
3. **client**: Multi-frontend architecture with shared UX primitives
   - Terminal UI with examine mode, cursor system, targeting

**Current Features**:
- ✅ Deterministic turn scheduling with entity activation
- ✅ Action system (Move, Attack, UseItem, Interact, Wait + system actions)
- ✅ Utility-based AI with trait composition (Species × Archetype × Faction × Temperament)
- ✅ State persistence (checkpoints, action logs, event logs, proof indices)
- ✅ CLI interface with examine panel and tactical targeting
- ✅ Oracle system for static content (maps, items, NPCs, loot tables)
- ✅ Stat system with unified bonus calculations
- 🚧 ZK proof generation (ProverWorker infrastructure in place)
- 📅 On-chain verification (planned)

**Development Status**: Early-stage prototype with active iteration on core gameplay systems. Architecture is stabilizing, but expect breaking changes as we refine the proof generation pipeline and blockchain integration.

## Build & Test Commands

**Recommended: Use Just command runner for multi-backend workflows**

Install Just: `cargo install just`

### Quick Start with Just

```bash
# Fast development (stub backend - instant, no real proofs)
just build stub
just run stub
just test stub

# Fast mode (no proof generation, no persistence)
just run-fast stub

# RISC0 backend (full production build with real proofs)
just build risc0
just run risc0

# SP1 backend (alternative production backend, all platforms)
just build sp1
just run sp1

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
- `just run [backend]` - Run CLI client (no blockchain)
- `just run-sui [backend]` - Run CLI client with Sui blockchain integration
- `just run-fast [backend]` - Run in fast mode (no proof generation, no persistence)
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

- `risc0` - RISC0 zkVM (production, real proofs, Linux x86_64 only for Groth16)
- `sp1` - SP1 zkVM (production, real proofs, all platforms including macOS)
- `stub` - Stub prover (instant, no real proofs, testing only)
- `arkworks` - Arkworks circuits (not implemented yet)

### Direct Cargo Commands (without Just)

If you prefer not to use Just, you can use cargo directly:

```bash
# CLI only (no blockchain)
cargo run -p dungeon-client --no-default-features --features "cli,stub"

# CLI + Sui blockchain
cargo run -p dungeon-client --no-default-features --features "cli,sui,sp1"

# Test workspace
cargo test --workspace --no-default-features --features stub

# Lint and format
cargo clippy --workspace --all-targets --no-default-features --features stub
cargo fmt --all
```

### Environment Variables

#### General Configuration
- `ZK_BACKEND` - Set default backend for Just commands (risc0, stub, sp1, arkworks)
- `RUST_LOG=info` - Logging level (use `info` or `warn` only - `debug` causes RISC0 to pollute TUI output)
- `ENABLE_ZK_PROVING=false` - Disable proof generation entirely (fast mode)
- `ENABLE_PERSISTENCE=false` - Disable state/action persistence (fast mode)

#### RISC0 Specific
- `RISC0_SKIP_BUILD=1` - Skip guest builds during cargo build (use for fast iteration)
- `RISC0_DEV_MODE=1` - Fast dev proofs (when running with real RISC0 backend)

#### SP1 Specific
- `SP1_PROVER` - SP1 prover mode (cpu, network, cuda, mock)
  - `cpu` (default): Local CPU proving (slow, high memory)
  - `network`: Succinct Prover Network (fast, requires API key)
  - `cuda`: Local CUDA GPU proving (fastest, requires NVIDIA GPU)
  - `mock`: Mock proving for testing (instant, no real proofs)
- `SP1_PROOF_MODE` - SP1 proof type (compressed, groth16, plonk)
  - `compressed` (default): Compressed STARK (~4-5MB, off-chain)
  - `groth16`: Groth16 SNARK (~260 bytes, on-chain, Sui compatible)
  - `plonk`: PLONK SNARK (~868 bytes, on-chain, no trusted setup)
- `NETWORK_PRIVATE_KEY` - Private key for SP1 Prover Network (required for network mode)
- `NETWORK_RPC_URL` - Custom RPC endpoint for SP1 Prover Network (optional, defaults to mainnet)

## Architecture

### Core Crate Structure

```
crates/
├── game/
│   ├── core/        # Pure deterministic state machine (no I/O, crypto, or randomness)
│   └── content/     # Static content and fixtures exposed through oracle adapters
├── runtime/         # Public API (RuntimeHandle), orchestrator, workers, oracles, repositories
├── zk/              # Proving utilities reused by prover worker and off-chain services
├── client/          # Composable binary (dungeon-client)
│   ├── bootstrap/   # Runtime initialization (proving, persistence, oracles)
│   ├── frontend/
│   │   ├── core/    # UI primitives (events, messages, view models) - client-frontend-core
│   │   └── cli/     # Terminal UI library - client-frontend-cli
│   └── blockchain/
│       ├── core/    # Blockchain abstraction (traits) - client-blockchain-core
│       └── sui/     # Sui implementation - client-blockchain-sui
└── xtask/           # Development tools (cargo xtask pattern): tail-logs, clean-data
```

**Dependency flow**: `client`, `runtime`, `zk` → depend on `game/core` only. Never the reverse.

### zk: Zero-Knowledge Proof Backends

The `crates/zk` crate provides a unified interface for multiple zkVM backends with feature-gated compilation:

**Backend Architecture:**
- **RISC0 zkVM** (`feature = "risc0"`): Production-ready zkVM with mature tooling
  - Guest program: `methods/risc0/state-transition/` (RISC0-specific APIs)
  - Groth16 compression: Linux x86_64 only, ~200 bytes
  - Requires: Docker for Groth16, RISC0 toolchain
  - Status: ✅ Fully implemented and tested

- **SP1 zkVM** (`feature = "sp1"`): Alternative production zkVM with cross-platform support
  - Guest program: `methods/sp1/state-transition/` (SP1-specific APIs)
  - Groth16 compression: All platforms (macOS, Linux, Windows), ~260 bytes
  - PLONK compression: All platforms, ~868 bytes, no trusted setup required
  - Requires: SP1 toolchain (`sp1up`)
  - Status: ✅ Fully implemented, identical logic to RISC0

- **Stub Prover** (`feature = "stub"`): Testing-only backend for fast iteration
  - No real proofs generated (instant execution)
  - Same interface as production backends
  - Status: ✅ Used for development

**Proof Structure (Identical Across Backends):**
Both RISC0 and SP1 use the same 168-byte public values structure:
```text
1. oracle_root       (32 bytes) - Commitment to static game content
2. seed_commitment   (32 bytes) - Commitment to RNG seed
3. prev_state_root   (32 bytes) - State hash before execution
4. actions_root      (32 bytes) - Commitment to action sequence
5. new_state_root    (32 bytes) - State hash after execution
6. new_nonce         (8 bytes)  - Action counter after execution
Total: 168 bytes
```

**Two-Stage Verification Model:**
- **Stage 1 (On-chain):** Groth16/PLONK proof verification with SHA-256 digest
- **Stage 2 (On-chain):** Public values content extraction and validation

**Guest Program Design:**
- Core execution logic is identical between RISC0 and SP1
- Only I/O APIs differ (`risc0_zkvm::guest::env` vs `sp1_zkvm::io`)
- Separate directories for clear separation: `methods/risc0/` and `methods/sp1/`
- Both use `commit_slice()` pattern to avoid serialization overhead
- Optimizations: Delta tracking disabled in zkVM mode (via `zkvm` feature flag)

**Backend Selection:**
```rust
// Feature flags in Cargo.toml (mutually exclusive)
[features]
default = ["risc0"]
risc0 = ["zkvm", "dep:risc0-zkvm", ...]
sp1 = ["zkvm", "dep:sp1-sdk", ...]
stub = ["zkvm"]
```

**Host-Side Prover Interface:**
```rust
pub trait Prover {
    fn prove(&self, start: &GameState, actions: &[Action], end: &GameState) -> Result<ProofData>;
    fn verify(&self, proof: &ProofData) -> Result<bool>;
}

// Unified proof data structure
pub struct ProofData {
    bytes: Vec<u8>,           // Serialized proof
    backend: ProofBackend,    // Risc0, Sp1, or Stub
    journal: Vec<u8>,         // 168-byte public values
    journal_digest: [u8; 32], // SHA-256(journal)
}
```

**When to Choose Which Backend:**
- **RISC0**: Mature ecosystem, extensive documentation, proven in production
- **SP1**: Cross-platform Groth16/PLONK, faster iteration on macOS, PLONK trustless setup
- **Stub**: Development and testing only (instant, no real proofs)

### game/core: Pure State Machine

- **Responsibility**: Deterministic rules engine, domain models, state management, and pure action execution
- **Architecture**: Three-phase action pipeline (pre_validate → apply → post_validate) with oracle-based environment
- **Core Modules**:
  - `action`: Action definitions and transitions (`Action`, `ActionTransition`, `CharacterActionKind`, `SystemActionKind`, `get_available_actions`)
  - `engine`: Action execution pipeline (`GameEngine::execute`, `ExecuteError`, actor validation, delta generation)
  - `state`: Canonical game state (`GameState`, `EntitiesState`, `WorldState`, `TurnState`, `StateDelta`)
  - `env`: Oracle trait definitions (`Env`, `GameEnv`, `MapOracle`, `ItemOracle`, `ActorOracle`, `TablesOracle`, `ConfigOracle`)
  - `stats`: Layered stat system (5 layers: Core → Derived → Speed → Modifiers → Resources with unified bonus calculation)
  - `config`: Game configuration (`GameConfig`)
- **Action System**:
  - Character actions: `Move`, `Attack`, `UseItem`, `Interact`, `Wait`
  - System actions: `PrepareTurn`, `ActionCost`, `Activation`
  - Actor validation: System actions from `EntityId::SYSTEM`, character actions from `state.turn.current_actor`
  - Available actions query: `get_available_actions(state, env, actor)` for AI and UI
- **State Management**:
  - Entity ID allocation with reserved IDs (0 = PLAYER, u32::MAX = SYSTEM)
  - Delta tracking: `StateDelta::from_states` captures all changes (skipped in zkvm mode)
  - Nonce increment after each successful execution
  - Tile view merging: static map + runtime occupancy
- **Stat System**:
  - 5-layer architecture (Core → Derived → Speed → Modifiers → Resources)
  - Unified bonus calculation: `Final = clamp((base + Flat) × (1 + %Inc/100) × More × Less, min, max)`
  - Trait-based design: `StatLayer` trait for Base → Bonuses → Final pattern
  - Snapshot consistency: All values locked at action initiation
- **Oracle Pattern**: Core reads oracles but never implements them (implementations live in runtime)
- **Constraints**: No I/O, no randomness, no floating point, no time/clocks, no crypto operations
- **Exports**: All public types re-exported through `lib.rs` (60+ types including actions, state, env, stats)

### runtime: Imperative Shell

- **Responsibility**: Orchestrates game loop, coordinates workers, manages persistence, implements oracles, and provides AI systems
- **Architecture**: Message-driven worker system with `tokio` channels, topic-based event bus, and flexible hook system
- **Core Modules**:
  - `api`: Public surface (`RuntimeHandle`, `ActionProvider`, `ProviderRegistry`, error types)
  - `runtime`: Runtime orchestrator with builder pattern (`Runtime`, `RuntimeBuilder`, config types)
  - `workers`: Background task coordination (`SimulationWorker`, `ProverWorker`, `PersistenceWorker`)
  - `events`: Topic-based event bus (`EventBus`, `Topic`, `GameStateEvent`, `ProofEvent`)
  - `hooks`: Post-execution hook system for runtime orchestration (`HookRegistry`, `PostExecutionHook`, `ActionCostHook`, `ActivationHook`)
  - `providers`: AI implementations (`UtilityAiProvider` with 3-layer utility scoring: Intent → Tactic → Action)
  - `oracle`: Oracle adapters (`OracleManager`, `MapOracleImpl`, `ActorOracleImpl`, `ItemOracleImpl`, `TablesOracleImpl`, `ConfigOracleImpl`)
  - `repository`: Persistence layer with trait-based storage (`StateRepository`, `CheckpointRepository`, `ActionLogReader`, `EventRepository` with file and in-memory implementations)
  - `scenario`: Entity placement and game initialization (`Scenario`, `EntityPlacement`, `EntityKind`)
- **Workers**: `SimulationWorker` (canonical state, action execution), `ProverWorker` (ZK proof generation), `PersistenceWorker` (state/event/proof persistence)
- **Event System**: Topic-based subscriptions (GameState, Proof, Turn topics) for efficient event routing
- **Hook System**: Post-execution hooks with priority ordering, chaining support, and criticality levels (Critical, Important, Optional)
- **AI System**: 3-layer utility-based AI (Intent → Tactic → Action) using TraitProfile composition (Species × Archetype × Faction × Temperament)

### client/frontend/core: Cross-Frontend Primitives

- **Crate name**: `client-frontend-core` (located at `crates/client/frontend/core/`)
- **Responsibility**: Shared UX glue for presenting the game across different frontend implementations
- **Modules**:
  - `event`: Event handling and consumption (`EventConsumer`, `EventImpact`)
  - `frontend`: Frontend abstraction layer (FrontendApp trait, message routing)
  - `message`: Message logging and formatting
  - `targeting`: Targeting system for tactical interactions
  - `view_model`: View models for rendering game state
  - `config`: Frontend configuration (`FrontendConfig` with channel and message settings)
- **Purpose**: Reusable presentation logic shared across CLI, GUI, and other frontend crates
- **Exports**: `EventConsumer`, `EventImpact`, `FrontendConfig`, frontend abstractions, view models

### client/bootstrap: Runtime Setup & Configuration

- **Crate name**: `client-bootstrap` (located at `crates/client/bootstrap/`)
- **Responsibility**: Bootstrap utilities for initializing runtime with proper configuration and oracles
- **Modules**:
  - `builder`: `RuntimeBuilder` builder pattern for assembling runtime with configuration
  - `config`: `CliConfig` and environment variable loading for client configuration
  - `oracles`: `OracleBundle`, `OracleFactory` trait, and `ContentOracleFactory` implementation
- **Purpose**: Reusable setup code shared across CLI, UI, and other front-end crates
- **Exports**: `RuntimeBuilder`, `RuntimeSetup`, `CliConfig`, `OracleBundle`, `OracleFactory`, `ContentOracleFactory`

### client/frontend/cli: Terminal Interface

- **Crate name**: `client-frontend-cli` (located at `crates/client/frontend/cli/`)
- **Responsibility**: Async terminal application with cursor system, examine UI, and tactical interactions
- **Architecture**: Consumes `client-frontend-core` and `client-bootstrap`, subscribes to runtime events, renders state
- **Modules**:
  - `app`: Main application loop and state management (`CliApp`, `CliAppBuilder`)
  - `config`: CLI-specific configuration (`CliConfig`)
  - `cursor`: Cursor system for examine mode and targeting
  - `input`: User input handling and command parsing
  - `presentation`: Terminal rendering and UI components
  - `logging`: Platform-specific log directory setup
- **Interaction**: Collects player commands, validates entity/turn alignment, forwards actions to runtime
- **Exports**: `CliApp`, `CliAppBuilder`, `CliConfig`, `FrontendConfig`, `RuntimeConfig`, `setup_logging`

### client/blockchain: Blockchain Integration Layer

The blockchain subsystem provides a pluggable abstraction for submitting ZK proofs to various blockchain networks. It follows the same trait-based pattern as the `zk` crate, allowing multiple blockchain implementations behind a common interface.

#### client/blockchain/core: Blockchain Abstraction

- **Crate name**: `client-blockchain-core` (located at `crates/client/blockchain/core/`)
- **Responsibility**: Trait definitions and types for blockchain-agnostic proof submission
- **Architecture**: Trait-based abstraction similar to `zk::Prover` pattern
- **Core Modules**:
  - `traits`: Blockchain client trait definitions
  - `types`: Common blockchain types (session, transaction, proof metadata)
  - `mock`: Mock implementation for testing
- **Trait Hierarchy**:
  ```rust
  // Core trait composition
  pub trait BlockchainClient: ProofSubmitter + SessionManager + Send + Sync {
      async fn list_pending_proofs(&self) -> Result<Vec<ProofMetadata>>;
      async fn submit_all_pending(&self, session_id: &SessionId) -> Result<Vec<SubmissionResult>>;
      fn config(&self) -> &dyn BlockchainConfig;
      async fn health_check(&self) -> Result<()>;
  }

  // Proof submission operations
  pub trait ProofSubmitter {
      async fn submit_proof(&self, session_id: &SessionId, proof_data: ProofData) -> Result<SubmissionResult>;
      async fn query_transaction(&self, tx_id: &TransactionId) -> Result<TransactionStatus>;
  }

  // Session lifecycle management
  pub trait SessionManager {
      async fn create_session(&self, initial_state_root: [u8; 32]) -> Result<SessionId>;
      async fn get_session_state(&self, session_id: &SessionId) -> Result<SessionState>;
      async fn finalize_session(&self, session_id: &SessionId) -> Result<TransactionId>;
  }

  // Blockchain-specific configuration
  pub trait BlockchainConfig {
      fn network_name(&self) -> &str;
      fn rpc_url(&self) -> &str;
      fn validate(&self) -> Result<(), String>;
  }
  ```
- **Common Types**:
  - `SessionId`: Opaque session identifier
  - `TransactionId`: Blockchain transaction hash/ID
  - `TransactionStatus`: Pending, Confirmed, or Failed
  - `ProofMetadata`: Proof tracking info (session, nonce, timestamp)
  - `SubmissionResult`: Transaction result with ID and status
- **Design Principles**:
  - Blockchain-agnostic abstractions (no Sui/Ethereum-specific types)
  - Async-first API using `#[async_trait]`
  - Explicit session management for multi-proof workflows
  - Network-based errors with context preservation

#### client/blockchain/sui: Sui Implementation

- **Crate name**: `client-blockchain-sui` (located at `crates/client/blockchain/sui/`)
- **Responsibility**: Sui-specific blockchain client implementation
- **Architecture**: Implements `BlockchainClient` trait using Sui SDK
- **Core Modules**:
  - `client`: `SuiBlockchainClient` implementation
  - `config`: Sui network configuration (`SuiConfig`, `SuiNetwork`)
  - `converter`: Proof format conversion for Sui Move contracts
  - `session`: Session state management and transaction building
- **Sui Networks**:
  - `Mainnet`: Production Sui network (`https://fullnode.mainnet.sui.io:443`)
  - `Testnet`: Testing network (`https://fullnode.testnet.sui.io:443`)
  - `Local`: Local development network (`http://127.0.0.1:9000`)
- **Configuration**:
  ```rust
  pub struct SuiConfig {
      pub network: SuiNetwork,
      pub rpc_url: Option<String>,       // Custom RPC override
      pub package_id: Option<String>,    // Deployed game contract
      pub gas_budget: u64,               // Transaction gas budget (MIST)
  }

  // Environment variable loading
  SUI_NETWORK=testnet        # Network selection (default: testnet)
  SUI_RPC_URL=<custom-url>   # Custom RPC endpoint
  SUI_PACKAGE_ID=0x...       # Game package ID
  SUI_GAS_BUDGET=100000000   # Gas budget in MIST (default: 0.1 SUI)
  ```
- **Proof Submission Flow**:
  1. Convert `ProofData` to Sui format via `SuiProofConverter`
  2. Build Move transaction calling verification contract
  3. Estimate gas and submit transaction
  4. Track transaction status until confirmation
- **Session Management**:
  - Sessions map to on-chain game state objects
  - Session creation initializes state with `initial_state_root`
  - Proof submissions update session state progressively
  - Session finalization locks the session and returns final transaction
- **Status**: Placeholder implementation (Sui SDK integration pending)

#### Future Blockchain Implementations

- **Ethereum**: Planned support for EVM-based chains (Ethereum, Polygon, etc.)
- **Feature Flags**: Each blockchain backend is feature-gated for optional compilation
- **Composability**: Frontends can be built with or without blockchain integration

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

- State data structures (`GameState`, `EntitiesState`, `WorldState`, `TurnState`)
- Action definitions and transition traits (`Action`, `ActionTransition`, character/system action kinds)
- Three-phase action pipeline (pre_validate, apply, post_validate)
- Oracle trait definitions (`MapOracle`, `ItemOracle`, `ActorOracle`, `TablesOracle`, `ConfigOracle`)
- Stat system (5-layer architecture with bonus calculations)
- Game rules (combat, movement, inventory, interactions)
- State delta tracking (`StateDelta` with field-level change detection)
- Entity ID allocation and management
- Available actions query system
- Deterministic state transitions
- Pure functional operations (no I/O, no side effects, no randomness)

### What belongs in game/content

- Static content: maps, NPC templates, loot tables
- Fixtures and test data
- Content exposed through oracle adapters

### What belongs in runtime

- Runtime orchestration and builder pattern (`Runtime`, `RuntimeBuilder`)
- Worker coordination (`SimulationWorker`, `ProverWorker`, `PersistenceWorker`)
- Topic-based event bus implementation (`EventBus`, topic subscriptions)
- Post-execution hook system (`HookRegistry`, hook implementations)
- Oracle implementations (`OracleManager`, oracle adapters for game-core traits)
- AI provider implementations (`UtilityAiProvider`, utility scoring functions)
- Repository traits and implementations (file-based and in-memory storage)
- Persistence layer (state, checkpoints, action logs, event logs, proof indices)
- Scenario system (entity placement, game initialization)
- Provider registry (entity-to-provider mapping)
- `RuntimeHandle` API for client interaction
- Configuration types (`RuntimeConfig`, `PersistenceSettings`, `ProvingSettings`)

### What belongs in client/core (crate: client-core)

- Cross-frontend presentation primitives
- Event handling and consumption logic
- Frontend abstraction layer (FrontendApp trait)
- Message logging and formatting
- Targeting system and tactical UI helpers
- View models for rendering game state
- Reusable UX logic shared across all frontends

### What belongs in client/bootstrap (crate: client-bootstrap)

- Runtime configuration and bootstrap logic (`RuntimeBuilder`, `RuntimeSetup`)
- Configuration loading from environment variables (`CliConfig`)
- Oracle factory trait and implementations (`OracleFactory`, `ContentOracleFactory`)
- Oracle bundle assembly (`OracleBundle`)
- Reusable setup utilities for all client front-ends

### What belongs in client/cli (crate: client-cli)

- Async terminal application and event loop
- User input collection and validation
- Cursor system and examine mode
- Terminal rendering and UI components
- Event consumption and display
- Player action provider implementation

## Security & Determinism Notes

- All randomness and side-effects are injected at the edges (providers, repositories)
- `game/core` must remain pure and deterministic (no I/O, no floating point, no time/clocks)
- Runtime workers communicate via message passing to maintain isolation
- State transitions are reproducible given the same action sequence
- Future: Proof generation will validate action sequences without re-running full engine in-circuit
- Future: Blockchain integration will verify proofs on-chain with minimal gas costs
