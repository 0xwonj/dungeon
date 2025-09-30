# Dungeon — `agent` Specification

*Mode: **Light ZK + Single Server** · Chain: **EVM** · Principle: **Functional core, imperative shell***

This document defines the **`dungeon-agent`** crate: its purpose, public API (ports), internal architecture (queues, workers, repositories), how it integrates with `dungeon-core`, `dungeon-proofs`, storage, and EVM. It complements the `game-core` spec and fixes the boundary between the two.

---

## 1) Mission & Non-Goals

### Mission (what `agent` **is**)

* The **authoritative runtime** that owns: time/scheduling, state custody, witness collection, proof generation, EVM submission, persistence, and secrets.
* The only component that **implements oracles** (`MapOracle`, `TablesOracle`) and exposes a **stable API** to UI/CLI (commands/queries/events).

### Non-Goals (what `agent` **is not**)

* No rendering or user input handling (UI’s job).
* No domain rule decisions beyond calling `game-core` (core’s job).
* No on-chain rule interpretation; the chain just verifies proofs.

> If it touches I/O, crypto, DB, networking, retries, or secrets → it belongs in **`agent`**.

---

## 2) Public API (Ports) — transport-agnostic

Expose **ports** (traits). Provide an **in-process adapter** now; later you can add gRPC/WebSocket clients/servers without changing UI/CLI code.

```rust
// crates/client/agent/src/api.rs
use dungeon_core::{Action};

pub struct Snapshot {
    pub state_commit: [u8; 32],
    pub turn_idx: u64,
    // Optional: render-friendly view (positions, stats) derived from State
}

pub struct AgentMetrics {
    pub proof_queue_len: usize,
    pub submit_queue_len: usize,
    pub avg_proof_ms: u64,
    pub last_gas_used: Option<u64>,
}

pub enum Event {
    SnapshotUpdated { commit: [u8; 32], turn_idx: u64 },
    ProofProgress   { proof_id: u64, pct: u8 },
    ProofSubmitted  { proof_id: u64, tx_hash: [u8; 32] },
    ProofFailed     { proof_id: u64, reason: String },
}

#[async_trait::async_trait]
pub trait AgentControl {
    async fn start_session(&self, game_id: [u8;32], map_root: [u8;32]) -> anyhow::Result<()>;
    async fn apply_action(&self, action: Action) -> anyhow::Result<()>;
    async fn submit_pending(&self) -> anyhow::Result<()>; // force-submit queued proofs
}

#[async_trait::async_trait]
pub trait AgentQuery {
    async fn snapshot(&self) -> anyhow::Result<Snapshot>;
    async fn metrics(&self) -> anyhow::Result<AgentMetrics>;
}

pub trait AgentEvents {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event>;
}
```

**Adapters (initial + future)**

* `InProcAgentClient` (MVP): wraps the runtime in the same process using channels.
* `GrpcAgentClient` / `WsAgentClient`: later, when you run `agentd`.

---

## 3) Internal Architecture

```
                    +---------------------+
Commands (mpsc) --->|  Command Handler    |----+
                    +----------+----------+    |
                               |               |
                               v               |
                       +-------+-------+       |
                       |   sim_queue   |       |
                       +-------+-------+       |
                               |               |
                               v               |
                 +-------------+--------------+|
                 |    Simulation Worker       ||
                 |  (calls game-core)         ||
                 +-------------+--------------+|
                               | witness deltas
                               v
                       +-------+-------+
                       |  proof_queue  |
                       +-------+-------+
                               |
                               v
                   +-----------+------------+
                   |   Proof Worker(s)      |
                   | (calls dungeon-proofs) |
                   +-----------+------------+
                               |
                               v
                       +-------+-------+
                       | submit_queue  |
                       +-------+-------+
                               |
                               v
                   +-----------+------------+
                   |   Submit Worker        |
                   | (ethers/alloy → EVM)   |
                   +-----------+------------+

Events (broadcast): SnapshotUpdated / ProofProgress / ProofSubmitted / ProofFailed
Queries: read snapshot/metrics from Agent state & repos
```

### 3.1 Tasks & Queues

* **Command handler**: validates commands, appends to `sim_queue`.
* **Simulation worker**: builds `Env` (oracles) and calls `core::step` one action at a time; collects `WitnessDelta`s, updates local state, emits `SnapshotUpdated`, enqueues a `ProofJob`.
* **Proof worker(s)**: heavy jobs via `spawn_blocking` or a worker threadpool; calls `dungeon-proofs`; reports `ProofProgress`; enqueues `SubmitJob`.
* **Submit worker**: sends tx via `ethers`/`alloy`, retries with backoff; upon receipt, updates repos and emits `ProofSubmitted` / `ProofFailed`.

