# State Delta Architecture for ZK-Provable Game Systems

**Status**: Design Document
**Version**: 1.0
**Last Updated**: 2025-10-15
**Authors**: Architecture Team

---

## Executive Summary

This document describes the architecture for state change tracking (deltas) in a deterministic, ZK-provable turn-based game system. The design separates game execution concerns from zero-knowledge proof generation concerns through a three-layer architecture:

1. **Game Core Layer**: Lightweight bitmask-based change tracking optimized for game execution
2. **ZK Transformation Layer**: Merkle tree construction and witness generation for proof circuits
3. **Circuit Layer**: Minimal constraint generation using only changed state elements

This separation enables independent optimization of each layer while maintaining a clear data flow from game actions to verifiable proofs.

**Key Design Principles:**
- **Separation of Concerns**: Game logic remains independent of cryptographic primitives
- **Deferred Computation**: ZK-specific data structures are built only when needed
- **Minimal Tracking**: Deltas store only metadata (which fields changed), not values
- **Scalability**: Architecture supports state growth from KBs to MBs without redesign

---

## Table of Contents

1. [Background and Motivation](#1-background-and-motivation)
2. [System Requirements](#2-system-requirements)
3. [Architecture Overview](#3-architecture-overview)
4. [Layer 1: Game Core Delta](#4-layer-1-game-core-delta)
5. [Layer 2: ZK Transformation](#5-layer-2-zk-transformation)
6. [Layer 3: Circuit Integration](#6-layer-3-circuit-integration)
7. [Design Rationale](#7-design-rationale)
8. [Alternative Approaches](#8-alternative-approaches)
9. [Performance Characteristics](#9-performance-characteristics)
10. [Implementation Phases](#10-implementation-phases)
11. [Future Extensions](#11-future-extensions)

---

## 1. Background and Motivation

### 1.1 Problem Statement

In ZK-provable game systems, every state transition must be:
- **Deterministically reproducible**: Same action on same state yields same result
- **Efficiently provable**: Proof generation time must scale sub-linearly with state size
- **Bandwidth efficient**: Network transmission of state changes should be minimal
- **Developer friendly**: Game logic should not require cryptographic expertise

Traditional approaches suffer from trade-offs:
- Storing full before/after states: High memory cost, simple implementation
- Computing granular patches with values: High CPU cost, complex diffing logic
- Using Merkle trees everywhere: Poor developer experience, slow game execution

### 1.2 Design Goals

1. **Performance**: Game execution should be fast (microseconds per action)
2. **Scalability**: Support state growth from 10KB to 10MB without architectural changes
3. **Clarity**: Clear separation between game logic and cryptographic concerns
4. **Efficiency**: Minimal memory overhead for delta tracking
5. **Extensibility**: Easy to add new state fields or entity types

---

## 2. System Requirements

### 2.1 Functional Requirements

**FR-1**: Track which fields changed during state transitions
**FR-2**: Support delta-to-proof conversion without full state re-computation
**FR-3**: Enable ZK circuits to skip proving unchanged state sections
**FR-4**: Provide audit trail of state changes for debugging and replay
**FR-5**: Support both synchronous (runtime) and asynchronous (prover) workflows

### 2.2 Non-Functional Requirements

**NFR-1**: Delta creation ≤ 10 microseconds for typical actions
**NFR-2**: Delta memory footprint ≤ 1KB for typical actions
**NFR-3**: ZK conversion ≤ 100 milliseconds for typical actions
**NFR-4**: Zero cryptographic dependencies in game core
**NFR-5**: Support testing without ZK infrastructure

### 2.3 Scale Assumptions

- **State size**: 10KB - 10MB (entities, world data, game state)
- **Action frequency**: 1-100 actions per second
- **Entity count**: 10 - 10,000 entities (players, NPCs, items)
- **Proof batch size**: 1-1000 actions per proof

---

## 3. Architecture Overview

### 3.1 Three-Layer Design

```
┌─────────────────────────────────────────────────────────┐
│                   Game Core Layer                       │
│  ┌─────────────┐         ┌──────────────┐              │
│  │ GameState   │ ──────▶ │ StateDelta   │              │
│  │ (Vec, Map)  │  diff   │ (Bitmask)    │              │
│  └─────────────┘         └──────┬───────┘              │
│         Concerns: Execution speed, developer UX         │
└────────────────────────────────────┬────────────────────┘
                                     │ conversion
                                     ▼
┌─────────────────────────────────────────────────────────┐
│                ZK Transformation Layer                  │
│  ┌──────────────┐       ┌────────────────────┐         │
│  │ StateDelta   │ ────▶ │ StateTransition    │         │
│  │ + GameState  │ build │ (Merkle witnesses) │         │
│  └──────────────┘       └─────────┬──────────┘         │
│     Concerns: Proof size, witness generation            │
└────────────────────────────────────┬────────────────────┘
                                     │ proving
                                     ▼
┌─────────────────────────────────────────────────────────┐
│                   Circuit Layer                         │
│  ┌────────────────────┐       ┌──────────┐             │
│  │ StateTransition    │ ────▶ │ Proof    │             │
│  │ (witnesses)        │ prove │ (bytes)  │             │
│  └────────────────────┘       └──────────┘             │
│     Concerns: Constraint count, verification cost       │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Data Flow

1. **Action Execution**: GameEngine applies action to GameState
2. **Delta Creation**: Compare before/after states, generate bitmask delta
3. **Event Broadcasting**: Runtime broadcasts delta to clients/observers
4. **ZK Conversion** (async): Convert delta + states to StateTransition
5. **Proof Generation**: Circuit proves transition validity
6. **On-chain Submission**: Proof submitted to blockchain for verification

### 3.3 Key Insight: Deferred Merkle Tree Construction

Merkle trees are **not** maintained during game execution. Instead:
- Game executes with efficient data structures (Vec, HashMap)
- Merkle trees are **built on-demand** during ZK conversion
- Bitmask delta guides which tree sections to build
- Incremental tree updates can be added later if needed

---

## 4. Layer 1: Game Core Delta

### 4.1 Design Philosophy

**Principle**: Deltas store *metadata about changes*, not the changed values themselves.

Rationale:
- Values are already in before/after GameState
- Storing values doubles memory usage
- Bitmasks provide sufficient information for:
  - Client-side UI updates (query fields from after state)
  - ZK conversion (knows which fields to prove)
  - Audit trails (combined with before/after snapshots)

### 4.2 Delta Structure

```
StateDelta
├── action: Action                 // What was executed
├── clock: Tick                    // When it executed
├── turn: TurnChanges              // Turn state changes
│   ├── flags: TurnFlags           // Bitmask (CLOCK | CURRENT_ACTOR)
│   ├── activated: Vec<EntityId>   // Entities activated this turn
│   └── deactivated: Vec<EntityId> // Entities deactivated this turn
├── entities: EntityChanges        // Entity changes
│   ├── player: Option<ActorChanges>
│   ├── npcs: CollectionChanges<ActorChanges>
│   ├── props: CollectionChanges<PropChanges>
│   └── items: CollectionChanges<ItemChanges>
└── world: WorldChanges            // World state changes
    └── occupancy: Vec<Position>   // Tiles with occupancy changes
```

### 4.3 Change Tracking Granularity

#### Entity-Level Changes

```
ActorChanges
├── id: EntityId              // Which actor changed
└── fields: ActorFields       // Bitmask of changed fields
    ├── POSITION    (1 << 0)
    ├── CORE_STATS  (1 << 1)
    ├── RESOURCES   (1 << 2)
    ├── BONUSES     (1 << 3)
    ├── INVENTORY   (1 << 4)
    └── READY_AT    (1 << 5)
```

Each entity type (Actor, Prop, Item) has its own field bitmask tailored to its structure.

#### Collection-Level Changes

```
CollectionChanges<T>
├── added: Vec<EntityId>      // New entities
├── removed: Vec<EntityId>    // Deleted entities
└── updated: Vec<T>           // Modified entities (with field bitmasks)
```

### 4.4 Delta Creation Algorithm

```
Algorithm: CreateStateDelta(action, before_state, after_state)
Input:
  - action: The executed action
  - before_state: State snapshot before action
  - after_state: State snapshot after action
Output:
  - delta: StateDelta with bitmask metadata

1. Initialize empty delta with action and clock

2. For each state section (turn, entities, world):
   a. Compare before and after structures
   b. Set bitmask flags for changed scalar fields
   c. Compute collection diffs (added/removed/updated)
   d. For updated entities:
      i. Compare field-by-field
      ii. Set bitmask for each changed field
      iii. Store entity ID + field bitmask

3. Return completed delta

Complexity: O(n) where n = number of entities
Memory: O(k) where k = number of changed entities (typically << n)
```

### 4.5 Benefits

1. **Minimal Memory**: 5-50 bytes per changed entity (vs. 100-500 bytes with full values)
2. **Fast Creation**: Simple field comparisons, no deep cloning
3. **Network Efficient**: Small payloads for real-time clients
4. **Clear Intent**: Bitmask explicitly shows what changed

### 4.6 Example: Player Movement

```
Action: Move(player, direction=North)

Before State:
  entities.player.position = (5, 5)
  entities.player.resources.hp = 100
  world.occupancy[(5,5)] = [player_id]
  world.occupancy[(5,6)] = []

After State:
  entities.player.position = (5, 6)
  entities.player.resources.hp = 100
  world.occupancy[(5,5)] = []
  world.occupancy[(5,6)] = [player_id]

Generated Delta:
  StateDelta {
    action: Move(player, North),
    entities: EntityChanges {
      player: Some(ActorChanges {
        id: player_id,
        fields: ActorFields::POSITION  // Only position changed
      }),
      npcs: empty,
      props: empty,
      items: empty
    },
    world: WorldChanges {
      occupancy: [(5,5), (5,6)]  // Two tiles affected
    }
  }

Delta size: ~30 bytes (vs. ~20KB for full state clone)
```

---

## 5. Layer 2: ZK Transformation

### 5.1 Purpose

Convert game-native delta into a ZK-circuit-friendly representation using Merkle trees and cryptographic commitments.

### 5.2 StateTransition Structure

```
StateTransition
├── action: Action
├── clock: Tick
├── before_root: StateRoot          // Merkle commitment
│   ├── turn_hash: Hash
│   ├── entities_root: Hash
│   └── world_root: Hash
├── after_root: StateRoot
└── witnesses: TransitionWitnesses  // Merkle proofs
    ├── turn_before: Option<TurnWitness>
    ├── turn_after: Option<TurnWitness>
    ├── entities: Vec<EntityWitness>
    └── world: Vec<TileWitness>
```

### 5.3 Merkle Tree Strategy

#### Hierarchical Commitment Scheme

```
StateRoot = Hash(turn_hash || entities_root || world_root)

entities_root = MerkleRoot([
  entity_1_leaf,
  entity_2_leaf,
  ...,
  entity_n_leaf
])

entity_leaf = Hash(entity_type || entity_id || entity_data)
```

**Key Property**: Only changed entities require Merkle proofs. Unchanged entities are represented by their hash in the tree without generating witnesses.

#### Tree Construction

Merkle trees are **built ephemerally** during conversion:

```
Algorithm: BuildEntityTree(entities_state)
1. Create sparse merkle tree
2. For each entity in state:
   a. Serialize entity to bytes
   b. Hash to create leaf value
   c. Insert (entity_id -> leaf) into tree
3. Compute and cache tree root
4. Return tree (ready for witness generation)

Complexity: O(n log n) where n = entity count
Optimization: Incremental trees (future work)
```

### 5.4 Witness Generation

```
Algorithm: GenerateWitnesses(delta, before_state, after_state)
1. Build before_tree from before_state.entities
2. Build after_tree from after_state.entities

3. For each changed entity in delta.entities:
   a. Query entity data from before_state
   b. Generate before_witness = before_tree.prove(entity_id)
   c. Query entity data from after_state
   d. Generate after_witness = after_tree.prove(entity_id)
   e. Extract changed_fields bitmask from delta
   f. Create EntityWitness(id, before, after, witnesses, bitmask)

4. Repeat for world tiles using delta.world.occupancy

5. Return TransitionWitnesses

Complexity: O(k log n) where k = changed entities, n = total entities
Key insight: k << n (typically 1-10 vs. 100-1000)
```

### 5.5 EntityWitness Structure

```
EntityWitness
├── id: EntityId
├── before: EntityLeaf
│   ├── entity_type: Actor/Prop/Item
│   └── hash: Hash(serialized entity)
├── before_proof: MerkleWitness
│   ├── siblings: Vec<Hash>  // Merkle path to root
│   └── path_bits: Vec<bool> // Left/right directions
├── after: EntityLeaf
├── after_proof: MerkleWitness
└── changed_fields: EntityFieldMask  // From delta bitmask
```

### 5.6 Optimization: Selective Tree Building

Not all state sections need Merkle trees:

- **Turn state**: Small (< 1KB), included directly in witness
- **Entities**: Large, benefit from Merkle trees
- **World occupancy**: Large, benefit from Merkle trees

```
if delta.turn.is_empty():
  witnesses.turn = None  // Skip entirely
else:
  witnesses.turn = TurnWitness::from_state(&after.turn)  // No Merkle tree
```

### 5.7 Conversion Performance

**Input**: StateDelta (30 bytes) + GameState (20 KB)
**Output**: StateTransition (~1-5 KB depending on changes)
**Time**: 1-10 milliseconds (dominated by Merkle tree construction)
**Parallelization**: Can be offloaded to separate thread/process

---

## 6. Layer 3: Circuit Integration

### 6.1 Circuit Responsibilities

The ZK circuit proves:
```
∀ witness ∈ StateTransition.witnesses:
  1. witness.before is in before_tree (Merkle verification)
  2. witness.after is in after_tree (Merkle verification)
  3. transition(witness.before, action) = witness.after (game logic)
  4. Only fields in witness.changed_fields were modified
```

### 6.2 Constraint Generation Strategy

```
For each EntityWitness:
  1. Verify before_proof against before_root (log n constraints)
  2. Verify after_proof against after_root (log n constraints)
  3. For each bit in changed_fields bitmask:
     a. If bit is 0 (unchanged):
        - Constrain: before.field == after.field (1 constraint)
     b. If bit is 1 (changed):
        - Apply game logic constraint (varies by field)
        - Example: position change validates as legal move
  4. Hash after entity, constrain against after leaf (varies)

Total constraints per entity: 2 log n + field_count + game_logic
```

### 6.3 Key Optimization: Skipping Unchanged Entities

Entities not in `delta.entities` are **proven implicitly**:
- Their leaves exist in both before and after trees
- Since the tree is deterministic, unchanged leaves → unchanged data
- No explicit constraints needed (zero cost!)

**Savings**: If 1000 entities exist but only 5 change:
- Naive: Prove all 1000 transitions (expensive)
- Optimized: Prove only 5 transitions + tree consistency (cheap)

### 6.4 Circuit Input Layout

```
Public Inputs:
  - before_root.commitment: Hash
  - after_root.commitment: Hash
  - action: serialized Action

Private Inputs (Witnesses):
  - For each changed entity:
    - before_entity: serialized entity data
    - before_proof: Merkle path
    - after_entity: serialized entity data
    - after_proof: Merkle path
    - changed_fields: bitmask
```

### 6.5 Verification Cost

On-chain verification only checks:
1. Proof validity (circuit-specific)
2. Public inputs match (before_root, after_root, action)

**No need to re-execute game logic on-chain** (this is the ZK advantage).

---

## 7. Design Rationale

### 7.1 Why Separate Game and ZK Layers?

#### Decision: Two data structures instead of one

**Alternative**: Use Merkle trees everywhere (single structure for game and ZK)

**Rejected because**:
- Merkle tree operations are 100-1000x slower than direct field access
- Game logic becomes cryptographically coupled (poor developer UX)
- Debugging requires understanding tree structure
- Testing requires Merkle tree infrastructure

**Chosen approach benefits**:
- Game developers write normal Rust with Vec/HashMap
- ZK complexity isolated in separate crate
- Easy to test game logic without ZK
- Can swap ZK backend without touching game code

#### Cost-Benefit Analysis

| Aspect | Single Structure (Merkle) | Separate Structures |
|--------|---------------------------|---------------------|
| Game execution | ❌ Slow (1-10ms/action) | ✅ Fast (1-10μs/action) |
| ZK generation | ✅ Instant (tree exists) | 🟡 Fast (1-10ms build) |
| Developer UX | ❌ Poor (crypto required) | ✅ Excellent (normal code) |
| Testing | ❌ Complex | ✅ Simple |
| Maintenance | ❌ Coupled | ✅ Independent |

**Conclusion**: 1-10ms conversion cost is negligible compared to developer productivity gains.

### 7.2 Why Bitmask Instead of Value Storage?

#### Decision: Store "what changed" not "how it changed"

**Alternative**: Store changed field values in delta (like traditional patches)

```
ActorPatch {
  id: EntityId,
  position: Option<Position>,      // Store actual new position
  resources: Option<Resources>,    // Store actual new resources
  ...
}
```

**Rejected because**:
- Doubles memory usage (values stored in both after_state and delta)
- Requires cloning complex types (Resources, Inventory)
- Adds 100-200 bytes per changed entity
- Values are already accessible via after_state

**Chosen approach benefits**:
- 5-10 bytes per changed entity (vs. 100-200)
- No cloning overhead
- Clear separation: delta = metadata, state = data
- Sufficient for all use cases (UI, ZK, audit)

#### Use Case Coverage

| Use Case | Needs Values? | Solution |
|----------|---------------|----------|
| Client UI update | Yes | Query after_state using entity ID |
| ZK witness generation | Yes | Query before/after states during conversion |
| Network transmission | No | Send bitmask, client has state |
| Audit log | Yes | Store before/after snapshots separately |

### 7.3 Why Deferred Merkle Tree Construction?

#### Decision: Build trees during conversion, not during execution

**Alternative**: Maintain incremental Merkle trees during game execution

```rust
pub struct GameState {
    entities: EntitiesState,
    entities_tree: IncrementalMerkleTree,  // Kept in sync
}
```

**Rejected because**:
- Every entity change triggers tree update (hash recomputation)
- Adds 1-5 microseconds per change (5-10x slowdown)
- Couples game logic to cryptographic operations
- Increases state size (tree structure overhead)

**Chosen approach benefits**:
- Game execution unaffected by ZK concerns
- Trees built only when needed (async prover workflow)
- Can use different tree implementations without touching game
- Future optimization: cached incremental trees (if needed)

#### Performance Comparison

| Operation | Incremental Tree | Deferred Tree |
|-----------|------------------|---------------|
| Entity update | 5-10μs (hash + tree) | 1μs (field write) |
| Delta creation | 1μs (tree is ready) | 1μs (bitmask compare) |
| ZK conversion | 0μs (tree exists) | 5ms (build from scratch) |
| **Total per action** | **6-11μs** | **1μs + 5ms (async)** |

**Key insight**: 5ms async cost is acceptable because proof generation takes seconds anyway.

### 7.4 Why Field-Level Granularity?

#### Decision: Track changes at individual field level (position, hp, etc.)

**Alternative**: Track changes at entity level only (entire entity changed)

**Rejected because**:
- ZK circuits would prove entire entity transitions (expensive)
- Example: HP change would also prove position unchanged (wasted constraints)
- Bandwidth waste (send entire entity over network)

**Chosen approach benefits**:
- Circuit proves only changed fields (minimal constraints)
- Network sends only changed fields
- Clear audit trail (exactly which stat changed)

**Granularity Trade-off**:
- Too coarse (entity-level): Wastes ZK constraints
- Too fine (byte-level): Excessive tracking overhead
- Just right (field-level): Balances ZK efficiency and tracking cost

---

## 8. Alternative Approaches

### 8.1 Full Merkle State (Alternative 1)

**Description**: Use Merkle trees as primary data structure for game execution.

**Evaluation**:
- ✅ ZK-native (no conversion needed)
- ✅ State commitments always available
- ❌ 100-1000x slower game execution
- ❌ Poor developer experience
- ❌ Complex debugging

**Verdict**: Unsuitable for real-time games. May work for turn-based games with long turns (minutes).

### 8.2 Event Sourcing (Alternative 2)

**Description**: Store deltas only, reconstruct state by replay.

**Evaluation**:
- ✅ Complete audit trail
- ✅ Time-travel debugging
- ✅ Deltas self-contained
- ❌ State queries require replay (slow)
- ❌ Memory grows unbounded
- 🟡 Complexity moderate (need snapshots)

**Verdict**: Complementary approach. Can be added on top of current design for audit logs.

### 8.3 Copy-on-Write State (Alternative 5)

**Description**: Use `Arc<T>` for structural sharing between before/after states.

```rust
pub struct GameState {
    entities: Arc<EntitiesState>,
    world: Arc<WorldState>,
}
```

**Evaluation**:
- ✅ Cheap state cloning (pointer copy)
- ✅ Memory efficient (shared unchanged sections)
- ❌ Requires immutable update patterns
- ❌ Arc overhead (reference counting)
- 🟡 Moderate complexity

**Verdict**: Valuable optimization if state cloning becomes bottleneck. Current design can migrate to COW without API changes.

### 8.4 Differential State (Alternative 4)

**Description**: Store forward and reverse deltas for bidirectional traversal.

**Evaluation**:
- ✅ Undo/redo support
- ✅ Bidirectional replay
- ❌ 2x delta size (forward + reverse)
- ❌ Complex to implement correctly
- ❌ Reverse delta bugs hard to catch

**Verdict**: Niche use case. Not worth complexity for most games.

### 8.5 ZK-First Design (Alternative 6)

**Description**: Define state structure in circuit, game interprets circuit execution.

**Evaluation**:
- ✅ Perfect ZK alignment
- ✅ Formal verification possible
- ❌ Extremely difficult to develop
- ❌ Inflexible (circuit changes = full rewrite)
- ❌ Poor debugging

**Verdict**: Research-grade approach. Not production-ready.

### 8.6 Why Current Design Wins

The chosen design (separate structures with bitmask deltas) balances:
- **Development velocity**: Normal Rust code for game logic
- **Performance**: Fast execution, acceptable conversion time
- **Scalability**: Handles growth without redesign
- **Flexibility**: Can add optimizations (COW, incremental trees) later

---

## 9. Performance Characteristics

### 9.1 Analytical Complexity

| Operation | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| State clone | O(n) | O(n) | n = state size; ~1-2μs per KB |
| Delta creation | O(k) | O(k) | k = changed entities |
| Merkle tree build | O(n log n) | O(n) | n = entity count |
| Witness generation | O(k log n) | O(k log n) | k witnesses, each log n size |
| Circuit proving | O(k log n + m) | O(k log n + m) | m = game logic constraints |

### 9.2 Empirical Benchmarks (Estimated)

Based on typical game state:
- 1000 entities
- 20 KB total state
- 5 entities changed per action

| Operation | Time | Memory | Throughput |
|-----------|------|--------|------------|
| State clone | 10 μs | 20 KB | 100k ops/s |
| Delta creation | 2 μs | 50 bytes | 500k ops/s |
| Merkle tree (1000 entities) | 5 ms | 100 KB | 200 ops/s |
| Witness gen (5 entities) | 500 μs | 5 KB | 2k ops/s |
| ZK conversion (total) | 6 ms | 105 KB | 150 ops/s |

**Key Takeaway**: ZK conversion is 3000x slower than delta creation, but still fast enough for async processing.

### 9.3 Scalability Projections

| State Size | Entity Count | Clone Time | ZK Conversion | Max Actions/s |
|------------|-------------|------------|---------------|---------------|
| 10 KB | 100 | 5 μs | 2 ms | 500 |
| 100 KB | 1,000 | 50 μs | 10 ms | 100 |
| 1 MB | 10,000 | 500 μs | 50 ms | 20 |
| 10 MB | 100,000 | 5 ms | 200 ms | 5 |

**Note**: Actions/s limited by ZK conversion. With parallel proving, can batch 100 actions into single proof.

### 9.4 Optimization Opportunities

When profiling reveals bottlenecks:

1. **State cloning** → Implement COW (Alternative 5)
2. **Tree building** → Use incremental Merkle trees
3. **Witness generation** → Cache stable subtrees
4. **Circuit proving** → Proof composition (prove sections independently)

---

## 10. Implementation Phases

### Phase 1: Core Delta System

**Goal**: Replace current `FieldDelta<T>` patches with bitmask-based changes.

**Deliverables**:
- `StateDelta` with `ChangesBitmask` structure
- `ActorChanges`, `PropChanges`, `ItemChanges` with field bitmasks
- `CollectionChanges<T>` generic tracker
- `StateDelta::from_states()` implementation
- Unit tests for delta creation

**Success Criteria**:
- All existing tests pass
- Delta size < 1KB for typical actions
- Delta creation < 10 μs

### Phase 2: ZK Transformation Layer

**Goal**: Implement `StateTransition` conversion with Merkle trees.

**Deliverables**:
- `crates/zk/src/state/` module structure
- Sparse Merkle tree implementation
- `StateTrees::from_state()` builder
- `StateTransition::from_delta()` converter
- Witness generation for entities and tiles
- Integration tests (delta → transition → verify structure)

**Success Criteria**:
- Conversion < 100 ms for 1000 entities
- Witness size scales with changed entities, not total entities
- No cryptographic dependencies in `game-core`

### Phase 3: Circuit Integration

**Goal**: Use `StateTransition` in actual ZK circuit.

**Deliverables**:
- Circuit definition (using SP1/RISC0 or custom)
- Merkle proof verification in circuit
- Game logic constraints for each action type
- Proof generation pipeline
- End-to-end test: action → delta → transition → proof → verify

**Success Criteria**:
- Proof generation < 10 seconds for single action
- Proof size < 100 KB
- Verification < 100 ms

### Phase 4: Optimization (Week 13+)

**Goal**: Optimize based on profiling data.

**Potential Optimizations**:
- Incremental Merkle trees (if tree building is bottleneck)
- COW state (if cloning is bottleneck)
- Proof batching (prove 100 actions together)
- Parallel witness generation
- Cached subtrees for stable state sections

**Approach**: Measure first, optimize bottlenecks.

---

## 11. Future Extensions

### 11.1 Incremental Merkle Trees

**Motivation**: Avoid rebuilding entire tree on every conversion.

**Approach**:
- Maintain tree in `SimulationWorker` alongside `GameState`
- Update tree incrementally as entities change
- Trade-off: Couples runtime to tree maintenance, but faster ZK conversion

**When to implement**: If tree building exceeds 50ms for typical state.

### 11.2 State Checkpointing

**Motivation**: Enable replay, time-travel debugging, rollback.

**Approach**:
```rust
pub struct StateHistory {
    checkpoints: Vec<(Tick, GameState)>,  // Every 100 turns
    deltas: Vec<StateDelta>,              // All deltas since last checkpoint
}
```

**Use cases**:
- Replay specific turns for debugging
- Rollback to previous checkpoint on critical bug
- Generate proofs for historical states

### 11.3 Proof Composition

**Motivation**: Prove batches of actions more efficiently.

**Approach**:
- Prove each action individually (parallel)
- Compose proofs using proof aggregation (Groth16, PLONK)
- Submit single aggregated proof on-chain

**Benefit**: Amortize verification cost across many actions.

### 11.4 Differential Compression

**Motivation**: Reduce network bandwidth for delta transmission.

**Approach**:
- Compress bitmasks (many zeros)
- Delta encoding for entity IDs (often sequential)
- Custom binary format (vs. JSON)

**Expected savings**: 50-70% size reduction.

### 11.5 Multi-Level State Roots

**Motivation**: Prove subsections independently.

**Approach**:
```rust
pub struct StateRoot {
    turn_hash: Hash,
    entities: SubsectionRoots {
        players_root: Hash,
        npcs_root: Hash,
        props_root: Hash,
        items_root: Hash,
    },
    world: SubsectionRoots {
        tiles_root: Hash,
        regions_root: Hash,
    },
}
```

**Benefit**: Prove "player movement" without proving "NPC states unchanged".

---

## Appendix A: Glossary

**Action**: A game command (Move, Attack, UseItem) that transitions state.

**Bitmask**: A compact representation of boolean flags using bit operations.

**Commitment**: A cryptographic hash that represents data without revealing it.

**Delta**: A description of changes between two states.

**Merkle Tree**: A hash tree enabling efficient proofs of inclusion.

**Merkle Witness**: A Merkle path (sibling hashes) proving a leaf exists in a tree.

**State**: The complete game world data at a specific point in time.

**State Transition**: The transformation of state by an action, with ZK proofs.

**ZK Proof**: A zero-knowledge proof that a computation was performed correctly without revealing inputs.

---

## Appendix B: References

### Related Work

1. **Ethereum State Trees**: Patricia Merkle Trees for account state
2. **zkSync**: Recursive proof composition for L2 scaling
3. **StarkNet**: STARK-based state transitions for Cairo VM
4. **Mina Protocol**: Fixed-size recursive proofs for blockchain state
5. **Dark Forest**: ZK game with Merkle-based fog of war

### Academic Papers

- "Incrementally Verifiable Computation" (Valiant, 2008)
- "Merkle Trees and Their Application" (Merkle, 1987)
- "Zerocash: Decentralized Anonymous Payments" (Sasson et al., 2014)

### Technical Resources

- SP1 zkVM Documentation: https://docs.succinct.xyz/
- RISC Zero zkVM: https://dev.risczero.com/
- "Merkle Tree in Rust": https://docs.rs/merkle/

---

## Appendix C: Decision Log

### Decision 1: Bitmask vs. Value Storage
- **Date**: 2025-10-15
- **Decision**: Use bitmask-only deltas
- **Rationale**: Memory efficiency (10x reduction) without losing functionality
- **Trade-off**: Must query states for actual values (acceptable)

### Decision 2: Deferred vs. Incremental Trees
- **Date**: 2025-10-15
- **Decision**: Build trees on-demand during conversion
- **Rationale**: Decouples game execution from ZK concerns
- **Trade-off**: 5-10ms conversion overhead (acceptable for async workflow)

### Decision 3: Field-Level vs. Entity-Level Tracking
- **Date**: 2025-10-15
- **Decision**: Field-level granularity
- **Rationale**: Minimizes ZK circuit constraints for sparse changes
- **Trade-off**: Slightly more complex delta structure (manageable)

### Decision 4: Separate vs. Unified Structures
- **Date**: 2025-10-15
- **Decision**: Separate game and ZK data structures
- **Rationale**: Developer productivity and execution performance
- **Trade-off**: Conversion cost and dual maintenance (acceptable)

---

## Appendix D: Open Questions

### Q1: Optimal Checkpoint Frequency
**Question**: How often should we snapshot full state for replay?
**Options**: Every 10/100/1000 turns, or adaptive based on proof batching?
**Status**: Deferred to Phase 4 (optimization)

### Q2: Merkle Tree Arity
**Question**: Binary trees (2-arity) or wider trees (4/8-arity)?
**Trade-off**: Wider = fewer levels (shorter proofs) but larger per-level hashes
**Status**: Start with binary, benchmark alternatives

### Q3: Proof Batching Strategy
**Question**: Batch by time (every 10s) or by count (every 100 actions)?
**Trade-off**: Time = consistent latency, count = consistent proof size
**Status**: Requires production data to decide

### Q4: State Pruning
**Question**: When can we safely discard old state/deltas?
**Options**: After checkpoint + proof submission, or keep for auditing?
**Status**: Depends on audit/compliance requirements

---

**End of Document**
