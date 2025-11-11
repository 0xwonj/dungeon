# Dungeon Codebase Summary

> **Generated**: Comprehensive analysis of the entire Dungeon codebase structure, architecture, and implementation status.

## Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture Overview](#architecture-overview)
3. [Workspace Structure](#workspace-structure)
4. [Core Crates](#core-crates)
5. [Key Systems](#key-systems)
6. [Implementation Status](#implementation-status)
7. [Design Patterns](#design-patterns)
8. [Technology Stack](#technology-stack)
9. [Development Workflow](#development-workflow)

---

## Project Overview

**Dungeon** is a verifiable roguelike RPG built on zero-knowledge proofs (ZKPs). It models gameplay as a deterministic finite state machine where every action can be proven valid without revealing hidden information.

### Core Philosophy

- **Determinism First**: All game logic is reproducible for ZK proof generation
- **Action-Validity Proofs**: Prove validation phases (pre/post) rather than full execution
- **Systemic Emergence**: Interactive systems produce emergent gameplay over scripts
- **Fairness Without Authority**: Cryptographic proofs ensure honest gameplay
- **Privacy-Preserving**: Prove validity without revealing spoilers (hidden maps, enemy intent)

### Project Status

- **Stage**: Early-stage prototype with active development
- **Rust Edition**: 2024 edition
- **Primary Language**: Rust
- **Async Runtime**: Tokio

---

## Architecture Overview

### High-Level Flow

```
Frontend (CLI/UI) 
    ↓
Client Core (bootstrap, messages, view models)
    ↓
Runtime (orchestrator, workers, API)
    ↓
Game Core (deterministic engine, actions, state)
    ↓
Game Content (static data: maps, items, NPCs)
```

### Dependency Graph

- **Frontends** → **Client Core** → **Runtime**
- **Runtime** → **Game Core** + **Game Content** + **ZK**
- **Game Core** → **Behavior Tree** (AI decision making)
- **ZK** → **Game Core** (for proof generation)

---

## Workspace Structure

### Root Workspace Members

```
crates/
├── client/              # Client-side crates
│   ├── bootstrap/       # Runtime bootstrap and configuration
│   ├── core/            # Shared client primitives (messages, view models, services)
│   └── cli/           # Terminal UI frontend
├── game/
│   ├── core/          # Pure deterministic game engine (no I/O, no randomness)
│   └── content/       # Static game content (maps, items, NPCs, loot tables)
├── runtime/           # Orchestrator: API, workers, oracles, repositories, hooks
├── zk/                # Zero-knowledge proof generation utilities
├── behavior-tree/     # Lightweight behavior tree library for AI
└── xtask/             # Build automation and development tools
```

---

## Core Crates

### 1. `game-core` - Deterministic Game Engine

**Purpose**: Pure, deterministic state machine with no side effects.

**Key Modules**:
- **`engine/`**: `GameEngine` orchestrates action execution pipeline
- **`action/`**: Action system with three-phase validation (pre-validate, execute, post-validate)
- **`state/`**: Hierarchical state (`GameState`, `StateDelta`, entity tracking)
- **`combat/`**: Combat resolution (hit chance, damage calculation)
- **`stats/`**: Stat system (core stats, derived stats, bonuses, modifiers)
- **`inventory/`**: Inventory management
- **`env/`**: Environment traits (oracles for map, items, NPCs, tables, config)

**Action Types**:
- **Character Actions**: `Move`, `Attack`, `UseItem`, `Interact`, `Wait`
- **System Actions**: `PrepareTurn`, `ActionCost`, `Activation`

**Key Types**:
- `GameEngine`: Executes actions through validation pipeline
- `GameState`: Root state container (TurnState, EntitiesState, WorldState)
- `StateDelta`: Tracks all state mutations for replay/proof generation
- `ActionTransition`: Trait for action validation and execution

**Design Principles**:
- Zero I/O (pure functions)
- No randomness (injected via `RngOracle`)
- Deterministic (same input = same output)
- Stateless functions (operate on provided state snapshots)

---

### 2. `runtime` - Orchestrator and Public API

**Purpose**: Coordinates workers, exposes runtime API, manages lifecycle.

**Key Modules**:
- **`api/`**: Public surface (`RuntimeHandle`, `GameEvent`, `ActionProvider`)
- **`runtime.rs`**: Main orchestrator (`Runtime`, `RuntimeBuilder`)
- **`workers/`**: Background tasks
  - `SimulationWorker`: Owns canonical `GameState`, processes turns/actions
  - `ProverWorker`: ✅ **Fully implemented** ZK proof generation with metrics
  - `PersistenceWorker`: ✅ **Fully implemented** coordinated state/event/action log persistence
  - `ProofMetrics`: Lock-free atomic metrics tracking
- **`hooks/`**: Post-execution hook system for side effects
- **`oracle/`**: Adapters exposing static game content
- **`repository/`**: Traits and implementations for state persistence
- **`events/`**: Topic-based event bus with lag detection
- **`providers/`**: Action provider implementations (AI, human input)
- **`utils/`**: State hashing utilities for verification

**Key Features**:
- Async message-driven architecture (tokio channels)
- Topic-based event broadcasting
- Hook chaining with cycle prevention
- Pluggable action providers (human, AI, replay)
- Multiple repository backends (in-memory, file-based)

**Hook System**:
- Priority-based execution
- Chainable hooks (`next_hook_names()`)
- Criticality levels (Critical, Important, Optional)
- Multiple actions per hook
- Entry hooks (trigger without generating actions)

---

### 3. `game-content` - Static Game Data

**Purpose**: Data-driven content definitions (NPCs, maps, items, loot tables).

**Key Modules**:
- **`loaders/`**: RON/TOML deserialization loaders
- **`traits/`**: NPC behavioral trait system
- **`data/`**: Content files (RON maps, TOML configs)

**Content Types**:
- Actor templates (NPC definitions)
- Map layouts (terrain, props)
- Item catalogs (weapons, armor, consumables)
- Loot tables
- Trait profiles (NPC behaviors)

---

### 4. `zk` - Zero-Knowledge Proof Generation

**Purpose**: Unified interface for different proving backends.

**Supported Backends**:
- **`risc0`** (default): RISC0 zkVM production backend
- **`stub`**: Dummy prover for testing (instant, no proofs)
- **`sp1`**: SP1 zkVM (not implemented)
- **`arkworks`**: Custom circuit proving (future)

**Key Modules**:
- **`prover/`**: Universal prover interface
- **`zkvm/`**: zkVM-based backend implementations
- **`oracle/`**: Oracle snapshot serialization
- **`circuit/`**: Custom circuits (future, arkworks feature)

**Proof Strategy**:
- Action-validity proofs (pre/post validation)
- Proves: executing `action` on `before_state` produces `after_state`
- State consistency checking (detects determinism bugs)
- Witness generation from `StateDelta` (future: Merkle commitments)
- Oracle snapshot serialization for proof generation

---

### 5. `client-core` - Shared Client Primitives

**Purpose**: Cross-frontend primitives (messages, view models, services).

**Key Modules**:
- **`view_model/`**: Transform `GameState` into presentation-friendly structures
- **`services/`**: Targeting strategies, view model updaters
- **`event/`**: Event consumption utilities
- **`message/`**: Client-runtime message types

**Services**:
- Targeting system (nearest, fastest, lowest health, threat-based)
- View model updaters (scope-based updates)
- Event consumers (impact-based filtering)

---

### 6. `client-cli` - Terminal Interface

**Purpose**: Full-featured async terminal application.

**Key Features**:
- **Examine Mode**: Cursor-based exploration (`E` key)
- **Cursor System**: Manual movement, automatic targeting
- **Action Input**: WASD movement, Space attack, inventory management
- **Real-time Updates**: Subscribes to runtime events
- **Widget System**: Modular UI components (map, stats, messages, footer)

**Architecture**:
- `CliApp`: Implements `FrontendApp` trait
- Event loop: Terminal input + runtime events
- State management: UI mode tracking, cursor position

---

### 7. `client-bootstrap` - Runtime Bootstrap

**Purpose**: Configuration and runtime setup.

**Key Modules**:
- **`config.rs`**: `CliConfig` for runtime parameters
- **`builder.rs`**: Constructs `RuntimeConfig` and `OracleBundle`
- **`oracles.rs`**: Oracle factory implementations

---

### 8. `behavior-tree` - AI Decision Making

**Purpose**: Lightweight behavior tree library for turn-based games.

**Features**:
- No delta time (turn-based semantics)
- No Running state (instant success/failure)
- Minimal state (ZK-friendly)
- Zero dependencies

**Node Types**:
- Composite: `Sequence`, `Selector`, `UtilitySelector`
- Decorator: `Inverter`, `AlwaysSucceed`

---

## Key Systems

### Action Execution Pipeline

```
Action submitted
    ↓
pre_validate()     # Check pre-conditions
    ↓
apply()            # Mutate state
    ↓
post_validate()    # Verify post-conditions
    ↓
StateDelta emitted # Capture changes
    ↓
Hooks execute      # Post-execution side effects
```

**Validation Phases**:
1. **Pre-validate**: Check legality (collision, range, resources)
2. **Apply**: Mutate state directly
3. **Post-validate**: Verify invariants (entity alive, position valid)

**Delta System**:
- `StateDelta` computed via snapshot comparison
- Tracks: `TurnDelta`, `EntitiesDelta`, `WorldDelta`
- Used for: Event replay, witness generation, state sync

---

### State Management

**Hierarchical Structure**:
```
GameState
├── TurnState        # Current tick, active entity, nonce
├── EntitiesState    # Actors, items, props
│   ├── Actors       # Player, NPCs
│   ├── Items        # Inventory, ground items
│   └── Props        # Doors, levers, etc.
└── WorldState       # Tile map, overlays, occupancy
```

**State Mutations**:
- Direct mutation (no reducer pattern)
- Delta via snapshot comparison
- Thread-safe (background worker owns canonical state)

---

### Hook System

**Architecture**:
- Priority-based root hook execution
- Chainable hooks (`next_hook_names()`)
- Maximum depth limit (prevents infinite loops)
- Criticality levels (Critical, Important, Optional)

**Built-in Hooks**:
- `ActionCostHook`: Apply action time costs (Critical priority)
- `ActivationHook`: Entity activation tracking
- `DamageHook`: Entry point for damage-related effect chains (chains to death_check, bleeding, etc.)

**Hook Architecture Details**:
- Entry hooks: Can trigger chains without generating actions (e.g., DamageHook)
- Maximum depth limit: 50 levels to prevent infinite loops
- Criticality handling: Critical hooks fail entire action, Important log errors, Optional fail silently

**Hook Execution**:
```
Action executes → Hooks evaluate → should_trigger()?
    ↓ Yes
create_actions() → Execute system actions → Chain to next hooks
```

---

### Oracle System

**Purpose**: Inject read-only game content into `game-core`.

**Oracle Types**:
- `MapOracle`: Terrain, dimensions, tile queries
- `ItemOracle`: Item definitions, stats
- `NpcOracle`: Actor templates, AI configs
- `TablesOracle`: Loot tables, action costs, config
- `ConfigOracle`: Game configuration parameters
- `RngOracle`: Deterministic random number generation

**Implementation**:
- `OracleManager`: Bundles all oracles
- `SnapshotOracleBundle`: Serializable oracle snapshots for ZK proofs

---

### Prover Worker System

**Purpose**: Generate zero-knowledge proofs for action executions.

**Workflow**:
1. Reads `ActionLogEntry` from action log (via `ActionLogReader`)
2. Generates proof: `prove(before_state, action, after_state)`
3. Verifies state consistency (zkVM must match simulation)
4. Saves proof to file (optional) and updates proof index
5. Publishes `ProofGenerated` or `ProofFailed` events
6. Tracks metrics (queue depth, success rate, proving time)

**Features**:
- Resume from checkpoint via proof index
- Memory-mapped file reading for performance
- Async proof generation (non-blocking)
- State consistency verification
- Error handling with appropriate logging

### Persistence Worker System

**Purpose**: Coordinate all persistence operations (state, events, action logs, checkpoints).

**Workflow**:
1. Subscribes to event bus (all topics)
2. On `ActionExecuted`: Writes action log entry with before/after state
3. On checkpoint trigger: Saves state, creates checkpoint with hash
4. Persists all events to event log
5. Automatic flushing for ProverWorker consumption

**Features**:
- Checkpoint strategies: Every N actions or manual
- Exponential backoff retry (5 attempts)
- State hashing for verification
- Action log offset tracking
- Lag detection (panics if events lost)

### Repository System

**Purpose**: Persist mutable runtime data.

**Repository Types**:
- `StateRepository`: Save/load/checkpoint game state
- `ActionLogReader` / `ActionLogWriter`: Log action sequences with before/after state
- `CheckpointRepository`: Save checkpoints with state hashes and action log offsets
- `EventRepository`: Persist game events for replay
- `ProofIndexRepository`: Track proof generation progress (nonce, offset, resume capability)

**Implementations**:
- **Memory**: `InMemoryStateRepo`, `InMemoryActionLogReader`, `InMemoryCheckpointRepository`, `InMemoryEventRepository` (testing)
- **File**: `FileStateRepository`, `FileActionLog`, `FileCheckpointRepository`, `FileEventLog`, `FileProofIndexRepository`
- **Performance**: `MmapActionLogReader` - zero-copy memory-mapped file reading for proof generation

**Persistence Features**:
- Checkpoint strategy: Every N actions or manual
- State hashing: Cryptographic hashes for state verification
- Action log: Records before/after state for each action (proof generation input)
- Resume capability: Proof index tracks progress, can resume from checkpoint
- Automatic flushing: Action logs flushed immediately for ProverWorker consumption

---

### Event System

**Architecture**: Topic-based event bus with lag detection.

**Event Topics**:
- `GameStateEvent`: TurnCompleted, ActionExecuted, ActionFailed (with state deltas)
- `ProofEvent`: ProofStarted, ProofGenerated, ProofFailed

**Event Types**:
- `ActionExecuted`: Includes nonce, action, delta, clock, before_state, after_state
- `ActionFailed`: Includes error details
- `ProofGenerated`: Includes proof data, generation time, backend
- `ProofFailed`: Includes error details for debugging

**Features**:
- Broadcast channels (multiple subscribers)
- Topic filtering (`subscribe()`, `subscribe_multiple()`)
- Lag detection: Panics if PersistenceWorker falls behind (data integrity)
- Event persistence via PersistenceWorker
- Large buffer size (50,000 events) to prevent lag

---

### Action Provider System

**Purpose**: Pluggable input sources for game actions.

**Trait**:
```rust
trait ActionProvider: Send + Sync {
    async fn provide_action(
        &self,
        state: &GameState,
        entity_id: EntityId,
    ) -> Result<Action>;
}
```

**Implementations**:
- **Human Input** (CLI): Terminal commands
- **AI Provider**: Utility-based AI decisions
- **Wait Provider**: Default (always `Action::Wait`)

**Planned**:
- Replay provider (deterministic playback)
- Remote providers (gRPC/WebSocket)
- On-chain agents

---

### Stat System

**Architecture**:
- **Core Stats**: Base attributes (Strength, Dexterity, etc.)
- **Derived Stats**: Computed from core (HP, Accuracy, etc.)
- **Bonuses**: Stacking modifiers (equipment, status effects)
- **Resources**: Current/maximum values (HP, MP, Stamina)

**Layers**:
1. Base stats (from actor template)
2. Equipment bonuses
3. Status effect modifiers
4. Final computed stats

**Snapshot System**: Serializable stat snapshots for ZK proofs.

---

### Combat System

**Features**:
- Hit chance calculation (accuracy vs evasion)
- Damage calculation (base + bonuses + modifiers)
- Multiple attack types (Normal, Heavy, Light)
- Range/cooldown validation

**Types**:
- `AttackResult`: Hit/miss/critical, damage dealt
- `AttackOutcome`: Complete combat result
- `DamageParams`: Configurable damage formulas

---

## Implementation Status

### ✅ Fully Implemented

- **Core Engine**: GameEngine, action pipeline, state management
- **Action System**: All action types with validation
- **State Management**: Hierarchical state, delta tracking
- **Combat System**: Hit chance, damage calculation, multiple attack types
- **Stat System**: Core/derived stats, bonuses, modifiers, resources
- **Inventory System**: Item management, equipment, consumables
- **Runtime Orchestration**: Workers, event bus, API
- **Hook System**: Priority-based hooks, chaining, criticality, entry hooks
- **Oracle System**: All oracle types, snapshot support
- **Repository System**: In-memory and file-based backends, memory-mapped readers
- **CLI Frontend**: Terminal UI with examine mode, cursor system
- **AI Provider**: Utility-based decision making
- **Content Loaders**: RON/TOML deserialization
- **Behavior Tree**: Minimal BT library for AI
- **ProverWorker**: Complete ZK proof generation with metrics and resume
- **PersistenceWorker**: Coordinated persistence with checkpointing
- **Metrics System**: Lock-free proof generation metrics
- **Action Log**: Complete action logging with before/after states
- **Checkpoint System**: State snapshots with resume capability
- **Event Bus**: Topic-based with lag detection and large buffers

### ✅ Fully Implemented (Recently Completed)

- **ProverWorker**: ✅ Fully implemented ZK proof generation
  - Reads from action log using `ActionLogReader` trait
  - Generates proofs for each action (before_state → action → after_state)
  - Resume from checkpoint via proof index
  - Metrics tracking (queue depth, success rate, proving time)
  - Proof file saving (optional)
  - State consistency verification (detects determinism bugs)
  - Async proof generation via `tokio::spawn_blocking`
  
- **PersistenceWorker**: ✅ Fully implemented coordinated persistence
  - Subscribes to event bus for all events
  - Writes action log with before/after states
  - Creates checkpoints (every N actions or manual)
  - Persists events for replay
  - Exponential backoff retry logic
  - State hashing for verification
  - Automatic flushing for ProverWorker consumption

- **Metrics System**: ✅ ProofMetrics with lock-free atomics
  - Generated/failed counts
  - Queue depth tracking
  - Peak queue depth
  - Average proving time
  - Success rate percentage
  - Snapshot support for monitoring

- **Action Log System**: ✅ Complete action logging infrastructure
  - Memory-mapped file reading (zero-copy)
  - Seek support for checkpoint resume
  - Refresh support for detecting new data
  - Session-based logging

- **Checkpoint System**: ✅ Full checkpoint/resume capability
  - State snapshots with hashes
  - Action log offset tracking
  - Proof index checkpoint resume
  - Manual checkpoint commands

### 📋 Planned / In Progress

- **State Commitments**: Merkle trees for state anchoring
- **Blockchain Integration**: Smart contracts, on-chain verification
- **Additional Frontends**: Bevy, WebAssembly
- **Advanced AI**: ML-powered NPCs, expanded behavior trees
- **Multiplayer**: Shared state, gossip protocol
- **Advanced Content**: Expanded maps, items, NPCs
- **SP1 Backend**: Alternative zkVM implementation
- **Arkworks Circuits**: Custom circuit proving backend

---

## Design Patterns

### 1. Deterministic State Machine

- All logic is pure functions
- No hidden randomness (injected via providers/oracles)
- Reproducible state transitions

### 2. Three-Phase Validation

- Pre-validate → Apply → Post-validate
- Enables action-validity proofs
- Clear error boundaries

### 3. Trait-Based Extensibility

- Oracles, providers, repositories are traits
- Swappable implementations without code changes
- Testable with mock implementations

### 4. Snapshot-Based Deltas

- Clone state before mutation
- Compute delta via comparison
- Enables replay and witness generation

### 5. Hook-Based Side Effects

- Post-execution hooks for system actions
- Chainable, priority-based
- Keeps action pipeline pure

### 6. Message-Driven Architecture

- Tokio channels for async communication
- Workers communicate via commands/events
- Decoupled, testable components

### 7. Lock-Free Metrics

- Atomic operations for thread-safe metrics
- No locks needed for concurrent access
- Snapshot API for monitoring

### 8. Zero-Copy File I/O

- Memory-mapped file reading (`MmapActionLogReader`)
- Efficient for large action logs
- Supports seek/refresh operations

---

## Technology Stack

### Core Dependencies

- **Async Runtime**: `tokio` (full features)
- **Serialization**: `serde`, `serde_json`, `bincode` (v1.3 for RISC0 compatibility)
- **Error Handling**: `thiserror`, `anyhow`
- **Logging**: `tracing`, `tracing-subscriber`, `tracing-appender`
- **Data Structures**: `bounded-vector`, `arrayvec`, `bitflags` (no_std compatible)

### ZK Dependencies

- **RISC0**: `risc0-zkvm` (v3.0), `risc0-build` (v3.0)

### Client Dependencies

- **Terminal UI**: `ratatui` (v0.29), `crossterm` (v0.29)
- **Utilities**: `directories`, `dotenvy`

### Development Tools

- **Command Runner**: `just` (recommended)
- **Testing**: `tempfile` (for integration tests)

---

## Development Workflow

### Build Commands

```bash
# Stub backend (fast development)
just build stub
just run stub
just test stub

# RISC0 backend (production)
just build risc0
just run risc0

# Default backend (via env)
export ZK_BACKEND=stub
just build
```

### Code Quality

```bash
just fmt          # Format code
just lint         # Clippy checks
just pre-commit   # Format + lint + test
```

### Testing

```bash
just test stub    # Test with stub backend
cargo test --workspace  # Test all crates
```

### Documentation

Key documentation files:
- `README.md`: Project overview, philosophy, quick start
- `docs/wip/architecture.md`: Detailed architecture diagrams
- `docs/philosophy.md`: Design philosophy and principles
- `docs/wip/research.md`: Design decisions and trade-offs
- `AGENTS.md`: Guidelines for AI-assisted development

---

## Key Files Reference

### Entry Points

- `crates/client/cli/src/main.rs`: CLI application entry
- `crates/runtime/src/runtime.rs`: Runtime orchestrator
- `crates/game/core/src/engine/mod.rs`: Game engine implementation

### Core Types

- `crates/game/core/src/state/mod.rs`: State definitions
- `crates/game/core/src/action/mod.rs`: Action system
- `crates/runtime/src/api/handle.rs`: RuntimeHandle API

### Configuration

- `crates/game/content/data/config.toml`: Game configuration
- `crates/client/bootstrap/src/config.rs`: Client configuration
- `justfile`: Build automation commands

---

## Notes

### ZK Proof Strategy Evolution

Originally planned to prove full execution in ZK circuits. Strategy evolved to:
- **Action-validity proofs only**: Prove pre/post validation passed
- **No execution proof**: Don't prove exact execution path (move logic, combat calculations)
- **Smaller circuits**: Faster proving, smaller proofs

### State Reducer Pattern Removed

Originally designed `StateReducer` pattern for explicit state changes. Replaced with:
- Direct state mutation
- Snapshot-based delta computation
- Simpler mental model, sufficient for current needs

### Transaction Guard Removed

Considered `TransactionGuard` for all-or-nothing semantics. Decision:
- Pre/post validation sufficient
- Hides bugs instead of fixing them
- Adds unnecessary complexity
- Conflicts with deterministic design

---

## Contributing

See `AGENTS.md` for AI-assisted development guidelines:
- Maintain determinism
- Respect module boundaries
- ZK awareness
- Rust 2024 discipline
- Testing & verification

---

**Generated from comprehensive codebase analysis**
