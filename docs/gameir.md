# Verifiable Game Engine: A Unified Rust→IR Framework for zk‑Executable Games

> **Status:** Draft
>
> **Scope:** Technical specification of a unified Rust→IR framework for zk-executable games, detailing the GameIR design, compilation pipeline, and proving integration.

---

## 1. Motivation

Modern game logic is a tapestry of **state transitions** driven by player and system **actions**. Turning such logic into something **externally verifiable** is hard. If we insist that every player action be validated with a **zero‑knowledge proof (ZK)**, we quickly face the classic pain point: rewriting gameplay rules as circuits. That rewrite makes maintenance brittle and multiplies development cost.

This document proposes a practical way out: a **unified compilation framework** where the **same Rust gameplay code** can both **execute at runtime** and **emit an intermediate representation (IR)** tailored for proof generation. In short: *one codebase, two products*—a running game and a proof trace.

## 2. Core Idea

We introduce a domain‑specific **GameIR** and require all gameplay code to go through a **Rust API layer**. The API supports two modes:

* **Execution Mode** — mutates real game state in the engine.
* **IR Mode** — runs the same code path but logs **IR nodes** instead of mutating state.

By writing game rules once against this API, developers get:

1. a deterministic runtime, and
2. an **IR trace** from which a ZK proof of *“this action was executed legitimately”* can be produced.

## 3. System Architecture

```
       ┌──────────────┐
       │   Game Code  │
       │  (Rust API)  │
       └──────┬───────┘
              │
     ┌────────┴─────────┐
     │     GameApi<T>   │
     │  ─────────────── │
     │  load, store,    │
     │  assert, lookup  │
     │  rng, commit ... │
     └────────┬─────────┘
              │
  ┌───────────┴─────────────┐
  │                         │
┌─┴───────────┐     ┌────────────┴─────┐
│ ExecBackend │     │   IRBackend      │
│ (Runtime)   │     │ (IR Logging)     │
└─────────────┘     └──────────────────┘
        │                     │
        │                     ▼
        │              ┌─────────────┐
        │              │  GameIR     │
        │              │  (SSA form) │
        │              └─────┬───────┘
        │                    │
        ▼                    ▼
   [Live Gameplay]      [ZKIR Lowering]
                         │
                 ┌───────────────┐
                 │ Halo2/Plonky3 │
                 │   or STARK    │
                 └───────────────┘
```

## 4. GameIR: Unified Intermediate Representation

**GameIR** abstracts Rust execution into a compact, ZK‑friendly IR that normalizes common game‑side effects into **field arithmetic, range checks, hashing, and Merkle verification**.

**Core node set (examples):**

| Node Type                    | Description                                    |
| ---------------------------- | ---------------------------------------------- |
| `Load(tree, key)`            | Read from a Merkle‑backed store                |
| `Store(tree, key, val)`      | Update a Merkle leaf                           |
| `Lookup(table, key)`         | Table lookup (e.g., tile costs, skill effects) |
| `AssertEq(a, b)`             | Equality constraint                            |
| `AssertRange(v, bits)`       | Range constraint (bit‑width)                   |
| `Poseidon(inputs)`           | Hash node (circuit‑friendly)                   |
| `CommitNextRoot(prev, next)` | Root update/verification                       |
| `Rand(seed)`                 | Deterministic RNG abstraction                  |
| `Select(cond, a, b)`         | Predication (branch removal)                   |

Because **every action** is implemented as `apply(api)` with explicit API calls, the IR captures **all side effects** and pre/post‑conditions.

## 5. Unified Rust API Design

### Example: `MoveAction`

```rust
impl Action for MoveAction {
    fn apply<B: Backend>(&self, api: &mut GameApi<B>) -> Result<(), &'static str> {
        let pos = api.load_pos(self.entity_id);
        let next = Pos { x: pos.x + self.dx, y: pos.y + self.dy };

        api.assert((self.dx.abs() + self.dy.abs()) == 1, "Manhattan step = 1");
        api.assert(api.is_walkable(next), "Tile blocked");

        api.store_pos(self.entity_id, next);
        api.commit();
        Ok(())
    }
}
```

