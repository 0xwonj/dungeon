# RISC0 zkVM Architecture Design

**Project**: Dungeon RPG
**Date**: 2025-01-15
**Status**: Design Phase

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current Codebase Analysis](#current-codebase-analysis)
3. [Design Challenges](#design-challenges)
4. [Proposed Architecture](#proposed-architecture)
5. [Component Design](#component-design)
6. [Data Flow](#data-flow)
7. [Implementation Phases](#implementation-phases)
8. [Testing Strategy](#testing-strategy)
9. [Performance Considerations](#performance-considerations)

---

## Executive Summary

This document describes the complete architecture for integrating RISC0 zkVM into the dungeon project. The design ensures:

- **Deterministic proof generation** for all game actions
- **Minimal code duplication** by reusing game-core logic
- **Oracle data serialization** for guest program access
- **Clean separation** between host (runtime) and guest (zkVM) code
- **Backward compatibility** with existing runtime architecture

### Key Decisions

1. **Guest program uses serialized oracle snapshots** - not live trait objects
2. **game-core remains std** - guest wrapper handles no_std translation
3. **Single ELF binary** for all action types
4. **Prover owns oracle serialization** - keeps runtime clean

---

## Current Codebase Analysis

### Existing Architecture

```
crates/
├── game/
│   ├── core/          # ✅ Pure deterministic logic (std)
│   │   ├── action/    # Action validation & execution
│   │   ├── engine/    # GameEngine orchestration
│   │   ├── env/       # Oracle trait definitions
│   │   └── state/     # GameState, StateDelta
│   └── content/       # ✅ Static content (empty placeholder)
│
├── runtime/           # Imperative shell
│   ├── oracle/        # ⚠️  Oracle implementations (Arc<dyn Trait>)
│   ├── workers/       # SimulationWorker, ProverWorker
│   └── hooks/         # Post-execution hooks
│
└── zk/                # ❌ Stub implementation
    ├── src/
    │   └── zkvm/      # StubZkvmProver (placeholder)
    └── methods/       # ❌ MISSING - guest program not implemented
```

### Key Constraints

#### game-core (Deterministic Core)

```toml
[dependencies]
bounded-vector = { version = "0.3", default-features = false }
arrayvec = { version = "0.7", default-features = false }
thiserror = { version = "2.0", default-features = false }
bitflags = { version = "2.9", default-features = false }
```

**Characteristics:**
- Already uses `default-features = false` for all deps
- Uses `std::error::Error` trait (requires std)
- Uses `String` and `Vec` (requires alloc)
- **NO** `serde` support currently

**Status**: ⚠️ **Can be made no_std + alloc with minimal changes**

#### Oracle Traits (game-core/src/env/)

```rust
pub trait MapOracle {
    fn dimensions(&self) -> MapDimensions;
    fn tile(&self, position: Position) -> Option<&StaticTile>;
    fn initial_entities(&self) -> &[InitialEntitySpec];
}

pub trait ItemOracle { ... }
pub trait TablesOracle { ... }
pub trait NpcOracle { ... }
pub trait ConfigOracle { ... }

// Runtime uses dynamic dispatch
pub type GameEnv<'a> = Env<
    'a,
    dyn MapOracle + 'a,
    dyn ItemOracle + 'a,
    dyn TablesOracle + 'a,
    dyn NpcOracle + 'a,
    dyn ConfigOracle + 'a,
>;
```

**Problem**: Guest cannot use `dyn Trait` (requires vtables, not serializable)

**Solution**: Serialize oracle data as concrete structs

---

## Design Challenges

### Challenge 1: no_std Compatibility

**Problem:**
- RISC0 guest requires `#![no_std]` (RISC-V embedded environment)
- game-core uses `std::error::Error` trait
- Oracle traits use dynamic dispatch (`dyn Trait`)

**Solution:**
- Keep game-core as `std` (too invasive to change)
- Create thin `no_std` wrapper in guest that:
  - Defines serializable oracle structs
  - Translates between serialized data and trait objects
  - Re-exports game-core logic

### Challenge 2: Oracle Data Handling

**Problem:**
- Runtime uses `Arc<dyn MapOracle>` (not serializable)
- Guest needs concrete oracle data
- Oracle data is large (maps, NPC templates, item definitions)

**Solution:**
- Define `OracleSnapshot` struct (serializable)
- Host serializes oracle data before proving
- Guest deserializes and wraps in trait adapters

### Challenge 3: Serde Support

**Problem:**
- game-core types don't derive `Serialize`/`Deserialize`
- Need serde for risc0-zkvm I/O

**Solution:**
- Add serde feature to game-core
- Conditionally derive serde traits
- Enable only in zk crate and guest

### Challenge 4: Code Duplication

**Problem:**
- Risk of duplicating game logic in guest
- Must ensure guest uses same code as runtime

**Solution:**
- Guest imports `game-core` directly
- Guest only provides oracle adapters
- Single source of truth for game logic

---

## Proposed Architecture

### Directory Structure

```
crates/
├── game/
│   └── core/
│       ├── Cargo.toml          # Add serde feature
│       └── src/
│           ├── lib.rs          # Conditional serde derives
│           ├── action/
│           ├── engine/
│           ├── env/
│           └── state/
│
├── runtime/
│   ├── Cargo.toml
│   └── src/
│       ├── oracle/             # Existing oracle implementations
│       └── workers/
│           └── prover.rs       # ✅ Serializes oracle snapshot
│
└── zk/
    ├── Cargo.toml              # Workspace + build deps
    ├── build.rs                # risc0_build::embed_methods()
    │
    ├── src/
    │   ├── lib.rs              # include!(methods.rs), re-exports
    │   ├── oracle.rs           # NEW: OracleSnapshot definition
    │   └── zkvm/
    │       ├── mod.rs
    │       ├── stub.rs         # Existing stub
    │       └── risc0.rs        # NEW: Risc0Prover
    │
    └── methods/
        └── guest/
            ├── Cargo.toml      # no_std guest dependencies
            └── src/
                ├── main.rs     # Guest entry point
                └── oracle.rs   # Oracle trait adapters
```

### Feature Flags

#### game-core/Cargo.toml

```toml
[features]
default = []
serde = ["dep:serde"]

[dependencies]
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"], optional = true }
```

#### zk/Cargo.toml

```toml
[features]
default = ["risc0"]
risc0 = ["risc0-zkvm", "risc0-build"]
sp1 = ["sp1-sdk"]
zkvm = []  # Stub prover

[build-dependencies]
risc0-build = { version = "3.0", optional = true }

[dependencies]
game-core = { path = "../game/core", features = ["serde"] }
risc0-zkvm = { version = "3.0", optional = true }
bincode = "2"
serde = { version = "1.0", features = ["derive"] }

[package.metadata.risc0]
methods = ["methods/guest"]
```

#### zk/methods/guest/Cargo.toml

```toml
[package]
name = "game-verifier-guest"
version = "0.1.0"
edition = "2024"

[dependencies]
risc0-zkvm = { version = "3.0", default-features = false, features = ["libm"] }
game-core = { path = "../../../../game/core", features = ["serde"] }
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }

# Guest must be no_std
[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

---

## Component Design

### 1. OracleSnapshot (zk/src/oracle.rs)

**Purpose**: Serializable snapshot of all oracle data for guest consumption.

```rust
use serde::{Deserialize, Serialize};
use game_core::{
    MapDimensions, StaticTile, InitialEntitySpec, ItemDefinition,
    NpcTemplate, MovementRules, AttackProfile, GameConfig, Position,
};

/// Serializable snapshot of all oracle data.
///
/// This structure captures all static game content needed by the guest
/// program to execute game logic deterministically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleSnapshot {
    pub map: MapSnapshot,
    pub items: ItemsSnapshot,
    pub npcs: NpcsSnapshot,
    pub tables: TablesSnapshot,
    pub config: ConfigSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSnapshot {
    pub dimensions: MapDimensions,
    /// Flat array of tiles (row-major order)
    pub tiles: Vec<StaticTile>,
    pub initial_entities: Vec<InitialEntitySpec>,
}

impl MapSnapshot {
    pub fn get_tile(&self, position: Position) -> Option<&StaticTile> {
        let MapDimensions { width, height } = self.dimensions;
        if position.x >= width || position.y >= height {
            return None;
        }
        let index = (position.y * width + position.x) as usize;
        self.tiles.get(index)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemsSnapshot {
    /// Map from ItemHandle to ItemDefinition
    pub items: Vec<(u32, ItemDefinition)>,
}

impl ItemsSnapshot {
    pub fn definition(&self, handle: u32) -> Option<&ItemDefinition> {
        self.items.iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, def)| def)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcsSnapshot {
    /// Map from template ID to NpcTemplate
    pub npcs: Vec<(u32, NpcTemplate)>,
}

impl NpcsSnapshot {
    pub fn template(&self, id: u32) -> Option<&NpcTemplate> {
        self.npcs.iter()
            .find(|(tid, _)| *tid == id)
            .map(|(_, tmpl)| tmpl)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablesSnapshot {
    pub movement_rules: MovementRules,
    pub attack_profiles: Vec<AttackProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub config: GameConfig,
}

impl OracleSnapshot {
    /// Creates a snapshot from runtime oracle implementations.
    ///
    /// This is called by the host (ProverWorker) before proof generation.
    pub fn from_runtime_oracles(
        map: &impl game_core::MapOracle,
        items: &impl game_core::ItemOracle,
        npcs: &impl game_core::NpcOracle,
        tables: &impl game_core::TablesOracle,
        config: &impl game_core::ConfigOracle,
    ) -> Self {
        // Serialize map data
        let dimensions = map.dimensions();
        let mut tiles = Vec::new();
        for y in 0..dimensions.height {
            for x in 0..dimensions.width {
                if let Some(tile) = map.tile(Position { x, y }) {
                    tiles.push(tile.clone());
                } else {
                    // Default empty tile
                    tiles.push(StaticTile::default());
                }
            }
        }

        let map_snapshot = MapSnapshot {
            dimensions,
            tiles,
            initial_entities: map.initial_entities().to_vec(),
        };

        // Serialize items (iterate all known handles)
        // Note: This requires ItemOracle to expose all items
        // For now, we'll need to enhance the trait or use a different approach
        let items_snapshot = ItemsSnapshot {
            items: vec![], // TODO: Populate from ItemOracle
        };

        // Serialize NPCs (iterate all known templates)
        let npcs_snapshot = NpcsSnapshot {
            npcs: vec![], // TODO: Populate from NpcOracle
        };

        // Serialize tables
        let tables_snapshot = TablesSnapshot {
            movement_rules: tables.movement_rules(),
            attack_profiles: vec![], // TODO: Iterate attack profiles
        };

        // Serialize config
        let config_snapshot = ConfigSnapshot {
            config: GameConfig {
                activation_radius: config.activation_radius(),
            },
        };

        Self {
            map: map_snapshot,
            items: items_snapshot,
            npcs: npcs_snapshot,
            tables: tables_snapshot,
            config: config_snapshot,
        }
    }
}
```

**Design Notes:**
- Host creates snapshot from `Arc<dyn Trait>` oracles
- Guest deserializes snapshot and wraps in trait adapters
- All data is cloned (acceptable for proof generation)
- Efficient flat storage (Vec instead of HashMap)

### 2. Guest Oracle Adapters (methods/guest/src/oracle.rs)

**Purpose**: Provide oracle trait implementations for guest using OracleSnapshot.

```rust
use game_core::{
    MapOracle, ItemOracle, NpcOracle, TablesOracle, ConfigOracle,
    MapDimensions, StaticTile, InitialEntitySpec, Position,
    ItemDefinition, NpcTemplate, MovementRules, AttackProfile,
    ItemHandle, GameConfig,
};
use crate::OracleSnapshot;

/// Guest-side oracle adapter for map data
pub struct GuestMapOracle<'a> {
    snapshot: &'a crate::MapSnapshot,
}

impl<'a> GuestMapOracle<'a> {
    pub fn new(snapshot: &'a crate::MapSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> MapOracle for GuestMapOracle<'a> {
    fn dimensions(&self) -> MapDimensions {
        self.snapshot.dimensions
    }

    fn tile(&self, position: Position) -> Option<&StaticTile> {
        self.snapshot.get_tile(position)
    }

    fn initial_entities(&self) -> &[InitialEntitySpec] {
        &self.snapshot.initial_entities
    }
}

/// Guest-side oracle adapter for items
pub struct GuestItemOracle<'a> {
    snapshot: &'a crate::ItemsSnapshot,
}

impl<'a> GuestItemOracle<'a> {
    pub fn new(snapshot: &'a crate::ItemsSnapshot) -> Self {
        Self { snapshot }
    }
}

impl<'a> ItemOracle for GuestItemOracle<'a> {
    fn definition(&self, handle: ItemHandle) -> Option<&ItemDefinition> {
        self.snapshot.definition(handle.0)
    }
}

// Similar adapters for NpcOracle, TablesOracle, ConfigOracle...

/// Creates a GameEnv from OracleSnapshot for use in guest
pub fn create_game_env<'a>(
    snapshot: &'a OracleSnapshot,
) -> game_core::Env<
    'a,
    GuestMapOracle<'a>,
    GuestItemOracle<'a>,
    GuestTablesOracle<'a>,
    GuestNpcOracle<'a>,
    GuestConfigOracle<'a>,
> {
    let map = GuestMapOracle::new(&snapshot.map);
    let items = GuestItemOracle::new(&snapshot.items);
    let tables = GuestTablesOracle::new(&snapshot.tables);
    let npcs = GuestNpcOracle::new(&snapshot.npcs);
    let config = GuestConfigOracle::new(&snapshot.config);

    game_core::Env::with_all(map, items, tables, npcs, config)
}
```

### 3. Guest Program (methods/guest/src/main.rs)

**Purpose**: Verify game state transition inside zkVM.

```rust
#![no_main]
#![no_std]

extern crate alloc;

use risc0_zkvm::guest::env;
use game_core::{GameState, Action, GameEngine, StateDelta};

mod oracle;
use oracle::create_game_env;

risc0_zkvm::guest::entry!(main);

pub fn main() {
    // Read inputs from host
    let oracle_snapshot: crate::OracleSnapshot = env::read();
    let before_state: GameState = env::read();
    let action: Action = env::read();
    let expected_after: GameState = env::read();

    // Create game environment from snapshot
    let game_env = create_game_env(&oracle_snapshot);

    // Execute action deterministically
    let mut engine = GameEngine::new(before_state.clone(), &game_env.into_game_env());

    let result = engine.execute(action.clone())
        .expect("Action execution must succeed in guest");

    // Verify state transition
    assert_eq!(
        result.state,
        expected_after,
        "State mismatch: computed state does not match expected state"
    );

    // Commit public outputs to journal
    env::commit(&result.delta);
    env::commit(&result.state.turn.clock);
    env::commit(&action);
}
```

**Design Notes:**
- Uses `#![no_std]` + `extern crate alloc`
- Reads serialized oracle snapshot first
- Creates trait adapters from snapshot
- Executes game logic using same GameEngine as runtime
- Commits delta and clock to journal for verification

### 4. Risc0Prover (zk/src/zkvm/risc0.rs)

**Purpose**: Host-side prover implementation using RISC0.

```rust
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use game_core::{Action, GameState, StateDelta};
use crate::{ProofBackend, ProofData, ProofError, OracleSnapshot};
use super::ZkvmProver;

#[cfg(feature = "risc0")]
use crate::{GAME_VERIFIER_ELF, GAME_VERIFIER_ID};

/// RISC0 zkVM prover implementation.
pub struct Risc0Prover {
    oracle_snapshot: OracleSnapshot,
}

impl Risc0Prover {
    /// Creates a new RISC0 prover with oracle snapshot.
    ///
    /// The oracle snapshot is captured once at prover creation and reused
    /// for all subsequent proofs. This assumes oracle data is immutable.
    pub fn new(oracle_snapshot: OracleSnapshot) -> Self {
        Self { oracle_snapshot }
    }
}

impl ZkvmProver for Risc0Prover {
    fn prove(
        &self,
        before_state: &GameState,
        action: &Action,
        after_state: &GameState,
        _delta: &StateDelta,
    ) -> Result<ProofData, ProofError> {
        // Build executor environment with inputs
        let env = ExecutorEnv::builder()
            // Send oracle snapshot first
            .write(&self.oracle_snapshot)
            .map_err(|e| ProofError::ZkvmError(format!("Failed to write oracle_snapshot: {}", e)))?
            // Send game state and action
            .write(&before_state)
            .map_err(|e| ProofError::ZkvmError(format!("Failed to write before_state: {}", e)))?
            .write(&action)
            .map_err(|e| ProofError::ZkvmError(format!("Failed to write action: {}", e)))?
            .write(&after_state)
            .map_err(|e| ProofError::ZkvmError(format!("Failed to write after_state: {}", e)))?
            .build()
            .map_err(|e| ProofError::ZkvmError(format!("Failed to build ExecutorEnv: {}", e)))?;

        // Generate proof
        let prover = default_prover();
        let prove_info = prover
            .prove(env, GAME_VERIFIER_ELF)
            .map_err(|e| ProofError::ZkvmError(format!("Proof generation failed: {}", e)))?;

        let receipt = prove_info.receipt;

        // Verify receipt locally (sanity check)
        receipt
            .verify(GAME_VERIFIER_ID)
            .map_err(|e| ProofError::ZkvmError(format!("Receipt verification failed: {}", e)))?;

        // Serialize receipt
        let bytes = bincode::serialize(&receipt)
            .map_err(|e| ProofError::SerializationError(e.to_string()))?;

        Ok(ProofData {
            bytes,
            backend: ProofBackend::Risc0,
        })
    }

    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError> {
        if proof.backend != ProofBackend::Risc0 {
            return Err(ProofError::ZkvmError(
                format!("Expected Risc0 backend, got {:?}", proof.backend)
            ));
        }

        // Deserialize receipt
        let receipt: Receipt = bincode::deserialize(&proof.bytes)
            .map_err(|e| ProofError::SerializationError(e.to_string()))?;

        // Verify receipt
        receipt
            .verify(GAME_VERIFIER_ID)
            .map_err(|e| ProofError::ZkvmError(format!("Verification failed: {}", e)))?;

        Ok(true)
    }
}
```

### 5. ProverWorker Integration (runtime/src/workers/prover.rs)

**Purpose**: Create oracle snapshot and invoke prover.

```rust
use zk::{OracleSnapshot, zkvm::{Risc0Prover, ZkvmProver}};

impl ProverWorker {
    pub fn new(
        initial_state: GameState,
        event_rx: broadcast::Receiver<GameEvent>,
        event_tx: broadcast::Sender<GameEvent>,
        oracle_manager: Arc<OracleManager>,  // NEW: Add oracle manager
    ) -> Self {
        Self {
            current_state: initial_state,
            oracle_manager,  // Store for snapshot creation
            entity_tree: None,
            world_tree: None,
            event_rx,
            event_tx,
        }
    }

    async fn generate_proof_full_rebuild(
        &self,
        action: &Action,
        delta: &StateDelta,
        before_state: &GameState,
        after_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        #[cfg(feature = "risc0")]
        {
            // Create oracle snapshot from runtime oracle manager
            let env = self.oracle_manager.as_game_env();
            let oracle_snapshot = OracleSnapshot::from_runtime_oracles(
                env.map().unwrap(),
                env.items().unwrap(),
                env.npcs().unwrap(),
                env.tables().unwrap(),
                env.config().unwrap(),
            );

            // Create prover with snapshot
            let prover = Risc0Prover::new(oracle_snapshot);

            // Generate proof
            prover.prove(before_state, action, after_state, delta)
        }

        #[cfg(all(not(feature = "risc0"), feature = "zkvm"))]
        {
            // Stub prover
            use zk::zkvm::StubZkvmProver;
            let prover = StubZkvmProver::new();
            prover.prove(before_state, action, after_state, delta)
        }

        #[cfg(not(any(feature = "risc0", feature = "zkvm")))]
        {
            // No proving
            Ok(ProofData {
                bytes: vec![],
                backend: ProofBackend::None,
            })
        }
    }
}
```

---

## Data Flow

### Proof Generation Flow

```text
┌─────────────────────────────────────────────────────────────┐
│                      SimulationWorker                       │
│  - Owns canonical GameState                                 │
│  - Executes actions via GameEngine                          │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ Broadcasts ActionExecuted event
                     │ (before_state, action, after_state, delta)
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                       ProverWorker                          │
│  - Subscribes to ActionExecuted                             │
│  - Owns Arc<OracleManager>                                  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 1. Create OracleSnapshot
                     │    from runtime oracle implementations
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              OracleSnapshot::from_runtime_oracles()         │
│  - Serializes all oracle data (map, items, npcs, tables)   │
│  - Returns OracleSnapshot (owned, serializable)             │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 2. Create Risc0Prover with snapshot
                     ▼
┌──────────────────────────────────────────────────────────���──┐
│                    Risc0Prover::prove()                     │
│  - Builds ExecutorEnv:                                      │
│    1. write(oracle_snapshot)                                │
│    2. write(before_state)                                   │
│    3. write(action)                                         │
│    4. write(after_state)                                    │
│  - Calls default_prover().prove(env, ELF)                   │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 3. Guest execution starts
                     ▼
┌─────────────────────────────────────────────────────────────┐
│               Guest Program (RISC-V zkVM)                   │
│  1. env::read() -> oracle_snapshot                          │
│  2. env::read() -> before_state                             │
│  3. env::read() -> action                                   │
│  4. env::read() -> expected_after                           │
│                                                             │
│  5. create_game_env(&oracle_snapshot)                       │
│     - GuestMapOracle wraps MapSnapshot                      │
│     - GuestItemOracle wraps ItemsSnapshot                   │
│     - ... (all oracle adapters)                             │
│                                                             │
│  6. GameEngine::new(before_state, &env)                     │
│  7. engine.execute(action)                                  │
│  8. assert_eq!(result.state, expected_after)                │
│                                                             │
│  9. env::commit(&result.delta)                              │
│ 10. env::commit(&result.state.turn.clock)                   │
│ 11. env::commit(&action)                                    │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 4. Proof generation completes
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                      Receipt Created                        │
│  - Journal: [delta, clock, action]                          │
│  - Seal: STARK proof bytes                                  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 5. Serialize receipt
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                    ProofData Returned                       │
│  - bytes: bincode::serialize(&receipt)                      │
│  - backend: ProofBackend::Risc0                             │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 6. Broadcast ProofGenerated event
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                   Clients / SubmitWorker                    │
│  - Receive proof event                                      │
│  - Can verify receipt.verify(IMAGE_ID)                      │
│  - Can submit to blockchain (future)                        │
└─────────────────────────────────────────────────────────────┘
```

### Key Data Structures

```rust
// Inputs to guest (via env::write)
struct GuestInputs {
    oracle_snapshot: OracleSnapshot,  // ~10-100KB (map + content)
    before_state: GameState,          // ~1-10KB (runtime state)
    action: Action,                   // ~100 bytes
    after_state: GameState,           // ~1-10KB
}

// Outputs from guest (via env::commit)
struct GuestOutputs {
    delta: StateDelta,                // ~1KB (changes)
    clock: Tick,                      // 8 bytes
    action: Action,                   // ~100 bytes (for verification)
}
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1)

**Goal**: Add serde support to game-core and define oracle snapshot.

**Tasks**:
1. ✅ Add serde feature to game-core/Cargo.toml
2. ✅ Derive Serialize/Deserialize for all game-core types
3. ✅ Create zk/src/oracle.rs with OracleSnapshot
4. ✅ Implement OracleSnapshot::from_runtime_oracles()
5. ✅ Unit tests for serialization roundtrip

**Validation**:
```rust
#[test]
fn test_oracle_snapshot_roundtrip() {
    let snapshot = OracleSnapshot::from_runtime_oracles(...);
    let bytes = bincode::serialize(&snapshot).unwrap();
    let deserialized: OracleSnapshot = bincode::deserialize(&bytes).unwrap();
    assert_eq!(snapshot, deserialized);
}
```

### Phase 2: Guest Program (Week 2)

**Goal**: Implement guest program with oracle adapters.

**Tasks**:
1. ✅ Create methods/guest directory structure
2. ✅ Write methods/guest/Cargo.toml (no_std config)
3. ✅ Implement methods/guest/src/oracle.rs (trait adapters)
4. ✅ Implement methods/guest/src/main.rs (entry point)
5. ✅ Configure zk/build.rs with risc0_build

**Validation**:
```bash
# Build guest program
cd crates/zk
cargo build --release --features risc0

# Verify ELF and ImageID generation
ls -lh target/release/build/zk-*/out/methods.rs
```

### Phase 3: Host Prover (Week 3)

**Goal**: Implement Risc0Prover and integrate with ProverWorker.

**Tasks**:
1. ✅ Implement zk/src/zkvm/risc0.rs (Risc0Prover)
2. ✅ Update zk/src/zkvm/mod.rs (DefaultProver selection)
3. ✅ Update runtime/src/workers/prover.rs (oracle snapshot creation)
4. ✅ Update Runtime::builder() to pass OracleManager to ProverWorker
5. ✅ Integration test with RISC0_DEV_MODE=1

**Validation**:
```rust
#[tokio::test]
async fn test_prover_worker_risc0_dev_mode() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let runtime = Runtime::builder()
        .enable_proving(true)
        .build()
        .await
        .unwrap();

    // Execute action and wait for proof
    let mut events = runtime.subscribe_events();
    runtime.execute_action(move_action).await.unwrap();

    while let Ok(event) = events.recv().await {
        if let GameEvent::ProofGenerated { proof_data, .. } = event {
            assert_eq!(proof_data.backend, ProofBackend::Risc0);
            break;
        }
    }
}
```

### Phase 4: Real Proving (Week 4)

**Goal**: Generate real STARK proofs and optimize performance.

**Tasks**:
1. ✅ Test with RISC0_DEV_MODE=0 (real proofs)
2. ✅ Measure proof generation time
3. ✅ Optimize oracle snapshot size
4. ✅ Add async execution with tokio::spawn_blocking
5. ✅ Integration test with full runtime

**Validation**:
```rust
#[tokio::test]
#[ignore] // Expensive test
async fn test_risc0_real_proof() {
    std::env::set_var("RISC0_DEV_MODE", "0");

    let start = Instant::now();
    let prover = Risc0Prover::new(oracle_snapshot);
    let proof = prover.prove(&before, &action, &after, &delta).unwrap();
    let elapsed = start.elapsed();

    println!("Proof generation: {:?}", elapsed);
    assert!(prover.verify(&proof).unwrap());
}
```

### Phase 5: Production Hardening (Week 5)

**Goal**: Error handling, logging, and production readiness.

**Tasks**:
1. ✅ Add comprehensive error handling
2. ✅ Add tracing/logging to prover pipeline
3. ✅ Handle lagged events gracefully
4. ✅ Add metrics (proof generation time, success rate)
5. ✅ Documentation and examples

**Validation**:
- All tests pass
- Error paths tested
- Performance benchmarks documented

---

## Testing Strategy

### Unit Tests

**game-core serialization:**
```rust
#[cfg(feature = "serde")]
mod serde_tests {
    #[test]
    fn test_game_state_serde() {
        let state = GameState::default();
        let bytes = bincode::serialize(&state).unwrap();
        let deserialized: GameState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(state, deserialized);
    }
}
```

**OracleSnapshot:**
```rust
#[test]
fn test_oracle_snapshot_creation() {
    let manager = OracleManager::test_manager();
    let env = manager.as_game_env();
    let snapshot = OracleSnapshot::from_runtime_oracles(...);

    assert_eq!(snapshot.map.dimensions, MapDimensions { width: 20, height: 20 });
}
```

### Integration Tests

**Guest program (dev mode):**
```rust
#[test]
fn test_guest_program_dev_mode() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let env = ExecutorEnv::builder()
        .write(&oracle_snapshot)
        .write(&before_state)
        .write(&action)
        .write(&after_state)
        .build()
        .unwrap();

    let prover = default_prover();
    let receipt = prover.prove(env, GAME_VERIFIER_ELF).unwrap().receipt;

    receipt.verify(GAME_VERIFIER_ID).unwrap();

    let delta: StateDelta = receipt.journal.decode().unwrap();
    // Verify delta matches expected changes
}
```

**ProverWorker:**
```rust
#[tokio::test]
async fn test_prover_worker_integration() {
    let (event_tx, event_rx1) = broadcast::channel(10);
    let event_rx2 = event_tx.subscribe();

    let worker = ProverWorker::new(
        initial_state,
        event_rx2,
        event_tx.clone(),
        oracle_manager,
    );

    tokio::spawn(async move { worker.run().await });

    // Emit ActionExecuted event
    event_tx.send(GameEvent::ActionExecuted { ... }).unwrap();

    // Wait for ProofGenerated event
    let event = event_rx1.recv().await.unwrap();
    assert!(matches!(event, GameEvent::ProofGenerated { .. }));
}
```

### End-to-End Tests

**Full runtime with proving:**
```rust
#[tokio::test]
async fn test_e2e_runtime_with_proving() {
    let runtime = Runtime::builder()
        .enable_proving(true)
        .oracles(OracleManager::test_manager())
        .build()
        .await
        .unwrap();

    let mut events = runtime.subscribe_events();

    runtime.execute_action(Action::move_action(...)).await.unwrap();

    // Verify we receive both ActionExecuted and ProofGenerated
    let mut action_executed = false;
    let mut proof_generated = false;

    while let Ok(event) = timeout(Duration::from_secs(30), events.recv()).await {
        match event.unwrap() {
            GameEvent::ActionExecuted { .. } => action_executed = true,
            GameEvent::ProofGenerated { .. } => proof_generated = true,
            _ => {}
        }

        if action_executed && proof_generated {
            break;
        }
    }

    assert!(action_executed);
    assert!(proof_generated);
}
```

---

## Performance Considerations

### Proof Generation Time

**Estimated Performance:**
- Dev mode (RISC0_DEV_MODE=1): ~10-50ms
- Real proofs (RISC0_DEV_MODE=0): ~5-30 seconds (depends on action complexity)

**Optimization Strategies:**
1. **Async execution**: Use `tokio::spawn_blocking` for proving
2. **Batching**: Future optimization to prove multiple actions together
3. **Bonsai**: Offload to remote proving for production
4. **Caching**: Cache oracle snapshot (immutable)

### Memory Usage

**Oracle Snapshot Size:**
- Map (20x20): ~1-2KB
- Items: ~1-5KB (depending on item count)
- NPCs: ~1-5KB
- Tables: ~1KB
- **Total**: ~5-15KB per snapshot

**GameState Size:**
- Turn state: ~100 bytes
- Entities: ~1-10KB (depends on entity count)
- World: ~1-5KB
- **Total**: ~2-15KB per state

**Peak Memory:**
- Host: ~100MB (RISC0 prover overhead)
- Guest: ~10MB (execution environment)

### Optimization: Oracle Snapshot Caching

```rust
pub struct ProverWorker {
    // Cache oracle snapshot (immutable)
    oracle_snapshot: Arc<OracleSnapshot>,
    // ... other fields
}

impl ProverWorker {
    pub fn new(
        initial_state: GameState,
        event_rx: broadcast::Receiver<GameEvent>,
        event_tx: broadcast::Sender<GameEvent>,
        oracle_manager: Arc<OracleManager>,
    ) -> Self {
        // Create snapshot once
        let env = oracle_manager.as_game_env();
        let oracle_snapshot = Arc::new(OracleSnapshot::from_runtime_oracles(
            env.map().unwrap(),
            env.items().unwrap(),
            env.npcs().unwrap(),
            env.tables().unwrap(),
            env.config().unwrap(),
        ));

        Self {
            current_state: initial_state,
            oracle_snapshot,  // Cached, reused for all proofs
            // ...
        }
    }
}
```

---

## Open Questions & Future Work

### Q1: How to handle dynamic content updates?

**Problem**: Oracle data is cached in ProverWorker. If content changes (new items, map updates), proofs will be stale.

**Solutions**:
1. **Immutable content** (current): Content never changes during runtime session
2. **Content versioning**: Include content hash in proof, verify on-chain
3. **Dynamic snapshots**: Recreate snapshot on content update events

**Decision**: Use immutable content for Phase 1-5. Address in Phase 6 if needed.

### Q2: How to optimize proof generation for complex actions?

**Problem**: Some actions (combat with multiple effects) may take longer to prove.

**Solutions**:
1. **Async proving**: Already planned (tokio::spawn_blocking)
2. **Proof batching**: Prove multiple actions in single proof (Phase 6+)
3. **Selective proving**: Only prove player actions, skip NPC actions
4. **Bonsai**: Remote proving for production

**Decision**: Start with async proving. Evaluate batching in Phase 6.

### Q3: How to handle proof verification on-chain?

**Problem**: EVM verification of RISC0 proofs requires specific contract setup.

**Solutions**:
1. **Groth16 wrapping**: RISC0 supports Groth16 recursion for EVM compatibility
2. **Receipt verification**: Submit receipts to RISC0 verification contract
3. **Optimistic rollup**: Submit state roots, challenge with proofs

**Decision**: Phase 7+ (blockchain integration). Not in scope for initial implementation.

---

## Success Criteria

### Phase 1-5 Complete When:

- ✅ game-core types support serde serialization
- ✅ OracleSnapshot successfully created from runtime oracles
- ✅ Guest program compiles to RISC-V ELF
- ✅ Guest executes game logic and generates proofs (dev mode)
- ✅ Risc0Prover integrates with ProverWorker
- ✅ ProverWorker emits ProofGenerated events
- ✅ All tests pass (unit, integration, e2e)
- ✅ Dev mode proving: <100ms
- ✅ Real proving: <60s
- ✅ Documentation complete

---

## Appendix: Complete File Listing

### Files to Create

```
crates/zk/
├── build.rs                           # NEW
├── src/
│   ├── oracle.rs                      # NEW
│   └── zkvm/
│       └── risc0.rs                   # NEW
└── methods/
    └── guest/
        ├── Cargo.toml                 # NEW
        └── src/
            ├── main.rs                # NEW
            └── oracle.rs              # NEW
```

### Files to Modify

```
crates/
├── game/core/
│   ├── Cargo.toml                     # Add serde feature
│   └── src/
│       ├── lib.rs                     # Conditional serde derives
│       ├── action/*.rs                # Derive Serialize/Deserialize
│       ├── state/*.rs                 # Derive Serialize/Deserialize
│       └── env/*.rs                   # Derive Serialize/Deserialize
│
├── runtime/
│   ├── Cargo.toml                     # Enable game-core serde feature
│   ├── src/
│   │   ├── runtime.rs                 # Pass OracleManager to ProverWorker
│   │   └── workers/
│   │       └── prover.rs              # Create oracle snapshot, use Risc0Prover
│
└── zk/
    ├── Cargo.toml                     # Add risc0 deps, build-dependencies
    └── src/
        ├── lib.rs                     # include!(methods.rs), re-export ELF/ID
        └── zkvm/
            └── mod.rs                 # DefaultProver selection
```

---

*Last updated: 2025-01-15*
