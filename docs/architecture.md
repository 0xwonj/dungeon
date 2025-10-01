# ZK Turn‑Based RPG — Developer Specification v1 (Light ZK + Single Server, EVM)

> **Audience**: Engineers starting implementation. **Mode**: Light ZK + Single Server authority. **Chain**: EVM (testnet/mainnet unspecified). **Intent**: High‑level but executable blueprint—no low‑level encodings or circuit equations.

See also: `docs/game_design.md` for the concrete game rules and v1 scope.

---

## 1) Project Scope & Principles

* **Deterministic core**: The same state‑transition rules run on client and prover, free of nondeterminism and floating‑point.
* **ZK‑enforced invariants**: Movement bounds, collision, cooldown/resource conservation, replay protection.
* **Authority‑backed NPC**: A single server issues signed NPC orders; clients prove only essential invariants.
* **Minimal on‑chain**: Store commitments and verify succinct proofs; avoid raw game data on chain.
* **Separation of concerns**: Strict module boundaries; core does not depend on rendering, networking, or storage.

Non‑Goals (for now): Full ZK NPC logic, committee signatures, L2 specifics.

---

## 2) System Architecture (High‑Level)

**Components**

* **Client (Rust, Bevy optional)**: Renders, captures input, runs deterministic core, gathers witnesses, requests NPC orders, packages proof submissions.
* **Prover (Rust; zkVM or Plonkish)**: Produces proofs (optionally over short sequences); optional recursive folding.
* **NPC Server (Authority)**: Computes NPC actions, signs orders, optionally publishes an append‑only audit log and VRF randomness.
* **On‑Chain (EVM)**: Verifier and minimal game contract maintaining player state commitments and nullifier registry.

**Data Contracts (abstract)**

* **MapRoot**: Commitment to the map (tile traits/costs/tags).
* **StateCommit S_t**: Commitment to player state (position, hp/mp, status_root, inv_root, turn_index, player_id, …).
* **Nullifier**: Anti‑replay tag derived from player identity, turn index, and state (exact formula out of scope here).
* **SignatureCommit**: Submission‑level aggregation/commitment of NPC orders included in a proof.

---

## 3) Repository Layout (Rust Workspace)

```
dungeon/
├─ Cargo.toml                      # workspace manifest only
├─ crates/
│  ├─ game-core/                   # deterministic rules engine
│  ├─ types/                       # shared commitments/IDs (no deps)
│  ├─ proofs/                      # proof system facade/backends
│  ├─ server/                      # NPC authority services
│  └─ client/
│     ├─ runtime/                  # lib crate (authoritative runtime)
│     ├─ cli/                      # bin crate (headless)
│     └─ ui/                       # bin crate (Bevy UI)
└─ onchain/
   └─ contracts/                   # verifier + state contracts

```

**Dependency direction**: `client`, `server`, `proofs` → depend on **core** only. `onchain` is isolated.

Workspace entrypoints: `cargo run -p client-cli` and `cargo run -p client-ui`.

---

## 4) Development Architecture (Responsibilities)

* **game-core**

  * Pure state machine reducer: data types for state/action plus `step(state, env, action) -> state'`.
  * Each action implements `ActionTransition` hooks (`pre_validate → apply → post_validate`) so software and zk proofs enforce the same constraints.
  * Integer/fixed‑point arithmetic only; deterministic tick length; no RNG inside core.
  * Interfaces for Map/Inventory/Status oracles (callers provide proofs/witnesses).

* **game-client (Bevy)**

  * ECS systems: input → actions; render from canonical state snapshot.
  * Plugins: networking (HTTP/gRPC for server), prover adapter, submission packager.
  * Deterministic loop: fixed cadence driving `step`.

* **proofs**

  * Facade with two backends:

    * **zkVM**: wrap `game-core` logic as a guest; optionally compress receipts.
    * **Plonkish**: implement essential invariant checks as circuits.
  * APIs returning a `ProofBundle` (opaque bytes + structured metadata), with optional recursion.

* **game-server**

  * Stateless endpoints to issue signed NPC orders based on context.
  * Optional: VRF output endpoint and append‑only audit log (periodic root publication).
  * Key management & rotation; on‑chain registry for the current public key.

* **onchain/contracts**

  * Verifies proofs, checks/rejects replays, updates player state commit, emits events.

---

## 5) Runtime Flows (High‑Level)

**A. Local Play & Prove**

1. Client simulates turns step-by-step with `game-core`, collecting minimal witnesses.
2. Client obtains NPC orders from the server (signed), if any.
3. Prover produces a proof asserting compliance with invariants and inclusion of authorized NPC inputs.
4. Client submits `(proof_bundle, start_commit, end_commit, metadata)` to the contract.

**B. On‑Chain Update**

1. Contract verifies proof and ensures `start_commit` matches stored state.
2. Contract records nullifier(s) and updates the stored `end_commit`.
3. Events emitted for indexers/UI.

**C. NPC Order Issue (Authority)**

1. Server computes actions from current context.
2. Server returns signed orders; optionally logs entries and updates an audit root.

---

## 6) Technology Stack & Tooling

* **Language**: Rust 2024+, Cargo workspaces.
* **UI/Client**: Bevy (ECS, rendering, input). Optional egui overlays and cli.
* **Async & Net**: Tokio, axum/reqwest or tonic (gRPC) for server ↔ client.
* **Proving**:

  * **Option A (iteration)**: zkVM (RISC Zero or SP1) to reuse `game-core`; optional receipt compression.
  * **Option B (cost)**: Plonkish (Halo2/Plonky2) for essential invariant circuits.
