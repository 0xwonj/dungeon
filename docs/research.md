# ZK Turn‑Based RPG — High‑Level Blueprint (Light ZK + Single Server)

## 1) Executive Summary

Default architecture: **Light ZK + Single Server authority**. Players execute turns locally, generate ZK proofs that enforce **essential invariants** (movement bounds, collision, cooldown/resource conservation, replay protection), and submit succinct evidence to **EVM on‑chain** contracts. NPC actions are accepted when **signed by a trusted server**, with audit options.

Alternative modes (for later consideration): **Full ZK** (prove NPC logic), **Committee signatures** (BLS), **L2 deployment** for cost/throughput.

---

## 2) System Overview

**Roles**

* **Client** (Rust; optional Bevy UI): Renders gameplay, runs deterministic core, collects witnesses, packages proof submissions.
* **Prover** (Rust; zkVM or Plonkish): Creates proofs (optionally over short sequences of turns).
* **NPC Authority (Single Server)**: Issues signed NPC orders; can publish audit logs and VRF randomness.
* **Coordinator** (optional): Relay/service; monitors on‑chain events.
* **On‑Chain (EVM)**: Verifier + minimal state (player state commitments, nullifiers), reward/settlement logic.

**Core commitments**

* `R_map`: Poseidon‑Merkle root of map tiles (walkable, cost, tags).
* `S_t`: Player state commitment (x,y,hp,mp,status_root,inv_root,turn_idx,player_pk,…).
* `nullifier_t`: Anti‑replay tag derived from `(player_pk, turn_idx, S_t)`.

---

## 3) End‑to‑End Pipeline

1. **Bootstrap**: Contract stores `R_map`, `S_0`, `game_id` (and optional randomness anchor).
2. **Local Play**: Client executes turns step‑by‑step deterministically → state commits + witnesses.
3. **NPC Inputs**: Client fetches per‑turn server‑signed orders `σ_i` and (optional) VRF/audit proofs.
4. **Proving**: Prover asserts transitions satisfy invariant checks and that NPC orders are validly signed; compresses nullifiers (`acc_nullifier`) and signatures (`npc_sig_commit`).
5. **Submission**: `(proof, S_start, S_end, acc_nullifier, npc_sig_commit, meta)` sent to contract.
6. **On‑Chain Verify**: Contract checks `state[player]==S_start`, verifies proof (and optional signature/VRF if kept on chain), marks `acc_nullifier` used, updates `S_end`.

---

## 4) Protocol Sketch (Light ZK)

* **Per‑turn message**: `msg_i = H(game_id || turn_i || S_{i-1} || npc_action_i || beacon_slot_i)`.
* **Server signature**: `σ_i = Sign_sk_server(msg_i)`; client includes `σ_i` (or a commit) per submission.
* **Auditability (optional)**: Server maintains an append‑only Merkle log of `msg_i` and periodically publishes/log‑signs the root; clients can provide membership proofs when challenged.
* **Randomness**: `VRF_y = VRF(sk_server, beacon_slot)` feeds NPC policy; verify on chain or inside ZK depending on cost.

**Public inputs**: `game_id, R_map, S_start, S_end, turn_idx (or span), acc_nullifier, npc_sig_commit`.

---

## 5) Threat Model & Mitigations (Essential Invariants)

| Threat                | Example                                            | Mitigation                                                                                                                         |
| --------------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Client cheating       | Wall‑clipping, cooldown bypass, resource inflation | ZK constraints: movement bounds, tile Merkle proofs, lookup‑based costs, cooldown/resource invariants                              |
| Replay/duplication    | Reusing old submissions                            | Monotonic `turn_idx`, on‑chain nullifier set, start/end state consistency                                                          |
| Malicious server      | Biased NPC orders or collusion                     | Signed orders tied to `(game_id, turn, S_prev)`; optional VRF and Merkle audit log; escalation path to Full ZK/committee if needed |
| RNG manipulation      | Predictable or ex‑post chosen randomness           | Public beacon + VRF; fixed beacon slots; commit‑reveal fallback                                                                    |
| Circuit/contract bugs | Constraint gaps, verifier issues                   | Independent implementation for testing, property tests, audits, canary deploys                                                     |

