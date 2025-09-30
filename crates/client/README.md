# Agent-Centric Client Architecture — **Agent / UI / CLI**

*Mode: Light ZK + Single Server · Chain: EVM · Core is deterministic & proof-friendly*

---

## 1. Overview

The **client** folder contains three sub-crates:

```

client/
├─ agent/   # library crate: authoritative runtime, API
├─ cli/     # binary crate: headless automation, DevOps
└─ ui/      # binary crate: Bevy visualization & control

```

- **Agent** is the authoritative runtime/daemon.  
- **CLI** is a thin wrapper for headless automation.  
- **UI** is a Bevy app for visualization & input.  
- All crates depend on the **core rules** and **proofs** libraries.

---

## 2. Agent: Mission & Scope

**The Agent owns**
- Authoritative simulation of core (`step`).  
- Proof pipeline: witness capture + proving backend (zkVM/Plonkish).  
- Chain I/O: submit to EVM, track receipts, manage gas/nonces.  
- Server I/O: consume signed NPC orders.  
- State custody: journal commits, crash recovery.  
- Event hub: pub/sub stream to UI/CLI.  
- Secrets boundary: RPC keys, proving keys (never exposed to UI/CLI).  

**The Agent does *not* own**
- Rendering, input, physics → belong to UI.  
- Human-facing tooling → belongs to CLI.  

---

## 3. Agent Architecture

- **Runtime**: Tokio, fixed turn index.  
- **Queues**:  
  - `sim_queue`: deterministic step → state + witnesses.  
  - `proof_queue`: heavy proof jobs.  
  - `submit_queue`: EVM submission + retries.  

**Modules**
- `core_adapter`, `witness`, `prover`, `chain`, `server`, `journal`, `bus`, `api`.

**Patterns**
- Command/Query separation.  
- Pub/Sub events (`SnapshotUpdated`, `ProofProgress`, `ProofSubmitted`, `ProofFailed`).  
- Record/replay logs for reproducibility.  

---

## 4. Interfaces

**Commands → Agent**
```

StartSession, ApplyAction, RequestProof, SubmitProof

```

**Queries → Agent**
```

GetSnapshot, GetMetrics, GetJournal

```

**Events ← Agent**
```

SnapshotUpdated, ProofProgress, ProofSubmitted, ProofFailed

```

**Transport**
- In-process channels (MVP).  
- gRPC/WebSocket (when agent runs as separate process).  

---

## 5. End-to-End Flow

1. UI/CLI sends actions.  
2. Agent simulates → new state + witnesses.  
3. Agent authorizes NPC orders.  
4. Agent proves → emits progress.  
5. Agent submits proof to EVM.  
6. Agent journals results & emits snapshot.  

---

## 6. UI (Bevy)

- Thin Bevy app: input → `ApplyAction`; render from `Snapshot`.  
- HUD shows progress/logs.  
- Plugins: `AgentClientPlugin`, `SnapshotStreamPlugin`.  
- No secrets or proofs in UI.  

---

## 7. CLI

- Subcommands: `play`, `prove`, `submit`, `inspect`, `bench`.  
- Headless; reuses Agent API.  
- JSON logs for CI; optional TUI.  

---

## 8. Security & Reliability

* Secrets live in Agent only.
* Anti-replay: monotonic turn index + nullifier cache.
* Crash-safe journal.
* Bounded queues for backpressure.
* Observability: `tracing`, metrics.
