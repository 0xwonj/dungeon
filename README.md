# Dungeon

> **⚠️ Early-stage prototype:** expect rapid iteration, missing features, and breaking changes. We’re sharing the core architecture early so contributors can help shape the design.

Dungeon is a **classic roguelike** reimagined for the age of cryptographic truth.  
Every tile, turn, and outcome is governed by deterministic rules and safeguarded by zero-knowledge proofs (ZKPs). The result is a **verifiable game** — one where everything that happens can be proven valid, yet parts of the world can remain secret.

At its heart, Dungeon is about **emergent stories born from deep systems**. Each world is procedurally generated, shaped by rules rather than scripts, and every encounter has weight because it follows from logic, not design shortcuts. The game values consequence over spectacle — a place where player choices ripple through deterministic mechanics to create meaning, tension, and sometimes tragedy. The world doesn’t tell a story to you; it **becomes one through your actions**.

Zero-knowledge proofs protect both fairness and mystery. They allow anyone to verify that an action, loot roll, or AI move followed the game’s rules, without revealing private information like hidden maps or enemy intentions. In other words, **the game proves its own honesty**, even when not all truths are visible.

### Verifiable Game

Traditional online games rely on centralized servers and unverifiable logic—players must trust both that the **operator plays fair** and that **other players aren’t cheating**. Dungeon removes that trust assumption entirely.  
Every state transition is deterministic, verifiable, and ultimately provable, making both server manipulation and client exploits cryptographically impossible. By combining deterministic simulation with zero-knowledge proofs, Dungeon explores how **games can become transparent systems of truth** rather than opaque entertainment products.

This is important — in Web3 games, fairness and trustlessness aren’t just design goals but **security guarantees**. Players own their assets and outcomes, so the rules themselves must be provable. Dungeon treats verifiability as a first-class principle: the game world operates transparently, **bound by math rather than authority**.

## Philosophy

### Determinism first
The runtime, repositories, and workers are structured so the same sequence of actions always yields the same state. Determinism makes the game debuggable, unlocks reproducible tests, and is the foundation for producing consistent zero-knowledge witnesses. We avoid hidden side effects or implicit randomness; all entropy comes from injectable providers.

### Prove validity, not everything
We view the game as a finite state machine: actions are the only way to transition state, and each action passes through `pre-validate`, `execute`, and `post-validate` phases. Instead of re-running the whole engine inside a zkVM, we plan to prove that each action satisfied the validation phases for the prior state. This keeps proofs succinct and affordable while still preventing cheating.

### Modular by default
Every boundary—input providers, oracle adapters, persistence, workers—speaks through traits or clear module contracts. Developers can slot in a custom AI policy, swap in new content packs, or back the runtime with different storage without rewriting the core. This modularity is what lets the project scale from a terminal prototype to richer clients and community mods.

### Players, then proofs
We are building a fun, expressive 2D dungeon crawl first. The system should encourage tactical depth, emergent NPC behaviors, and designer creativity. Zero-knowledge enforcement lives alongside the gameplay loop instead of dictating it; the proving story supports the RPG, not the other way around.

### Minimal blockchain footprint
Proof artifacts and occasional state commitments are the only data meant for chains. We push heavy computation off-chain and keep gameplay loops snappy. The design lets players cooperate or compete without burdening them with constant transactions, while still enabling on-chain verification when needed.

## Zero-Knowledge Strategy

There are multiple ways to bring ZK proofs into games. One approach is to prove the entire runtime (AI, physics, rules) inside a zkVM—easy to reason about but prohibitively expensive and slow for richly interactive games. Dungeon instead proves **action validity**:

1. **Finite State Machine** – the game is treated as an FSM where only actions transition state. `game-core` remains deterministic and stateless, accepting state snapshots and action descriptions.
2. **Validation Phases** – each action consists of `pre-validate`, `execute`, and `post-validate` steps. The plan is to encode the validation phases into ZK circuits so we can prove “this action was legal for the prior state” without running the full engine in-circuit.
3. **Witness Production** – during execution the runtime emits checkpoints and witnesses for the prover worker. Proofs will assert action validity and can be posted on-chain or stored off-chain for audits.
4. **Minimal Blockchain Interaction** – by proving validity off-chain, only proof artifacts and occasional state commitments need to be persisted to a chain, keeping costs low and throughput high while still preventing cheating.

## Repository Layout

```
crates/
├── client/
│   ├── core/              # Shared UX glue: config, messages, view models, oracle factories
│   └── frontend/
│       ├── core/          # Frontend abstraction layer: FrontendApp trait, message routing
│       └── cli/           # Async terminal application with cursor system and examine UI
├── game/
│   ├── core/              # Pure deterministic state machine (actions, engine, validation)
│   └── content/           # Static content and fixtures (maps, items, NPCs, loot tables)
├── runtime/               # Public API (RuntimeHandle), orchestrator, workers, oracles, repositories
└── zk/                    # Proving utilities (planned for prover worker and off-chain services)

docs/                      # Architecture, research notes, design decisions
```

Key implementation features:
- **Action System**: Comprehensive action validation with pre-validate, execute, and post-validate phases
- **Turn Management**: Deterministic turn scheduling with entity activation and cooldown tracking
- **CLI Interface**: Terminal UI with examine mode, cursor system, and targeting for tactical gameplay
- **Event Broadcasting**: Runtime emits `GameEvent` notifications for all state transitions
- **Worker Architecture**: `SimulationWorker` manages canonical state; `ProverWorker` planned for ZK proofs

To see how these pieces interact end-to-end, read the workspace and runtime diagrams in [`docs/architecture.md`](docs/architecture.md).

## Prerequisites

- Rust toolchain (1.85+ recommended). Install via [rustup](https://rustup.rs/) if you have not already.
- `cargo` (bundled with the Rust toolchain).

Some crates use async runtimes (`tokio`) and expect a POSIX-like environment. All commands below assume you are in the repository root.

## Common Commands

| Task | Command |
|------|---------|
| Build everything | `cargo build --workspace` |
| Launch CLI client | `cargo run -p client-frontend-cli` |
| Run all tests | `cargo test --workspace` |
| Run specific crate tests | `cargo test -p runtime` or `cargo test -p game-core` |
| Run single test | `cargo test --workspace <test_name>` |
| Format code | `cargo fmt` |
| Lint with Clippy | `cargo clippy --workspace --all-targets --all-features` |
| Generate API docs | `cargo doc --no-deps --open` |

## Contributing

We welcome contributions!
Please read the full [Contributing Guidelines](.github/CONTRIBUTING.md) before opening a Pull Request.

---

## Additional Resources

- [`docs/architecture.md`](docs/architecture.md) – High-level diagrams and subsystem overviews
- [`docs/status.md`](docs/status.md) – Current implementation status and roadmap (updated frequently)
- [`docs/research.md`](docs/research.md) – Exploratory notes and design investigations
- [`.github/CONTRIBUTING.md`](.github/CONTRIBUTING.md) – Contributing guidelines and code standards
- [`CLAUDE.md`](CLAUDE.md) – Development guidance for AI-assisted coding

Happy crawling!
