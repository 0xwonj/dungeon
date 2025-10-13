# Game Philosophy

> *“When rules are honest, stories write themselves.”*

Dungeon is not only a technical experiment in verifiable computation — it’s a statement about what games could become when **truth and imagination share the same language**.

---

## 1) Worlds That Prove Themselves

Most games ask players to *believe* in their worlds: that the dice rolled fairly, that the AI didn’t cheat, that the system didn’t favor one outcome over another.  
Dungeon doesn’t ask for belief — it offers **proof**.

Zero-knowledge proofs make the world mathematically honest.  
Each state transition, random roll, and interaction between entities can be validated without revealing the underlying secrets of the world. You can trust that your victory wasn’t scripted and your defeat wasn’t rigged — because both are cryptographically inevitable results of the same deterministic laws.

Verifiability isn’t the end goal — it’s the foundation. Once rules are provably fair, we focus on what matters: **worlds worth proving.**

---

## 2) Systems Over Scripts

Dungeon values **emergence** over authorship. Rather than prewritten narratives and brittle quest chains, it builds **interacting systems** — ecology, behavior, factions, item generation — whose collisions produce coherent surprises.  
Procedural generation isn’t a gimmick; it’s a philosophy of *possibility*. Determinism makes that possibility **readable**: same causes, same effects.

The role of the player is not to consume a story, but to **shape** one through interaction with a living rule system.  
A small decision — lighting a torch, closing a door, sparing an enemy — can ripple outward, creating meaning, tension, and sometimes tragedy.

---

## 3) Ownership, Authority, and the Problem With “Game Assets”

“Players own their items” is often an illusion. In most games, assets live on the operator’s servers and are ultimately controlled by the operator:
- Accounts can be **suspended** or wiped.
- Drop rates and stats can be **silently changed**.
- Economies can be **manipulated**.
- Database entries — your “items” — are **editable** at will.

True digital ownership requires more than UI and promises. It requires a substrate where **no one can unilaterally take or alter** what you have.

---

## 4) Why Just “Put It On-Chain” Doesn’t Work

Blockchains seem like the solution: put assets and logic on-chain, and the operator can’t cheat. But fully on-chain games are **impractical**:
- **Cost**: rich simulations are too expensive to execute on-chain.
- **Latency**: block times kill interactivity.
- **Privacy**: everything is public, which breaks many game designs.
- **Expressiveness**: contract languages and execution limits constrain gameplay.

So most “blockchain games” quietly move logic **off-chain** while keeping only thin wrappers on-chain. The result often reverts to **de facto centralization** with a token attached.

---

## 5) ZK as the Missing Bridge

Zero-knowledge proofs bridge the gap between rich, fast **off-chain** play and **on-chain** trust:
- **Off-chain compute, on-chain verification**: run complex logic locally; post succinct proofs that the rules were followed.
- **Selective secrecy**: prove validity **without** revealing spoilers (hidden maps, enemy intent, RNG seeds).
- **Composable trust**: mods, content packs, or third-party services can be accepted if they produce correct proofs.

**Fairness and mystery** coexist: the world stays honest even when parts of it remain hidden.

---

## 6) Single-Player, Without Single-Point-of-Trust

In traditional single-player games, all state lives locally — easy to modify, impossible to police.  
Dungeon takes a different stance:
- Play **locally** for responsiveness and privacy.
- Generate **proofs** for each action or checkpoint.
- Have those proofs **verified on-chain** (or by neutral verifiers) to grant legitimacy — achievements, records, or persistent world effects count **only if** they verify.

This model lets single-player runs be **personally private** yet **publicly credible** when it matters.

---

## 7) Design Principles (Why the Engine Looks This Way)

- **Determinism first**: predictable causality turns complexity into meaning and enables reproducible proofs.
- **Action-validity over full re-execution**: prove that each action satisfied `pre-validate / execute / post-validate` for the prior state, instead of re-running the entire engine in-circuit.
- **Minimal chain footprint**: store proof artifacts and periodic commitments; keep the gameplay loop off-chain and snappy.
- **Modularity**: inputs, oracles, storage, and workers talk via clean contracts; third parties can integrate without privileged trust — **proofs are the gatekeeper**.
- **Players, then proofs**: fun systems and emergent play drive the design; proofs serve the game, not the other way around.

---

## 8) Practical Flows (At a Glance)

**RNG / Loot Roll**
1. Deterministic seed from world commitment + action context.
2. Roll happens locally; engine emits a witness.
3. Prover produces `LootValid(state_prev, action, state_next)`.
4. Verifier (on-chain or off-chain) checks succinct proof; ledger records the outcome hash, **not** the spoiler.

**Hidden-Information Move**
1. Player takes an action that depends on hidden tiles.
2. Proof asserts legality (e.g., no wall-clipping), without revealing the hidden layout.
3. Observers learn *it was valid*, not *what was hidden*.

**Single-Player Record**
1. Local run generates periodic checkpoints + proofs.
2. On publish, chain verifies a batch proof; leaderboard / achievements update **only** if valid.

---

## 9) The Long View

If every action is provable and every world deterministic, multiple players — or multiple clients — can share truth without sharing trust:
- **Player-run shards** where moderation is mathematical.
- **Persistent histories** with cryptographic audit trails.
- **Interoperable content** that cannot cheat the engine because **validity is a proof, not a promise**.

Dungeon starts as a single-player roguelike because the genre rewards systemic depth. Its architecture, however, points toward **trust-minimized shared worlds**.

---

> **Dungeon believes that games should be both wondrous and honest — full of secrets, but never lies.**
