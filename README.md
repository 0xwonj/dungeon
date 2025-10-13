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
- [`CLAUDE.md`](CLAUDE.md), [`AGENTS.md`](AGENTS.md) – Development guidance for AI-assisted coding

Happy crawling!
