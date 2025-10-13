# Project Status

> **Status:** Living document  
>
> **Scope:** Tracks Dungeon’s implementation progress, feature roadmap, and current technical priorities across engine, runtime, client, and proving layers.

---

This document tracks the current implementation status and roadmap for the Dungeon project. Unlike the architecture documentation, this file is updated frequently as features are completed.

## Current Implementation

### ✅ Core Engine & Runtime

- **Game Core Engine**: Pure deterministic state machine with no I/O or side effects
  - Three-phase action validation (pre-validate, execute, post-validate)
  - Comprehensive action types: Movement, Combat, Inventory, Interaction
  - State delta tracking for witness generation and event replay
  - Environment oracle system for content injection

- **Turn-Based Scheduling**: Deterministic turn management
  - Entity activation tracking with cooldown system
  - Turn preparation via `GameEngine::prepare_next_turn()`
  - Ready-at timestamps for action ordering

- **Runtime Orchestration**: Message-driven worker architecture
  - `SimulationWorker`: Manages canonical `GameState`, processes turns and actions
  - Event broadcasting via `tokio::broadcast` for fan-out consumption
  - `RuntimeHandle` API for client interaction
  - In-memory state repositories for testing

- **Event System**: Real-time game event notifications
  - `TurnCompleted`: New turn prepared with tick number
  - `ActionExecuted`: Successful actions with state deltas
  - `ActionFailed`: Validation/execution errors with context

### ✅ Client Infrastructure

- **Client Core**: Bootstrap and configuration layer
  - `CliConfig` for runtime parameters
  - Oracle factory system (`TestOracleFactory`)
  - Runtime initialization and lifecycle management

- **Frontend Abstraction**: `FrontendApp` trait in `client/frontend/core`
  - View model transformations
  - Message routing between UI and runtime
  - Event consumption patterns

- **CLI Terminal Interface**: Full-featured async application
  - **Examine Mode**: Press `E` to explore the map with cursor
    - WASD navigation to inspect tiles and entities
    - Detailed entity stats, items, and tile information display
  - **Cursor System**:
    - Manual cursor control in examine mode
    - Automatic targeting for combat (nearest enemy selection)
    - Target validation and range checking
  - **Action Input**: WASD movement, Space attack, inventory commands
  - **Real-time Updates**: Subscribes to runtime events for immediate feedback
  - **State Management**: Tracks UI modes, cursor position, player entity

### ✅ Content System

- **Static Content**: Maps, items, NPCs, loot tables in `game/content`
- **Oracle Implementations**: Adapters exposing content to game engine
  - `MapOracleImpl`: Terrain and map dimensions
  - `ItemOracleImpl`: Item definitions and categories
  - `NpcOracleImpl`: NPC templates and behaviors
  - `TablesOracleImpl`: Loot tables and drop rates
  - `ConfigOracleImpl`: Game configuration (cooldowns, ranges, damage)

## In Progress

### 🚧 Content Pipeline

- Map authoring tools
- NPC behavior definition system
- Loot table balancing utilities
- Content validation and testing framework

### 🚧 Advanced Combat Mechanics

- Status effects (poison, stun, buffs/debuffs)
- Area-of-effect abilities
- Damage types and resistances
- Combo system and action chaining

## Roadmap

### 📋 Near-term

**Persistence Layer**
- Database-backed `StateRepository` implementations
  - RocksDB for local single-player saves
  - PostgreSQL for server-side multiplayer persistence
- Save/load functionality with state compression
- Checkpoint management for replay and debugging

**Enhanced Content**
- Richer item system with equipment slots
- Procedural map generation
- Quest and objective tracking
- Dialogue system for NPCs

**Testing & Quality**
- Integration test suite for end-to-end scenarios
- Property-based testing for action validation
- Performance benchmarking and optimization
- Fuzzing for edge case discovery

### 📋 Medium-term