* **Execution Mode:** mutates the live world state.
* **IR Mode:** records the API effects as `IRNode`s.

**Outputs from one code path**

1. Updated `World`/entities (runtime).
2. A **proof‑ready GameIR trace** that fully records the transition rules.

## 6. Compilation Pipeline

1. **GameIR Emission**
   Trace Rust API calls → build an IR node list; canonicalize into **SSA** (`pos1`, `next1`, `cond1`, …).
2. **Static Analysis Passes**

   * **Effect analysis:** automatically extract minimal Merkle witnesses.
   * **Branch normalization:** convert control flow to `Select` (predication).
   * **Range/bit‑width inference** → insert `AssertRange` checks.
   * **Lookup synthesis:** compress table‑driven costs into efficient circuits.
3. **ZKIR Lowering**
   Lower to a backend constraint builder (Halo2 / Plonky3 / STARK). Provide primitives for Poseidon, lookups, range checks, commits.
4. **Proving & Verification**
   Produce per‑action proofs; compose via **recursion** to compress per‑turn or per‑round; verify state roots on‑/off‑chain.

## 7. Research Contributions

| Area                     | Contribution                                                                              |
| ------------------------ | ----------------------------------------------------------------------------------------- |
| **Compiler Design**      | Domain‑specific compiler that automatically converts game transitions into ZK‑friendly IR |
| **Language/Type System** | Symbolic typing to prevent misuse during IR emission (`Concrete<T>` vs `Symbolic<T>`)     |
| **Efficient Proving**    | Footprint analysis to minimize Merkle witnesses and constraint count                      |
| **Back‑end Agnosticism** | Lowering layers targeting Halo2, Plonky3, and STARK systems                               |
| **DX/Tooling**           | Single Rust API for both execution and proving with consistent developer ergonomics       |

## 8. Experimental Plan

| Item                            | Goal                                                                 |
| ------------------------------- | -------------------------------------------------------------------- |
| **Single‑Action Proof Perf**    | Measure proving time/constraint count for `Move`, `Attack`, `Pickup` |
| **N‑Turn Recursion**            | Compare size/speed when aggregating per‑turn proofs                  |
| **Merkle Witness Optimization** | IR‑driven effect analysis vs. manual witness construction            |
| **Backend Comparison**          | Halo2 vs. Plonky3 vs. Winterfell (speed, memory, code complexity)    |
| **Debugging UX**                | Evaluate IR‑trace‑based replay/verification tooling                  |

## 9. Broader Impact

* **Path to a ZK‑Game DSL**
  Once GameIR stabilizes, a higher‑level macro/DSL (e.g., `zk_action!`) can express gameplay rules declaratively—*write a game, get proofs for free*.

* **Relation to zkVMs**
  Compared to general‑purpose zkVMs (RISC Zero, SP1), this approach exploits **domain structure** (short loops, constrained data models) for better constants while remaining composable with zkVMs when needed.

* **Security/Systems Extensions**
  Enables proof‑carrying gameplay, DoS‑aware protocol design, and verifiable AI agents interacting with the world.

## 10. Conclusion

This framework is a pragmatic step toward **“game logic == proof logic.”** By funnelling gameplay through a **unified Rust API** that can either **execute** or **emit IR**, we preserve familiar workflows while gaining a **ZK‑ready proving pipeline**.

The endgame is clear: treat the entire game as a **verifiable program**, and use compiler technology to make that both tractable and pleasant for developers.

## Appendix A — Implementation Notes (WIP)

* **Determinism:** Treat RNG as an explicit API and seed source. Avoid hidden entropy.
* **State Storage:** Prefer Sparse‑Merkle variants; abstract via `tree_id` to support multiple stores.
* **Tables & Lookups:** Keep canonical tables versioned; IR captures table IDs, not raw values.
* **Testing:** Golden‑file IR traces for actions; property tests ensure execution/IR parity.
* **Tooling:** IR visualizer, witness size estimator, and backend adapters as separate crates.