---

## 6) Performance & Cost (High‑Level)

* **Proof strategy**: Per-step or short-sequence proofs; recursive folding for long sessions; zkVM → SNARK compression when using zkVM.
* **Constraint economy**: 1‑step Manhattan movement; integer/fixed‑point arithmetic; lookup tables for costs/skills.
* **Data minimization**: Commitments only on chain (`S_t`, `R_map`); compress per‑turn artifacts (`acc_nullifier`, `npc_sig_commit`).
* **Signature checks**: Prefer verifying server signatures inside the ZK proof; if on chain, aggregate to minimize pairings.

---

## 7) Architecture Split (Responsibilities)

**Client**: Deterministic core execution; witness collection; proof packaging; retries/fee estimation.

**Prover**

* Mode A: **zkVM** (RISC Zero / SP1) to reuse Rust core; SNARK‑compress receipts.
* Mode B: **Plonkish circuits** (Halo2/Plonky2) implementing only essential invariants.

**NPC Server**: Signs orders; exposes VRF endpoint and audit‑log root; rotates keys via on‑chain registry.

**Coordinator** (optional): Relay; on‑chain event watcher.

**On‑Chain (EVM)**: Verifier contract; `(player → S_t)` mapping; nullifier registry; optional on‑chain signature/VRF checks.

---

## 8) Data Model (Commitments)

* **State**: `S = Poseidon(x,y,hp,mp,status_root,inv_root,turn_idx,player_pk, …)`.
* **Map**: `R_map = MerkleRoot(Poseidon(x,y,walkable,cost,tag))`.
* **Inventory/Status**: Separate Merkle roots to keep proofs narrow.

---

## 9) Considerations & Future Options

* **Full ZK validation**: Prove NPC transitions (for competitive/high‑stakes modes).
* **Committee signatures**: Replace single server with BLS (t‑of‑n) aggregation for trust distribution.
* **L2 deployments**: If/when needed, migrate verifier/contracts to an L2; consider Stylus‑style Rust contracts for specific environments.

---

## 10) Open Questions & Research Tasks

* **Proof backend choice**: Compare RISC Zero vs. SP1 vs. Plonky2 for developer ergonomics, performance, and licensing; prototype benchmarks (proof time, memory footprint) with representative action sequences.
* **Signature scheme**: Decide between Ed25519, secp256k1, or BLS for NPC orders; evaluate verifier cost (in proof vs. on-chain) and key management implications.
* **Networking protocol**: Determine whether HTTP/JSON or gRPC suits agent ↔ server messaging best; measure latency and streaming support for live sessions.
* **State commitment design**: Validate Poseidon parameters, tree arity, and collision resistance; confirm compatibility with chosen proof backend.
* **Submission policy**: Model latency vs. gas/proof amortization for different proof spans, and define dynamic adjustment rules for PvE/PvP modes.
* **Nullifier compression**: Investigate accumulators or rolling hashes to reduce public input size without weakening replay protection.
* **Audit log storage**: Explore append-only Merkle log retention (on server vs. decentralized storage) and challenge-response workflows.
* **Client platform targets**: Assess feasibility of WebAssembly build for agent/cli, and implications on cryptography/proof tooling.

---

## 11) Risks & Mitigations To Validate

* **Proof performance**: Risk of exceeding acceptable latency; mitigate via early benchmarking and fallback to lighter invariant set.
* **Server trust assumptions**: Single authority compromises fairness; plan for VRF transparency and path to multi-signer upgrade.
* **Map/inventory growth**: Large Merkle proofs could bottleneck; consider segmented trees or on-demand prefetching.
* **Client determinism**: Divergent behavior across platforms; enforce integer math, add reproducibility tests, and capture execution traces.
* **Key custody**: Server key leakage or rotation mishaps; formalize HSM usage or threshold signing research milestone.
* **On-chain verification cost**: Gas spikes for proof verification; monitor during prototyping and budget for verification circuit optimizations.