**ZK Proof System**
- Action validation circuits in `zk` crate
  - Pre-validate phase circuit
  - Execute phase circuit
  - Post-validate phase circuit
- `ProverWorker` implementation
  - Checkpoint consumption from `SimulationWorker`
  - Proof generation with witness data
  - Proof artifact broadcasting
- Proof aggregation for batch verification
- Performance optimization (parallel proving, recursion)

**Blockchain Integration**
- Smart contract verifier (Solidity)
  - Action validity verification
  - State commitment anchoring
  - Gas-optimized proof verification
- `SubmitWorker` for proof submission
  - Transaction signing and submission
  - Retry logic and rate limiting
  - Gas management and MEV protection
- RPC integration with EVM chains
- Event monitoring and synchronization

**AI/NPC Providers**
- Heuristic-based decision trees for NPCs
- Behavior scripting language
- ML-powered agents (reinforcement learning)
- Emergent cooperative/competitive behaviors
- Training infrastructure for AI agents

### 📋 Long-term

**Multiplayer Support**
- Remote `ActionProvider` implementations
  - gRPC-based provider for low latency
  - WebSocket provider for browser clients
- Client-server architecture
  - Authority model with client prediction
  - State synchronization and reconciliation
  - Lag compensation and rollback
- Anti-cheat via ZK proofs
  - Replay protection
  - Action validity enforcement
  - State tampering detection

**Additional Frontends**
- **Bevy Frontend**: 2D/3D graphical client
  - Sprite-based rendering or 3D models
  - Particle effects and animations
  - Audio integration
  - Same `FrontendApp` trait implementation
- **WebAssembly UI**: Browser-based client
  - Canvas or WebGL rendering
  - Touch controls for mobile browsers
  - Offline play with local persistence
- **Headless Client**: For AI training and simulation
  - Batch processing mode
  - Metrics collection
  - Replay analysis

**Modding Support**
- Plugin API for custom actions and rules
  - Hook system for action phases
  - Custom validation logic
  - New action types
- Content packs with versioning
  - Asset loading (maps, sprites, sounds)
  - Dependency management
  - Compatibility checking
- Mod marketplace with ZK verification
  - Content authenticity proofs
  - Sandboxed execution
  - Revenue sharing for creators

**Developer Tools**
- **Editor Mode**: Visual map and entity editor
  - Tile placement and terrain editing
  - Entity spawning and configuration
  - Real-time playtesting via runtime handle
  - Undo/redo support
- **Analytics Dashboard**: Gameplay metrics
  - Action frequency and success rates
  - Combat balance analysis
  - Performance profiling (FPS, memory, latency)
  - Player behavior insights
- **Debugging Tools**
  - State inspector and time-travel debugging
  - Action replay and bisect
  - Network traffic analysis
  - Proof verification debugging

## Technical Debt & Known Issues

### High Priority

- [ ] Error handling consistency across crates
- [ ] Documentation coverage (API docs, examples)
- [ ] Test coverage for edge cases (especially action validation)

### Medium Priority

- [ ] Performance profiling and optimization
- [ ] Memory usage analysis (large state snapshots)
- [ ] CI/CD pipeline setup (testing, linting, builds)
- [ ] Logging and tracing infrastructure

### Low Priority

- [ ] Code style consistency (some modules use different patterns)
- [ ] Dependency audit and minimization
- [ ] Cross-platform compatibility testing (Windows, macOS, Linux)

## Metrics & Progress Tracking

| Category | Implemented | In Progress | Planned | Total |
|----------|-------------|-------------|---------|-------|
| **Core Systems** | 8 | 2 | 3 | 13 |
| **Client Features** | 5 | 0 | 3 | 8 |
| **Content & Tools** | 3 | 2 | 5 | 10 |
| **ZK & Blockchain** | 2 (foundations) | 0 | 6 | 8 |
| **Total** | 18 | 4 | 17 | 39 |

**Completion: ~46%** (18 implemented + 4 in progress out of 39 total features)