* **Crypto**: Poseidon hash; Merkle trees for map/inventory/status; server signatures (algorithm TBD); optional VRF library.
* **On‑Chain**: Solidity + Foundry; client bindings via alloy.
* **Local chain**: Anvil (for dev/test), EVM testnet for staging.
* **CI/CD**: GitHub Actions (lint, unit/integration, Foundry tests).

---

## 7) Interfaces (Abstract, Example Purpose Only)

* **Core API**

  * `step(env, state, action) -> state'`
  * `build_commit(state) -> StateCommit`

* **Prover API**

  * `create_proof(start_commit, end_commit, actions, witnesses) -> ProofBundle`
  * `verify_locally(bundle) -> bool`

* **Server API (Authority)**

  * `POST /npc/order` → returns an *OrderToken* (signed NPC actions for a given context).
  * `GET /audit/root` (optional)
  * `GET /health`, `GET /version`

* **Contract Interface (EVM)**

  * `submitProof(proofBundle, startCommit, endCommit, meta)`
  * Events: `ProofAccepted(player, startCommit, endCommit)`, `ProofRejected(reason)`

> Exact schemas and encodings are intentionally left open; implementations must tie all tokens/commits to game/session/turn context.

---

## 8) Configuration, Keys, and Environments

* **Envs**: dev → staging (EVM testnet) → prod.
* **Server keys**: offline‑generated; rotation procedure; on‑chain key registry.
* **Parameters**: Poseidon/Merkle constants, movement/cost tables, commitment versions; versioned across components.

---

## 9) Testing & Quality Gates

* **Unit**: core transitions and invariants.
* **Property‑based**: randomized action sequences to ensure determinism & invariant preservation.
* **Integration**: client ↔ prover round‑trip; negative tests (tampered witnesses/orders).
* **On‑Chain**: Foundry tests for verifier/state updates/replay attempts.
* **E2E**: local harness with client+server+coordinator+contracts.
* **Security reviews**: checklist‑driven; third‑party audit before production.

Definition of Done (per feature): tests passing, metrics instrumented, docs updated, feature flags/version bumps applied.

---

## 10) Performance & Telemetry

* **Targets**: tune proof submission cadence/size per mode; local proving within acceptable latency on dev hardware (targets TBD by team).
* **Metrics**: proof time, turns per submission, rejection reasons, server order latency, contract verify gas.
* **Logging**: structured tracing across components; correlation IDs for each proof submission.

---

## 11) Security & Privacy Checklist

* Deterministic core; no floating point; RNG outside core only.
* Replay protection via monotonic turn index + nullifier registry on chain.
* All cross‑component inputs validated by commitments; plaintext treated as untrusted.
* Rate‑limit server APIs; protect keys; audit trail if audit mode enabled.
* Gate upgrades behind versioned contracts and explicit migrations.

---

## 12) Implementation Roadmap

### Phase 0 — Planning & Scaffolding (wk 0)

* Lock workspace layout (done) and converge on design docs (`architecture.md`, `research.md`).
* Identify team owners per crate; enumerate external dependencies (zkVM, Bevy, networking).
* Define success metrics for each subsequent phase (proof latency, CI coverage, etc.).

### Phase 1 — Deterministic Core Foundations (wks 1–2)

* Build `types` primitives (commitments, IDs, map metadata) with serialization strategies.
* Implement `game-core` state model, action set, environment inputs, and `step`.
* Add unit/property tests, fixture maps, and documentation of invariants.

### Phase 2 — Proof Facade MVP (wks 2–4)

* Stand up `proofs` crate with trait-driven interface for backends (zkVM first, Plonkish stub).
* Implement witness builders bridging `game-core` to proof inputs; provide local verification helpers.
* Establish dummy proof pipeline and CI job to guard serialization compatibility.

### Phase 3 — Runtime Skeleton (wks 3–5)

* Flesh out `client/runtime` modules: command API, event bus, simulation/proof queues.
* Integrate `game-core` and `proofs` crates; stub out chain/server adapters behind traits.
* Provide integration tests for action replay, proof request lifecycle, and failure handling.

### Phase 4 — Interfaces: CLI & UI (wks 4–6)

* `client/cli`: implement subcommands (`play`, `prove`, `submit`, `inspect`, `bench`) calling runtime API.
* `client/ui`: stand up Bevy harness, snapshot rendering, and user input → runtime command bridge.
* Define shared configuration (YAML/TOML) for runtime parameters, documented under `crates/client`.

### Phase 5 — NPC Authority & Networking (wks 5–7)

* Implement `server` crate with signed order issuance, VRF/audit hooks, and REST/gRPC endpoints.
* Wire client networking adapters (reqwest/tonic) and error handling backoff policies.
* Add load/perf tests for order latency and resilience (retry, rate limits).

### Phase 6 — On-Chain Contracts & Integration (wks 6–9)

* Prototype verifier/state contract in `onchain/contracts` with Foundry tests.
* Connect runtime submission flow to local Anvil; add e2e harness covering client → proof → contract.
* Measure proof verification gas and iterate on circuit/front-end adjustments as needed.

### Phase 7 — Hardening & Observability (wks 8–10)

* Embed tracing/metrics across crates; define dashboards (proof latency, gas, NPC order health).
* Conduct security review checklist, add fuzz/property tests, and document threat mitigations.
* Draft release procedure: versioning, key rotation runbook, staging rollout plan.

Dependencies across phases: Phase (n) should land before the dependent work in Phase (n+1) begins; parallelization is possible when interfaces are frozen. Each phase concludes with updated documentation and CI gates (`cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace`, targeted integration suites).
