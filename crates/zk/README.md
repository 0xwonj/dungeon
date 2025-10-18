# ZK Proof Generation

This crate provides zero-knowledge proof generation for game actions.

## Default Behavior: No Proving ⚡

**By default, this crate does NOT compile any proving code.** The game runs normally without any proof generation overhead.

```bash
cargo build        # No proving, fast compilation, game-only mode
cargo run          # Game works perfectly without proofs
```

This is ideal for development and testing where proof generation would slow things down.

To enable proving, use Cargo features (see below).

## Features

The crate supports multiple proving backends controlled by Cargo features:

### No Proving (Default) ⚡

**Default mode** - Game runs without any proof generation.

**Advantages:**
- ✅ Fast compilation (no ZK dependencies)
- ✅ Fast runtime (no proof overhead)
- ✅ Perfect for development and testing
- ✅ Smallest binary size

**Usage:**
```toml
# Default - no proving
zk = { path = "../zk" }
```

**What compiles:**
- Core types only (ProofData, ProofBackend)
- No zkVM code
- No custom circuit code

### zkVM ✅

Uses zkVM (SP1 or RISC0) to automatically prove execution traces.

**Advantages:**
- ✅ Simple implementation (no circuit design needed)
- ✅ Reuses existing `game-core` code 100%
- ✅ No manual witness generation
- ✅ Easy to maintain and debug

**Disadvantages:**
- ⚠️ Slower proof generation (5-60 seconds)
- ⚠️ Larger proof size (~200 KB compressed)

**Usage:**
```toml
# Enable zkVM stub prover
zk = { path = "../zk", features = ["zkvm"] }

# With SP1 backend (Phase 2B)
zk = { path = "../zk", features = ["sp1"] }

# With RISC0 backend (default)
zk = { path = "../zk", features = ["risc0"] }  # or just default = ["risc0"]
```

### Custom Circuit (Phase 2+) 🚧

Hand-crafted circuits with explicit Merkle witness generation.

**Advantages:**
- ✅ Fast proof generation (10-100 ms)
- ✅ Small proof size (~10 KB)
- ✅ Low on-chain verification cost

**Disadvantages:**
- ❌ Requires circuit expertise
- ❌ Complex implementation (StateTransition, Merkle trees)
- ❌ Months of development time
- ❌ Game logic changes require circuit updates

**Usage:**
```toml
# Custom circuit only (not zkVM)
zk = { path = "../zk", features = ["custom-circuit"] }

# Hybrid: both zkVM and custom circuit
zk = { path = "../zk", features = ["sp1", "custom-circuit"] }
```

**Status:** Not yet implemented. Placeholder code exists but will error if called.

## Runtime Configuration

The `runtime` crate also needs to enable proving:

```rust
// Default: proving disabled
let runtime = Runtime::builder()
    .oracles(oracles)
    .build().await?;

// Enable proving (ProverWorker spawned)
let runtime = Runtime::builder()
    .oracles(oracles)
    .enable_proving(true)  // ← Must set this!
    .build().await?;
```

**Both conditions must be met for proving to work:**
1. ✅ `zk` crate compiled with proving features (`zkvm` or `custom-circuit`)
2. ✅ `RuntimeConfig.enable_proving = true`

## Architecture

### No Proving (Default)

```
SimulationWorker
  ↓
ActionExecuted(delta, before, after)
  ↓
Clients (UI updates from delta)
```

**No ProverWorker spawned. No proof events emitted.**

### With Proving Enabled

```
SimulationWorker                ProverWorker
  ↓                                 ↓
ActionExecuted              (subscribes to events)
  ↓                                 ↓
Clients                     ProofGenerated/Failed
```

**ProverWorker spawned when `enable_proving=true`.**

### zkVM Flow (When Enabled)

```
ProverWorker
  ↓
zkvm::prove(before_state, action, after_state)
  ↓
zkVM Guest Program executes game logic
  ↓
Proof (execution trace)
```

**No Merkle trees or witnesses needed!**

### Custom Circuit Flow (Future)

```
ProverWorker
  ↓
StateDelta (from game-core)
  ↓
StateTransition (Merkle witnesses)
  ↓
Circuit (constraint system)
  ↓
Proof (compact)
```

**Requires Merkle tree construction and witness generation.**

## Implementation Phases

### Phase 1: No Proving ✅ (Current)

- [x] Default feature = no proving
- [x] Game runs without proof overhead
- [x] ProverWorker disabled by default in runtime
- [x] Fast compilation and runtime

### Phase 2A: Stub zkVM ✅

- [x] Feature flag structure
- [x] ProofBackend enum
- [x] StubZkvmProver (returns dummy proofs)
- [x] Module structure for future backends