**Determinism & safety**: single writer principle for state; bounded queues to apply backpressure.

---

## 4) Oracles & Repositories

### 4.1 Oracles (core-facing, read-only)

`agent` **implements** `MapOracle` and `TablesOracle` backed by repositories + in-memory caches.

```rust
struct MapOracleImpl { root: [u8;32], cache: LruCache<(i32,i32), Tile>, map_repo: Arc<dyn MapRepo> }
impl MapOracle for MapOracleImpl {
    fn tile(&self, x: i32, y: i32) -> Tile {
        // 1) try LRU
        // 2) else load from DB via MapRepo (already verified on ingestion)
        // 3) fallback: block or return a sentinel (design choice) if not present
    }
}

struct TablesOracleImpl { version: u32, table_repo: Arc<dyn TablesRepo> }
impl TablesOracle for TablesOracleImpl {
    fn skill_cost(&self, id: u16) -> (i32,i32) { /* read constants */ }
    fn max_step(&self) -> i32 { /* read constant */ }
}
```

> **Ingestion** of tiles/orders must verify proofs/signatures **before** they enter repos (anti-corruption layer).

### 4.2 Repositories (agent-side, behind traits)

Use traits for testability; start with SQLite or sled; add caches where it helps.

```rust
#[async_trait::async_trait]
pub trait StateRepo {
    async fn latest(&self, session: Uuid) -> anyhow::Result<Option<StateRow>>;
    async fn append(&self, session: Uuid, turn_idx: u64, commit: [u8;32], state_json: String) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait MapRepo {
    async fn get_tile(&self, root: [u8;32], x: i32, y: i32) -> anyhow::Result<Option<Tile>>;
    async fn put_tile_verified(&self, root: [u8;32], x: i32, y: i32, tile: Tile, proof_blob: Vec<u8>) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait NpcRepo {
    async fn put_order_verified(&self, turn_idx: u64, order_blob: Vec<u8>) -> anyhow::Result<()>;
    async fn get_order(&self, turn_idx: u64) -> anyhow::Result<Option<NpcOrder>>;
}

#[async_trait::async_trait]
pub trait ProofRepo {
    async fn enqueue(&self, pending: PendingProof) -> anyhow::Result<ProofId>;
    async fn mark_proven(&self, id: ProofId, bundle: ProofBundle) -> anyhow::Result<()>;
    async fn mark_submitted(&self, id: ProofId, tx: [u8;32]) -> anyhow::Result<()>;
    async fn mark_finalized(&self, id: ProofId, receipt: TxReceipt) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait ReceiptRepo { /* ... */ }
#[async_trait::async_trait]
pub trait NullifierRepo { /* ... */ }
#[async_trait::async_trait]
pub trait MetricsRepo { /* ... */ }
```

---

## 5) Simulation & Witness Collection

### 5.1 Building `Env`

* On each sim job, construct `Env { map: &MapOracleImpl, tables: &TablesOracleImpl }` scoped to the **current `map_root` and tables version**.

### 5.2 Calling `game-core`

* Apply actions by calling `core::step(&env, &state, &action)` one at a time, in order. Accumulate `WitnessDelta`s for proof construction.

### 5.3 From deltas to transcript

* Collect `Vec<WitnessDelta>` into a **witness transcript**:

  * (x,y,tile) touches
  * resource deltas
  * cooldown checks / table version
  * bind context: `{ game_id, map_root, core_version, turn_idx, player_addr }`
  * attach **server-signed NPC orders** for those turns (after signature verification).

---

## 6) Commit & Proof Pipeline

### 6.1 Commit hashing (agent-side)

Use the **commit shape** from core to ensure canonical ordering; the agent selects the hash (e.g., Poseidon).

```rust
fn commit_state(s: &dungeon_core::State) -> [u8;32] {
    use dungeon_core::FieldInput;
    let iter = dungeon_core::state_fields_order(s);
    // map FieldInput -> field elements; feed Poseidon/MiMC in the exact order; serialize as bytes32
}
```

**Cautions**

* Fix integer encoding and endianness in one place; fuzz test vs a second implementation.
* Include `CORE_VERSION` (already in shape) so commits change on breaking rules.

### 6.2 Proving

```rust
let bundle = dungeon_proofs::prove(ProofInput {
    start_commit,
    end_commit,
    witness_transcript,
    public_meta: PublicMeta { game_id, map_root, core_version, turn_idx, player_addr },
})?;
```

* Run in a `spawn_blocking` thread; throttle parallelism.
* Optionally `verify_local(&bundle)` before enqueueing submission.

---

## 7) EVM Submission

