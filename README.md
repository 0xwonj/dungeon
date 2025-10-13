# Dungeon

> **⚠️ Early-stage prototype:** expect rapid iteration, missing features, and breaking changes. We’re sharing the core architecture early so contributors can help shape the design.

Dungeon is a **verifiable roguelike RPG** — a deterministic world where every turn can be proven valid, yet not all truths are visible.

Built on zero-knowledge proofs (ZKPs), Dungeon ensures that every action, roll, and AI move followed the rules **without revealing** hidden information.  
The result is a game that’s both **honest and mysterious** — fair because it’s provable, alive because it’s systemic.

At its core, Dungeon explores how **games can become transparent systems of truth** rather than opaque entertainment products.  
Each world is procedural, deterministic, and shaped by interacting systems rather than scripts.  
Your choices — lighting a torch, sparing an enemy, sealing a door — ripple through the rule system to form emergent stories that feel inevitable, not authored.

> *Fairness without authority. Secrecy without deceit.*

Learn more about the design vision and philosophy in [**philosophy.md**](./docs/philosophy.md)

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