### Phase 2B: RISC0 Integration ✅ (Complete)

- [x] Add RISC0 zkVM dependency
- [x] Write guest program (proves game execution)
- [x] Implement Risc0Prover with oracle snapshots
- [x] Build script for guest compilation
- [x] Integration with runtime ProverWorker
- [x] Production and development mode support

### Phase 2C: SP1 Integration (Future)

- [ ] Add SP1 SDK dependency
- [ ] Port guest program to SP1
- [ ] Implement Sp1Prover

### Phase 3: Custom Circuit (Future)

Only implement when:
- Proof generation time becomes a bottleneck
- On-chain verification costs are too high
- Team has bandwidth for multi-month project

Requires:
- [ ] Sparse Merkle tree implementation
- [ ] StateTransition structure
- [ ] Witness generation from StateDelta
- [ ] Circuit definitions for each action type
- [ ] Constraint system integration (Halo2/Plonky2)

## Usage Example

### Development (No Proving)

```rust
// Default configuration
let runtime = Runtime::builder()
    .oracles(oracles)
    .build().await?;

// Game works normally, no proofs generated
runtime.step().await?;
```

### With RISC0 Proving

```toml
# Cargo.toml (risc0 is default)
zk = { path = "../zk" }
```

```bash
# Production mode (real proofs, 30-60 seconds per action)
ENABLE_ZK_PROVING=1 cargo run -p client-cli

# Development mode (mock proofs, <100ms per action)
ENABLE_ZK_PROVING=1 RISC0_DEV_MODE=1 cargo run -p client-cli
```

```rust
let runtime = Runtime::builder()
    .oracles(oracles)
    .enable_proving(true)  // Enable ProverWorker
    .build().await?;

// Subscribe to proof events
let mut events = runtime.subscribe_events();
while let Ok(event) = events.recv().await {
    match event {
        GameEvent::ProofGenerated { generation_time_ms, .. } => {
            println!("Proof generated in {}ms", generation_time_ms);
        }
        _ => {}
    }
}
```

## StateDelta vs Witnesses

### StateDelta (Always Used)

Bitmask-based change tracking created by `game-core`:

```rust
pub struct StateDelta {
    pub action: Action,
    pub entities: EntityChanges,  // Bitmasks only
    pub world: WorldChanges,
}
```

**Uses:**
- ✅ UI updates (which fields changed?)
- ✅ Network bandwidth optimization
- ✅ Audit trails and replay
- ✅ Optimization hints for proving

**Not used for zkVM witness generation** (zkVM generates execution traces automatically)

### Witnesses (Custom Circuit Only)

Merkle proofs generated from StateDelta:

```rust
pub struct StateTransition {
    pub before_root: StateRoot,
    pub after_root: StateRoot,
    pub witnesses: Vec<EntityWitness>,  // Merkle proofs
}
```

**Only needed for custom circuits** (not zkVM!)

## Design Decisions

### Why No Proving by Default?

1. **Faster Development**: No ZK compilation overhead during iteration
2. **Simpler Testing**: Game logic can be tested without proof infrastructure
3. **Optional Feature**: Proving is an add-on, not a requirement
4. **Progressive Enhancement**: Add proving when needed, not from day one

### Why zkVM First (When Enabling)?

1. **Faster Development**: 2-3 days vs. 2-3 months
2. **Flexibility**: Game logic can change freely
3. **Maintainability**: No circuit expertise needed
4. **Proven Technology**: SP1/RISC0 are production-ready

### When to Use Custom Circuit?

Only when profiling shows:
- Proof generation time > 10 seconds per action
- On-chain verification cost > $1 per proof
- These become real bottlenecks (not premature optimization)

### Why Feature Flags?

- Keeps proving code dormant by default (doesn't compile)
- Clear separation between proving approaches
- Easy to enable proving later without breaking changes
- Supports hybrid deployments (zkVM + custom circuit)

## Binary Size Comparison

```bash
# Default (no proving)
cargo build --release
# Binary size: ~5 MB

# With zkVM stub
cargo build --release --features zkvm
# Binary size: ~5 MB (stub is tiny)

# With SP1 (when implemented)
cargo build --release --features sp1
# Binary size: ~50 MB (SP1 SDK is large)

# Custom circuit only
cargo build --release --features custom-circuit
# Binary size: ~10 MB (circuit + crypto)
```

## See Also

- [State Delta Architecture](../../docs/state-delta-architecture.md) - Design rationale for delta system
- [ZK Feature Flags Guide](../../docs/zk-feature-flags.md) - Detailed feature flag usage
- [game-core](../game/core/) - Deterministic game engine
- [runtime](../runtime/) - ProverWorker integration
