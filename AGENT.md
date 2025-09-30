# Repository Guidelines

## Project Structure & Module Organization
The root `Cargo.toml` only defines the workspace; executable entrypoints live under `crates/client/cli` and `crates/client/ui`. Domain-specific logic lives in `crates/`: `game-core` for gameplay rules, `types` for shared data structures, `proofs` for zk-friendly routines, `server` for backend services, and `client/` for UI, CLI, and runtime front-ends (see `crates/client/README.md`). Keep tests alongside their crates, and document larger decisions in `architecture.md` or exploratory notes in `research.md`. Build artifacts go to `target/`; never commit that directory.

## Build, Test, and Development Commands
Run `cargo build --workspace` for a full debug build of every crate. `cargo run -p client-cli` exercises the headless client entry point, while `cargo test --workspace` executes unit and integration tests across the workspace. Format and lint before review with `cargo fmt` and `cargo clippy --workspace --all-targets`. Use `cargo doc --no-deps --open` to inspect API docs when shaping new modules.

## Coding Style & Naming Conventions
Use Rust 2024 defaults with 4-space indentation and trailing commas where possible. Functions, modules, and files stay in `snake_case`; exported structs, enums, and traits use `PascalCase`; constants remain `SCREAMING_SNAKE_CASE`. Prefer explicit module boundaries with `mod.rs` or re-export patterns in `lib.rs`. Let `rustfmt` and `clippy` enforce formatting and idioms; address warnings rather than suppressing them.

## Testing Guidelines
Colocate fast unit tests in `#[cfg(test)]` modules next to the code. For cross-crate behavior, add integration suites under `tests/` within the relevant crate (e.g., `crates/game-core/tests`). Name tests after observable behavior such as `handles_empty_party()` to clarify intent. Always run `cargo test --workspace` before pushing and capture regression scenarios from bugs as new cases.

## Commit & Pull Request Guidelines
Adopt Conventional Commits (e.g., `feat: wire runtime event bus`) until a formal standard is published. Keep commits scoped to a single concern and include doc updates when behavior changes. Pull requests should link to design notes or issues, summarize user-facing impact, list verification steps (`cargo fmt`, `cargo clippy`, `cargo test`), and attach logs or screenshots when CLI output changes.

## Security & Configuration Tips
Treat secrets (RPC keys, proving keys) as runtime-only concerns per `crates/client/README.md`; load them via environment variables or local config files excluded from VCS. When introducing new config, document defaults and safe overrides, and avoid writing secrets to logs or artifacts.
