# Dungeon — `game-core` Specification

*Mode: Light ZK + Single Server · Chain: EVM · Principle: **Functional core, imperative shell***

This document defines the **`game-core`** crate: its purpose, boundaries, public API, and implementation patterns. It also clarifies **what belongs to `agent`** vs **what must remain in `game-core`**, so there’s no ambiguity when you start coding.

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

> If a concern touches storage, crypto, networking, secrets, or scheduling, it belongs to **`agent`**, not `game-core`.

---

## 2) Boundary with `agent` (sharp contract)

| Concern           | `game-core`                | `agent`                                                       |
| ----------------- | -------------------------- | ------------------------------------------------------------- |
| State & rules     | ✅ owns                     | calls `game-core`                                             |
| Environment facts | **reads via oracles**      | **implements** oracles from verified caches                   |
| Witness           | returns **witness deltas** | assembles **witness transcript** (tiles/resources/NPC orders) |
| Commitments       | **specifies** field order  | computes **hash**, manages nullifiers                         |
| Proofs            | ❌                          | ✅ (calls `proofs`)                                            |
| Storage & network | ❌                          | ✅ (repos, RPC, chain)                                         |
| Secrets           | ❌                          | ✅                                                             |
| NPC orders        | ❌ (inputs only)            | ✅ (fetch/verify/signature bind)                               |

**Ambiguities resolved here:**

* **Randomness:** not allowed in `game-core`. If a rule needs entropy, the **value is an explicit input** (e.g., carried inside `Action` or supplied as a deterministic “ticket”).
* **Map/skill tables:** read via **oracles**; `game-core` never verifies proofs or fetches data.
* **Hashing:** `game-core` defines **order only**; `agent`/`proofs` do the hashing.