* Build an **idempotency key** `proof_id = keccak(prev_commit || next_commit || turn_idx || player_addr)`.
* Submit `submitProof(proof, prevCommit, nextCommit, meta)` via `ethers`/`alloy`.
* **Retry/backoff** on transient errors; **mark_finalized** after receipt confirmations.
* Cache **nullifiers** client-side to prevent double-spend attempts prior to finality.

---

## 8) State & Journaling

* Append `(turn_idx, commit, state_json)` to `StateRepo` after simulation (before proving) so crash recovery can resume.
* On startup: load latest session, reconstruct snapshot, resume pending proofs.
* Maintain **metrics** (proof time, queue length, error codes) in `MetricsRepo`.

---

## 9) Configuration

Support file + env overrides:

```toml
# agent.toml
[session]
game_id = "0x..."
map_root = "0x..."

[proofs]
backend = "zkvm"     # or "plonkish"
max_parallel = 2

[chain]
rpc_url = "https://..."
contract = "0x..."
gas_policy = "auto"

[storage]
backend = "sqlite"   # or "sled"
path = "dungeon.db"

[security]
keystore = "os-keychain"  # or "file:~/.dungeon/keystore.json"
```

---

## 10) Concurrency, Backpressure, Errors

* **Single writer** to state; serialize commands.
* Bounded `mpsc` queues; if full, return a typed error (`QueueFull`) to callers.
* Typed error categories: `InvalidInput`, `OracleUnavailable`, `ProofBackend`, `EvmRpc`, `Storage`, `Retryable`, `Fatal`.
* Surface **progress events** early; keep UI responsive.

---

## 11) File Layout (crate)

```
crates/client/agent/
  src/
    lib.rs            // re-exports; constructors for InProcAgentClient
    api.rs            // ports (traits) + DTOs (Snapshot, Metrics, Event)
    runtime.rs        // queues, workers, scheduler, event bus
    oracles.rs        // MapOracleImpl, TablesOracleImpl
    transcript.rs     // build witness transcript from WitnessDelta + context
    commit.rs         // commit_state() using core::state_fields_order
    prover.rs         // calls to dungeon-proofs (+ feature flags)
    chain.rs          // ethers/alloy tx submission, receipts
    repos/
      mod.rs          // traits: StateRepo, MapRepo, ...
      sqlite.rs       // sqlite implementation (or sled.rs)
    config.rs         // load/merge config
    errors.rs         // AgentError categories
```

---

## 12) Minimal Boot (in-proc)

```rust
// in lib.rs
pub fn new_inproc_agent(cfg: Config) -> anyhow::Result<InProcAgentClient> {
    // init repos, caches, event bus
    // start tokio tasks: sim_worker, proof_worker(s), submit_worker
    // return a handle implementing AgentControl + AgentQuery + AgentEvents
}
```

UI/CLI will call `new_inproc_agent(config)` and then use the **ports** only.

---

## 13) Security & Determinism Guardrails

* **Secrets** (wallet, proving keys, RPC tokens) live only in Agent; never leak to UI/CLI.
* **Verify before trust**: Merkle proofs and signatures are validated during ingestion (before repos/oracles).
* **No floats or time** influence rule decisions; core handles all rules; agent ensures consistent oracles.
* **Record/Replay** step-by-step: record each `(S_i, a_i) → S_{i+1}` with transcript hash; re-run to assert equality.
* **Canonical commits** only via `commit_state()`; forbid ad-hoc hashing elsewhere.

---

## 14) Testing

* **Unit**: repos (CRUD), commit coder (cross-check vs alt impl), tx builder, error mapping.
* **Integration**: sim→prove→submit happy path; retries; crash+resume; backpressure; invalid NPC signatures.
* **E2E (dev)**: Anvil chain + stub NPC server + sample map; run a short dungeon and assert on events/receipts.
* **Fuzz**: feed random legal actions; assert no panics and stable commits.

---

## 15) Migration to `agentd` (daemon) — later

* Keep ports (traits) as is.
* Add `agentd` binary exposing gRPC/WS; map 1:1 from RPC to ports.
* UI/CLI switch adapters from `InProcAgentClient` → `GrpcAgentClient`.
* Secrets/config remain in the daemon process; multiple clients supported.

---

### TL;DR

* `agent` is the **imperative shell** around `game-core`: it **implements oracles**, **collects witnesses**, **proves**, **submits**, and **journals**.
* Expose **ports** (AgentControl/Query/Events); run **workers** behind **bounded queues**; store everything behind **repository traits**.
* Use core’s **commit shape**; keep secrets in Agent; validate inputs before they reach oracles; record/replay for confidence.
* Start **in-proc**; you can flip to a **daemon** without touching UI/CLI or core.
