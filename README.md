# Dungeon

Dungeon is a deterministic 2D dungeon crawler built to be verifiable. The goal is simple: every action a player or NPC takes can be proven correct with succinct zero-knowledge proofs, keeping cheating at bay while sparing the blockchain from heavy computation. This repo hosts the full stack—clients, runtime, game rules, and early ZK plumbing—so the project can evolve into a flexible, moddable RPG playground.

> **⚠️ Early-stage prototype:** expect rapid iteration, missing features, and breaking changes. We’re sharing the core architecture early so contributors can help shape the design.

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
  client/
    core/              # shared UX glue: config, messages, view models, oracle factories
    frontend/cli/      # async terminal application, event loop, action provider
  game/
    core/              # deterministic rules engine, domain models, validation schema
    content/           # static content and fixtures exposed through oracle adapters
  runtime/             # public API (RuntimeHandle), orchestrator, workers, oracles, repositories
  zk/                  # proving utilities reused by prover worker and off-chain services

docs/                  # architecture, research notes, design decisions
```

To see how these pieces interact end-to-end, read the workspace and runtime diagrams in [`docs/architecture.md`](docs/architecture.md).

## Prerequisites

- Rust toolchain (1.85+ recommended). Install via [rustup](https://rustup.rs/) if you have not already.
- `cargo` (bundled with the Rust toolchain).

Some crates use async runtimes (`tokio`) and expect a POSIX-like environment. All commands below assume you are in the repository root.

## Common Commands

| Task | Command |
|------|---------|
| Format code | `cargo fmt` |
| Lint with Clippy | `cargo clippy --workspace --all-targets --all-features` |
| Run tests | `cargo test --workspace` |
| Build everything | `cargo build --workspace` |
| Launch CLI client | `cargo run -p client-frontend-cli` |
| Run runtime-only tests | `cargo test -p runtime` |

> Tip: run formatting and linting before pushing changes so CI passes on the first try.

## Contributing

1. Fork the repository and create a feature branch.
2. Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features`, and `cargo test --workspace` before opening a pull request.
3. Document architectural changes in [`docs/architecture.md`](docs/architecture.md) or `docs/research.md` as appropriate.
4. Use Conventional Commits (e.g., `feat: add npc action provider`) for commit messages.

Please open issues for design discussions or to share research notes so the documentation stays current.

## Additional Resources

- [`docs/architecture.md`](docs/architecture.md) – high-level diagrams and subsystem overviews.
- [`docs/research.md`](docs/research.md) – exploratory notes and design investigations (create/update as needed).
- [`docs/runtime.md`](docs/runtime.md) – detailed runtime design (expected future update).

Happy crawling!
