# AGENTS.md

Guidelines for autonomous or semi-autonomous assistants contributing to the Dungeon project.

## 1. Mission Context

- **Project scope**: Dungeon is an early-stage 2D dungeon crawler that models gameplay as a deterministic finite state machine. Every action is intended to be proven with zero-knowledge (ZK) circuits, minimizing blockchain interaction.
- **Architecture**: The workspace is a multi-crate Rust project (Rust 2024 edition). Core crates include `runtime`, `client`, `game-core`, `game-content`, and `zk`. Architectural diagrams live in `docs/architecture.md`.
- **Primary goals**:
  - Maintain determinism across the runtime and `game-core` so proofs remain reproducible.
  - Implement action-validity proofs (encode pre/post validation checks) rather than zkVM-style full-program proofs.
  - Preserve modularity (providers, oracles, repositories) to keep the game extensible.
  - Deliver a fun, systemic RPG experience while enabling verifiable play.

## 2. Operating Rules for AI Agents

1. **Stay deterministic**
   - Avoid introducing hidden sources of randomness. Any randomness must be injected via providers and documented.
   - Do not rely on wall-clock time or global mutable state when writing logic or tests.

2. **Respect module boundaries**
   - `runtime` exposes the public API (`RuntimeHandle`, `GameEvent`, `RuntimeConfig`) and orchestrates workers. Keep internal wiring (workers, repositories, oracles) encapsulated.
   - `game-core` should remain deterministic and stateless; its functions operate on provided state snapshots.
   - `client` crates consume the runtime API. Avoid leaking runtime internals into front-end crates.

3. **ZK Awareness**
   - When adding gameplay logic, consider how it will be expressed in ZK circuits (especially pre/post validation). Favor simple, auditable functions.
   - Emit the data that future circuits will need (checkpoints, witnesses) and document the assumptions.

4. **Rust 2024 discipline**
   - Use `rustfmt` defaults (4 spaces, trailing commas where appropriate).
   - Resolve `clippy` warnings instead of silencing them unless a justification is documented.
   - Prefer `async` patterns consistent with existing `tokio` usage.

5. **Documentation expectations**
   - Significant architectural changes must be reflected in `docs/architecture.md` or `docs/research.md`.
   - README edits should keep the current tone: concise overview, philosophy, ZK strategy, and practical commands.

6. **Testing & verification**
   - Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features`, and `cargo test --workspace` before submitting changes.
   - When adding new features, consider unit tests inside the relevant crate and integration tests if cross-crate behavior is affected.

7. **Commit & PR style**
   - Use Conventional Commits (e.g., `feat: add npc activation proof witness`).
   - Keep commits focused; include tests or documentation updates with the functional change.
   - Summaries should mention ZK implications when relevant (e.g., new witnesses, validation logic).

8. **Code review mindset**
   - Highlight anything that could break determinism, make proofs harder, or expand blockchain interaction.
   - Verify that new modules maintain the trait-driven interfaces (action providers, oracles, repositories).

9. **Safety & Security**
   - Do not store secrets or credentials in the repo. Any runtime secrets must be loaded via environment or ignored configuration files.
   - Be cautious with external dependencies; ensure licenses are compatible and crates are actively maintained.

10. **When uncertain**
    - Add notes to `docs/research.md` capturing open questions or experimental findings.
    - Prefer leaving TODOs with context (`// TODO: encode post-validate check in circuit`) rather than silent assumptions.

## 3. Quick Reference Checklist

Before finishing a task:

- [ ] Code formatted with `cargo fmt`.
- [ ] Lint clean: `cargo clippy --workspace --all-targets --all-features`.
- [ ] Tests pass: `cargo test --workspace` (and more targeted suites if modified).
- [ ] Documentation updated (`README.md`, `docs/*.md`) when behavior, architecture, or philosophy shifts.
- [ ] Architectural constraints respected (deterministic FSM, modular providers, action-validity proof plan).
- [ ] Conventional commit message prepared.

## 4. Additional Resources

- `README.md` – project overview, philosophy, ZK strategy, and contributor workflow.
- `docs/architecture.md` – workspace dependency graph, runtime control flow, and worker responsibilities.
- `docs/research.md` – log experiments, design explorations, and outstanding decisions.

Stay aligned with the project’s philosophy: build a fun dungeon crawler that delivers verifiable gameplay with minimal blockchain friction.
