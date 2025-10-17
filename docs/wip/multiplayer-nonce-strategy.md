# Multiplayer Nonce Strategy (Future Design)

> **Status**: Design note for future implementation
> **Current**: Single-player only, using global nonce
> **When**: Implement when adding multiplayer support

## Current State (Single-player)

### Global Nonce Only

```rust
// game-core/src/state/types/turn.rs
pub struct TurnState {
    /// Sequential action counter (0, 1, 2, ...)
    /// Increments with every action execution
    pub nonce: u64,
    // ...
}
```

**Usage**:
- Event logging: `ActionRef { nonce: u64 }`
- Proof tracking: `ProofEntry { nonce: u64 }`
- State versioning: `StateRepository::save(nonce, state)`

**Sufficient for**:
- ✅ Tracking execution order
- ✅ Proof generation sequencing
- ✅ Debugging and logging
- ✅ State checkpoint identification

## Future: Multiplayer with Per-Actor Nonce

### Why Per-Actor Nonce?

In multiplayer environments:

1. **Multiple clients submit actions concurrently**
   ```
   Client A (PLAYER):   Action { Move, nonce=? }
   Client B (NPC AI):   Action { Attack, nonce=? }
   Client C (PLAYER_2): Action { UseItem, nonce=? }
   ```

2. **Each client needs replay protection**
   - Client can't predict global nonce (other actors' actions affect it)
   - Client CAN predict their own action sequence

3. **Blockchain-style ordering**
   - Like Ethereum: each account has independent nonce
   - Server can reorder actions from different actors
   - Cannot reorder actions from same actor

### Proposed Design

#### 1. Dual Nonce System

```rust
pub struct TurnState {
    /// Global action counter (all actions: player + NPC + system)
    /// Purpose: Logging, proof sequencing, state versioning
    pub global_nonce: u64,

    /// Per-actor action counters
    /// Purpose: Replay prevention, per-actor ordering
    pub actor_nonces: BTreeMap<EntityId, u64>,

    pub clock: Tick,
    pub active_actors: BTreeSet<EntityId>,
    pub current_actor: EntityId,
}
```

#### 2. Action with Actor Nonce

```rust
pub enum Action {
    Character {
        actor: EntityId,
        actor_nonce: u64,  // This actor's Nth action
        kind: CharacterActionKind,
    },
    System {
        // No actor_nonce (always server-generated, sequential)
        kind: SystemActionKind,
    },
}
```

#### 3. Validation in GameEngine

```rust
impl GameEngine {
    pub fn execute(&mut self, action: &Action) -> Result<StateDelta> {
        match action {
            Action::Character { actor, actor_nonce, .. } => {
                // Validate per-actor nonce
                let expected = self.state.turn.actor_nonces
                    .get(actor)
                    .copied()
                    .unwrap_or(0) + 1;

                if *actor_nonce != expected {
                    return Err(ExecuteError::InvalidActorNonce {
                        actor: *actor,
                        expected,
                        got: *actor_nonce,
                    });
                }

                // Increment actor nonce after execution
                *self.state.turn.actor_nonces.entry(*actor).or_insert(0) += 1;
            }
            Action::System { .. } => {
                // No actor nonce validation for system actions
            }
        }

        // Always increment global nonce (all action types)
        self.state.turn.global_nonce += 1;

        // ... rest of execution
    }
}
```

#### 4. Client-Side Action Provider

```rust
/// Multiplayer client that tracks its own nonce
pub struct MultiplayerActionProvider {
    my_actor: EntityId,
    my_next_nonce: AtomicU64,  // Client-managed counter
    action_rx: mpsc::Receiver<CharacterActionKind>,
}

#[async_trait]
impl ActionProvider for MultiplayerActionProvider {
    async fn provide_action(&self, entity: EntityId, _state: &GameState) -> Result<Action> {
        assert_eq!(entity, self.my_actor);

        // Client increments its own nonce
        let nonce = self.my_next_nonce.fetch_add(1, Ordering::SeqCst);

        let kind = self.action_rx.recv().await?;

        // Action includes client's nonce
        Ok(Action::character(entity, nonce, kind))
    }
}
```

### Migration Path

#### Phase 1: Current (Single-player)
- Global nonce only
- Action has NO nonce field
- Simple, sufficient for current needs

#### Phase 2: Add Per-Actor Nonce (Multiplayer Prep)
1. Add `actor_nonces: BTreeMap<EntityId, u64>` to `TurnState`
2. Add `actor_nonce: u64` to `Action::Character`
3. Update `GameEngine::execute()` to validate both nonces
4. Update serialization (breaking change for saved states)
5. Update all Action construction sites

#### Phase 3: Multiplayer Features
1. Network protocol for action submission
2. Action queue with conflict resolution
3. Client-side optimistic execution
4. Server authority and validation

### ZK Proof Implications

#### Action Nullifier Generation

```rust
// Current (single-player): global nonce only
nullifier = H(action_commitment || global_nonce)

// Future (multiplayer): include actor_nonce
nullifier = H(actor || actor_nonce || action_commitment)
```

Per-actor nonce provides:
- ✅ Unique nullifier per actor's action sequence
- ✅ Prevents replay across different actors
- ✅ Enables parallel proof generation (proofs for different actors are independent)

#### Circuit Constraints

```rust
// Verify both nonces in circuit
circuit.constrain(action.actor_nonce == prior_state.actor_nonces[action.actor] + 1);
circuit.constrain(after_state.global_nonce == prior_state.global_nonce + 1);
```

## References

- [action-validity.md](../action-validity.md#62-ordering--reorgs): "Actions carry (actor_id, nonce)"
- Ethereum transaction nonce system
- CRDT ordering with causal consistency

## Decision Log

- **2025-01-XX**: Decided to defer per-actor nonce until multiplayer implementation
- **Rationale**: YAGNI - global nonce sufficient for single-player, avoid premature complexity
