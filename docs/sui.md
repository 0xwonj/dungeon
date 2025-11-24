# 1. Goal & Trust Model (High-level)

* The **off-chain** system (Rust TUI/runtime) executes the entire game deterministically → produces **proofs, logs, and commitments**.
* The **on-chain** system (Sui + Move contracts) only verifies minimal commitments and proofs, ensuring **correctness and integrity** of the off-chain computation.
* **Walrus** guarantees data availability for large logs or replay packages (without storing them on-chain).
* **Optimistic AI challenge**: AI logic is public and lower-security; correctness is assumed unless challenged — a small proof is then required for the disputed turn.

---

# 2. On-chain Components (Modules & Objects)

### A. **Data / Definition Layer**

* **Oracle Registry**
  Stores **static game data snapshots** — maps, items, tables — with their version, root hash, and Walrus `BlobRef (id, hash)`.
  Provides deterministic reproducibility across game runs.

* **Player Profile**
  Maintains the player’s **persistent state root** (resources, permanent upgrades, etc.).
  At game start, this root is recorded as the run’s initial condition.

---

### B. **Execution / Verification Layer**

* **Game Core**

  * `RunTicket`: The starting point of a play session.
    Includes: player, oracle_root, profile_root_at_start, seed_commitment, expiration, and usage flag.
  * `Claim`: The result of a completed run, including final state root, score, action log Merkle root, Walrus references, and status.
  * Calls `verify_state` to check the ZK proof against the declared public inputs.

* **ZK Verify**
  Stores verifying keys (curve, version, length) and exposes a unified verification wrapper.
  Defines consistent **public input schema and endian conventions**.

* **Rewards**
  `RewardPool` tracks reward policy and prevents double claims.
  `finalize_and_reward` confirms a claim and distributes points or tokens.

---

### C. **Dispute / Challenge Layer (Optional)**

* **Challenge Module**
  Allows **turn-level verification** if a player’s AI behavior is disputed.
  Uses **bond + timeout + proof window** to resolve conflicts via ZK proofs for specific actions.

---

# 3. Off-chain Components (Integration Path)

* **Rust TUI Runtime**:
  Executes all game logic, generates checkpoints, final state root, score, and ZK proofs.

* **Artifacts Folder**:
  Contains committed inputs and outputs:

  ```
  oracle_root.hex
  profile_root.hex
  seed_commit.hex
  final_state_root.hex
  score.u64
  action_log.merkle_root.hex
  state_proof.groth16
  (optional) ai_turn_i.groth16
  ```

* **Walrus Uploader**:
  Uploads logs/replay packages → returns `BlobID` and `hash` for on-chain submission.

* **Sui SDK (Rust)**:
  Issues transactions: `start_run` → `publish_claim` → `finalize`.

---

# 4. Data Schema & Commit Interface

* **Hash Scheme**: Keccak-256 (32 bytes, fixed endianness).
* **Public Inputs for State Proof**:

  1. `oracle_root`
  2. `profile_root_at_start`
  3. `seed_commitment`
  4. `final_state_root`
  5. `score` (u64 → u256)
  6. `run_hash` = keccak(action_log_root || ticket_id || vk_version)
* **Walrus Links**:

  * `walrus_log(id, hash)`
  * optional: `walrus_final(id, hash)` (checkpoint or replay bundle)

---

# 5. Lifecycle (Game Session Timeline)

```
(1) Setup
  - Register oracle snapshot
  - Register ZK verifying key
  - Initialize reward pool

(2) Player Onboarding
  - Initialize or update PlayerProfile (persistent root)

(3) Run Start
  - start_run(profile_ref, oracle_ref, seed_commit, ttl) → RunTicket(shared)

(4) Off-chain Execution
  - Game runs locally → produces action_log, final_state_root, score, proof
  - Upload logs to Walrus → get blob IDs + hashes

(5) Claim Submission
  - publish_claim(ticket, final_root, score, roots, walrus_refs, proof, inputs)
  - verify_state(vk, proof, inputs) == true → Claim(PUBLISHED)

(6) Finalization & Reward
  - optional challenge window
  - finalize_and_reward(claim, pool) → reward distributed

(7) Dispute (optional)
  - open_challenge(claim, turn_idx, bond)
  - resolve_with_ai_proof(...) or timeout_slashed(...)
```

---

# 6. Access Control & Guards

* **RunTicket**: single-use, expires after TTL.
* **Claim**: unique per `run_hash`; state transitions only allowed `PUBLISHED → FINALIZED/VOID`.
* **Reward**: one-time payout per claim (provable invariant).
* **Walrus Data**: enforce hash/length/format guard.
* **Challenge**: ensure valid bond, time window, and prevent duplicates.

---

# 7. Events & Observability

* `RunStarted(player, ticket_id, oracle_version, seed_commit)`
* `ClaimPublished(claim_id, player, score)`
* `ClaimFinalized(claim_id, reward)`
* `Challenged(claim_id, turn_idx)`
* `ChallengeResolved(claim_id, ok)`

> The explorer or dashboard can reconstruct the full state transitions just by subscribing to events.

---

# 8. Failure & Recovery

* Proof verification failure → `publish_claim` reverts → player regenerates proof.
* Walrus upload failure → retry allowed within ticket TTL.
* Oracle / VK updates mid-run are safe → version and root are locked in each RunTicket.
* Node restart or chain reset → no issue, as all objects persist on-chain.

---

# 9. Gas & Performance

* On-chain state = **only 32B roots, u64 score, and compact metadata**.
* Large logs stored in Walrus only (hash references on-chain).
* ZK cost ∝ number of public inputs → minimized schema.
* PTB batching can combine multiple calls (`publish + finalize`) atomically.

---

# 10. Governance / Upgradability

* Config registries are independent shared objects:

  * `OracleRegistry`, `VKRegistry`, `RewardPolicy`
* Updated through admin or multisig.
* Each RunTicket fixes versions at creation, ensuring deterministic verification.


## 🧩 Summary

* **On-chain** = `RunTicket → Claim (ZK verify) → Reward`
* **Off-chain** = deterministic execution, proof generation, Walrus upload.
* **Challenge layer** = optional optimistic dispute on AI logic.

This layering ensures your **game runtime, ZK pipeline, and data proofs** can evolve freely — the on-chain interface remains **stable, auditable, and minimal**.
