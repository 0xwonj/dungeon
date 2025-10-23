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

## Prerequisites

- Rust toolchain (1.85+ recommended). Install via [rustup](https://rustup.rs/) if you have not already.
- `cargo` (bundled with the Rust toolchain).
- [`just`](https://github.com/casey/just) command runner (recommended): `cargo install just`

Some crates use async runtimes (`tokio`) and expect a POSIX-like environment. All commands below assume you are in the repository root.

## Quick Start

**Recommended:** Use `just` for easier multi-backend development:

```bash
# Install just (one-time setup)
cargo install just

# Fast development with stub backend (no real proofs)
just build stub
just run stub
just test stub

# Fast mode (no proof generation, no persistence)
just run-fast stub

# Set default backend via environment
export ZK_BACKEND=stub
just build  # uses stub automatically
just run

# See all available commands
just --list
just help
```

### Available ZK Backends

- `risc0` - RISC0 zkVM (production, real proofs, slow)
- `stub` - Stub prover (instant, no proofs, testing only)
- `sp1` - SP1 zkVM (not implemented yet)
- `arkworks` - Arkworks circuits (not implemented yet)

### Common Just Commands

| Task | Command |
|------|---------|
| Build with backend | `just build [backend]` |
| Run CLI client | `just run [backend]` |
| Run in fast mode | `just run-fast [backend]` (no proofs, no persistence) |
| Run all tests | `just test [backend]` |
| Lint code | `just lint [backend]` |
| Format code | `just fmt` |
| Pre-commit checks | `just pre-commit` |
| Verify all backends | `just check-all` |
| Fast dev loop | `just dev` (format + lint + test stub) |
| Monitor logs | `just tail-logs [session]` |
| Clean data | `just clean-data` |

### Direct Cargo Commands (without Just)

If you prefer not to use `just`, you can use cargo directly:

```bash
# Stub backend (fast development)
cargo build --workspace --no-default-features --features stub
cargo run -p client-cli --no-default-features --features stub
cargo test --workspace --no-default-features --features stub

# RISC0 backend (default)
cargo build --workspace
RISC0_SKIP_BUILD=1 cargo build --workspace  # skip guest builds

# Format and lint
cargo fmt --all
cargo clippy --workspace --all-targets
```

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
