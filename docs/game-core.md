# Dungeon — `game-core` Specification

*Mode: Light ZK + Single Server · Chain: EVM · Principle: **Functional core, imperative shell***

This document defines the **`game-core`** crate: its purpose, boundaries, public API, and implementation patterns. It also clarifies **what belongs to `runtime`** vs **what must remain in `game-core`**, so there’s no ambiguity when you start coding.

---

## 1) Purpose & Non-Goals

### Purpose (what `game-core` **is**)

* A **pure, deterministic domain library**: given an initial `State`, an `Env` (read-only facts), and an `Action`, it computes the **next `State`**.
* Produces a **witness delta** describing which facts were read and which invariants were enforced (for ZK proof binding).
* Defines the **canonical field order** of `State` for commitments (but does **not** hash).

### Non-Goals (what `game-core` **is not**)

* No I/O, no DB, no network, no clocks/timers, no randomness.
* No Merkle verification, no signatures, no proof generation, no chain logic.
* No rendering, input handling, or UI concerns.

> If a concern touches storage, crypto, networking, secrets, or scheduling, it belongs to **`runtime`**, not `game-core`.

---

## 2) Boundary with `runtime` (sharp contract)

| Concern           | `game-core`                | `runtime`                                                     |
| ----------------- | -------------------------- | ------------------------------------------------------------- |
| State & rules     | ✅ owns                     | calls `game-core`                                             |
| Environment facts | **reads via oracles**      | **implements** oracles from verified caches                   |
| Map data          | terrain grid + spawn list  | persists map roots, serves oracle                             |
| Witness           | returns **witness deltas** | assembles **witness transcript** (tiles/resources/NPC orders) |
| Commitments       | **specifies** field order  | computes **hash**, manages nullifiers                         |
| Proofs            | ❌                          | ✅ (calls `proofs`)                                            |
| Storage & network | ❌                          | ✅ (repos, RPC, chain)                                         |
| Secrets           | ❌                          | ✅                                                             |
| NPC orders        | ❌ (inputs only)            | ✅ (fetch/verify/signature bind)                               |

**Ambiguities resolved here:**

* **Randomness:** not allowed in `game-core`. If a rule needs entropy, the **value is an explicit input** (e.g., carried inside `Action` or supplied as a deterministic “ticket”).
* **Map/skill tables:** read via **oracles**. Map oracle exposes only immutable terrain and initial entity placement so the static commitment stays small; `game-core` never verifies proofs or fetches data.
* **Hashing:** `game-core` defines **order only**; `runtime`/`proofs` do the hashing.

## 3) Canonical actions & command layer

* `Action`/`ActionKind` remain the **authoritative closed set** used for serialization, proofs, and reducers. Adding gameplay requires adding or updating these variants so circuits stay bounded.
* Higher-level systems can remain ergonomic by implementing `ActionCommand` and materialising them with `Action::from_command(actor, command, ctx)`. The trait lets builders consult state/env via `CommandContext` before emitting the canonical action.
* Commands always receive the aggregated `GameEnv` (map/items/tables). Even when a command does not need every oracle, it still uses the same entry point so the dependency surface stays uniform.
* `GameEnv` can leave individual oracles empty (`Env::new(None, Some(items), None)`), letting call-sites pay only for the data they need while keeping the interface consistent.
* This keeps zk wiring and replay deterministic while still allowing plugins or data-driven content to plug their own validation/build steps upstream of the core reducer.

```rust
use game_core::{
    Action, ActionCommand, CommandContext, EntityId, GameEnv, GameState, MoveAction,
};

fn queue_move(actor: EntityId, cmd: MoveAction, state: &GameState, env: GameEnv<'_>) -> Action {
    let ctx = CommandContext::new(state, env);
    Action::from_command(actor, cmd, ctx).expect("MoveAction is infallible")
}

// For helpers that only need item data you can supply partial envs:
// let env = Env::new(None, Some(items_oracle), None).into_game_env();
```

## 4) Explicit state machine reducer

* `game-core` is modelled as a **finite state machine**: the core API is `step(prev_state, env, action) -> Result<next_state, Error>` (re-exported from `reducer::step`). All state evolution flows through this reducer so determinism, logging, and zk wiring stay aligned.
* Each action variant implements `ActionTransition` which exposes `pre_validate`, `apply`, and `post_validate`. The reducer always calls these hooks **in order**, mirroring the constraint checks the proof system enforces around the mutation.
* `pre_validate` inspects the incoming snapshot and oracle data before any mutation; `apply` performs the actual writes; `post_validate` reasserts global invariants (HP ≥ 0, inventory capacity, etc.) on the updated state.
* Reducer helpers record which state/env fields were read or written so witness builders and zk circuits consume the same access pattern. Every mutation therefore has an audit trail (`actor` id + action + witness delta) suitable for replay or proofs.
* System-driven effects (ticks, scripted events) are just actions authored by reserved actors (e.g., `EntityId::SYSTEM`), keeping the machine closed under the same transition interface.
