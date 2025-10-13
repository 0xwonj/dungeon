# Research Notes

> **Status:** Internal notes  
>
> **Scope:** Design decisions and reasoning logs for Dungeon’s core architecture — documents technical trade-offs, removed patterns, and rationale behind engine and proof model evolution.

---

## StateReducer Pattern Removal (2025-10-10)

### Context

Initially designed a `StateReducer` pattern to provide a structured API for state mutations. The reducer would collect granular state changes and build a `StateDelta` from those recorded mutations.

### Original Design

```rust
// Proposed reducer-based approach
let mut reducer = StateReducer::new(&mut state);
reducer.move_entity(entity, new_pos);
reducer.update_hp(entity, delta);
let state_delta = reducer.finish();
```

**Rationale:**
- Capture every state change explicitly
- Build `StateDelta` incrementally from recorded operations
- Provide a clean API boundary for state mutations
- Potentially easier to prove all logic in ZK (every operation is explicit)

### Design Changes

**1. Direct state mutation instead of reducer API:**
```rust
// Current approach
state.entities.player.position = new_pos;
state.entities.player.hp += delta;
```

**2. StateDelta computation via snapshot comparison:**
```rust
let before = state.clone();
// ... mutate state directly ...
let delta = StateDelta::from_states(action, &before, &after);
```

**Reasons for change:**

1. **Simpler mental model** - Direct mutation is straightforward. No need to learn a reducer API.

2. **Reduced boilerplate** - No need to wrap every state change in a reducer method call.

3. **Flexible delta computation** - Comparing snapshots can detect any change, regardless of how it was made. More robust than relying on reducer methods to correctly record everything.

4. **ZK proof strategy evolved** - Originally planned to prove all game logic in ZK circuit. This would require explicit operations (reducers help here).

### ZK Proof Strategy Change

**Original plan:** Prove entire game logic execution in circuit
- Every operation needs to be circuit-friendly
- Reducer pattern makes operations explicit and easier to translate to constraints
- High circuit complexity, large proof size

**Current plan:** Prove only validation, not full execution
- Prove `pre_validate()` and `post_validate()` passed
- Prove action was valid and state transition was valid
- Don't prove the exact execution path (move logic, combat calculations, etc.)
- Much smaller circuit, faster proving

**Implication:** Reducer is unnecessary for current ZK approach. We only need to prove:
```
assert(pre_validate(state_before, action) == Ok);
assert(state_after == apply(state_before, action));
assert(post_validate(state_after, action) == Ok);
```

The exact implementation of `apply()` doesn't need to be proven step-by-step.

### When Reducer Would Be Useful Again

**If we need to prove full execution:**
- Regulatory requirements to prove exact game logic
- Player disputes requiring proof of specific calculations
- Full deterministic replay needs to be verifiable on-chain

**If state becomes very large:**
- Cloning entire state becomes expensive
- Incremental delta tracking via reducer is more efficient
- But this is premature optimization for now

### Conclusion

Direct state mutation + snapshot-based delta computation is simpler and sufficient for current needs. The reducer pattern adds unnecessary abstraction when our ZK proof only validates transitions, not full execution logic.

**Action:** Keep direct mutation approach. Reducer can be reintroduced later if proof strategy changes.

**Lesson:** API design should follow from actual requirements, not anticipated complexity. Start simple.

---

## Transaction Guard for State Mutations (2025-10-11)

### Context

Considered adding a `TransactionGuard` to `GameEngine::execute()` to provide all-or-nothing semantics for state mutations. The concern was that if execution fails partway through (e.g., during hook application), the state might be left in a partially modified, inconsistent state.

### Current Architecture

```rust
GameEngine::execute() {
    pre_validate()  // Validate before changes
    � apply()       // Modify state
    � post_validate()  // Verify invariants after changes
    � hooks         // Post-execution side effects
}
```

**Design principles:**
- `game/core` is pure, deterministic, no I/O
- If `pre_validate()` passes, `apply()` should be infallible
- State transitions are reproducible and testable

### Proposed Solution

Implemented `TransactionGuard` that:
1. Captures a snapshot before execution
2. Explicitly rolls back on error
3. Reuses the clone already done for delta computation (zero additional cost)

**Implementation:**
```rust
let guard = TransactionGuard::new(self.state);
let before = guard.snapshot();

let result = dispatch_transition!(...);
if let Err(e) = result {
    guard.rollback(self.state);
    return Err(e);
}
// Success path...
```

### Decision: NOT to use TransactionGuard

**Reasons against:**

1. **Validation is sufficient** - The pre/post validation pipeline already guarantees correctness. If validation passes, apply should not fail.

2. **Hides bugs instead of fixing them** - A rollback guard masks implementation bugs that should be caught by tests. If `apply()` can fail after `pre_validate()` succeeds, that's a bug in the transition logic that needs fixing, not hiding.

3. **Adds complexity** - Extra rollback logic, conditional branches, and mental overhead for readers. The code becomes less clear about when failures can occur.

4. **Conflicts with deterministic design** - `game/core` is designed to be a pure, deterministic state machine. Runtime guards suggest we don't trust our own validation logic.

5. **Performance overhead** - While the clone is free (already done for delta), the error checking and rollback branching adds overhead on every execution.

6. **Testing over guards** - Bugs should be caught by comprehensive unit tests, not papered over by runtime guards. Guards make it harder to discover actual bugs.

**When it WOULD be justified:**

- Hooks become a plugin system accepting untrusted third-party code
- Production environment requires graceful degradation over correctness
- State transitions involve external I/O or non-deterministic operations

**Current context:** None of these apply. `game/core` is controlled, deterministic, and fully tested.

### Conclusion

Keep the code simple and rely on validation + testing. If we can't trust our validation logic, we should fix the validation, not add runtime guards.

**Action:** Remove `TransactionGuard` implementation and keep the original straightforward execution path.

**Lesson:** Defensive programming has its place, but in a pure functional core with strong validation, it's often unnecessary complexity.
