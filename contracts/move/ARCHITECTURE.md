# zkDungeon Move Contracts Architecture

## Overview

This document describes the Sui Move smart contract architecture for zkDungeon, a ZK-proof verified roguelike game that demonstrates **fairness without authority** and **secrecy without deceit**.

## Core Principles

1. **ZK Proof for State Transitions**: Every game state update is verified on-chain via Groth16 SNARKs
2. **Modular Challenge System**: Optional action logs enable competitive play and AI behavior verification
3. **Progressive Proof Submission**: Players choose proof frequency (every turn, every 100 turns, etc.)
4. **Content-Addressed Oracle**: Deterministic game content via Merkle commitments
5. **Off-chain Storage**: Full data stored in Walrus, only commitments on-chain

## Module Architecture

```
dungeon/
├── proof_verifier.move   # Groth16 verification wrapper
├── game_session.move     # Core game state management
└── action_log.move       # Optional challenge/replay data
```

### Module Dependencies

```
action_log
    ↓
game_session
    ↓
proof_verifier
    ↓
sui::groth16
```

## Module Details

### 1. proof_verifier.move

**Purpose**: Wraps Sui's native Groth16 verifier for game-specific proof verification.

**Key Types**:
- `VerifyingKey`: Prepared verification key (from RISC0 Groth16 trusted setup)
- `PublicInputs`: Public values committed in ZK proof (8 field elements × 32 bytes)

**Public Inputs Schema** (must match RISC0 guest program output):
1. `oracle_root` (32 bytes) - Game content commitment
2. `seed_commitment` (32 bytes) - RNG seed hash
3. `prev_state_root` (32 bytes) - State before actions
4. `prev_actions_root` (32 bytes) - Actions before this batch
5. `prev_nonce` (32 bytes, u64 padded) - Nonce before actions
6. `new_state_root` (32 bytes) - State after actions
7. `new_actions_root` (32 bytes) - Actions after this batch
8. `new_nonce` (32 bytes, u64 padded) - Nonce after actions

**Critical Security Feature**: `actions_root` is included in public inputs to cryptographically bind actions to state transitions. Without this, a player could:
1. Play with manipulated AI (cheating)
2. Generate valid ZK proof with real actions
3. Submit fake "legitimate" actions to ActionLog
4. Pass challenge system verification

**Functions**:
- `create_verifying_key()` - Initialize VK from RISC0 output
- `verify_game_proof()` - Verify Groth16 proof against public inputs
- `new_public_inputs()` - Construct PublicInputs from components

---

### 2. game_session.move

**Purpose**: Core game state management with progressive ZK proof verification.

**Key Type**:
```move
public struct GameSession has key, store {
    id: UID,
    player: address,

    // Immutable context (set at creation)
    oracle_root: vector<u8>,
    initial_state_root: vector<u8>,
    seed_commitment: vector<u8>,

    // Mutable state (updated by proofs)
    state_root: vector<u8>,
    actions_root: vector<u8>,
    nonce: u64,

    finalized: bool,
}
```

**Design Decisions**:
- **No timestamps in struct**: Stored in events only (reduces storage costs, avoids non-deterministic ZK proof data)
- **Nonce instead of turn_count**: Matches codebase terminology
- **vector<u8> for roots**: Move doesn't support fixed-size arrays as struct fields
- **Owned object**: Players own their sessions, can transfer/delete them

**Functions**:
- `create()` - Start new game session with commitments
- `update()` - Update state with ZK proof verification
- `finalize()` - Mark session complete (prevents further updates)
- `delete()` - Remove finalized session (storage rebate)

**Update Flow**:
1. Player calls `update(session, vk, proof, new_state_root, new_actions_root, new_nonce)`
2. Validates ownership and finalization status
3. Constructs `PublicInputs` from current + new state
4. Calls `proof_verifier::verify_game_proof()` (aborts if invalid)
5. Updates session state
6. Emits `SessionUpdatedEvent`

---

### 3. action_log.move

**Purpose**: Optional module for storing action replay data, enabling competitive play and challenge verification.

**Key Type**:
```move
public struct ActionLog has key, store {
    id: UID,
    session_id: address,          // Reference to GameSession
    player: address,
    actions_blob_id: vector<u8>,  // Walrus blob reference
    actions_root: vector<u8>,     // Must match GameSession.actions_root
    finalized: bool,
}
```

**Design Rationale**:
- **Separated from GameSession**: Modular design allows:
  - Private sessions (no ActionLog) - casual play
  - Public sessions (with ActionLog) - competitive/verifiable play
  - Independent module upgrades
  - Gas optimization (players who don't care about challenges don't pay for unused fields)

- **1:1 Relationship**: One ActionLog per GameSession (enforced by session_id reference)

- **Actions Root Validation**: `publish()` and `update()` verify that `actions_root` matches the session's `actions_root` (which is ZK-verified)

**Functions**:
- `publish()` - Create ActionLog linked to GameSession
- `update()` - Update with new Walrus blob reference
- `finalize()` - Mark log complete
- `delete()` - Remove finalized log

**Validation Flow**:
1. Player calls `publish(session, actions_blob_id, actions_root)`
2. Validates ownership and session not finalized
3. **Critical**: Validates `actions_root == session.actions_root`
4. Creates ActionLog with Walrus blob reference
5. Emits `ActionLogPublishedEvent`

**Why Separate from GameSession?**:
- **Modularity**: Challenge system is optional, not core to state verification
- **Upgradeability**: Challenge mechanics can evolve independently
- **Privacy**: Players can choose whether to publish actions
- **Gas Efficiency**: Non-competitive players don't pay for challenge infrastructure

---

## Data Flow Architecture

```
Off-chain (Client)                   On-chain (Sui)
─────────────────                   ────────────────

Game Engine
  ↓
Execute Actions
  ↓
State Transitions
  ↓
Batch Actions
  ↓
Generate ZK Proof ──────────────→ GameSession.update()
(RISC0 STARK → Groth16)              ↓
                                  Verify Proof
                                     ↓
                                  Update State

Upload to Walrus ───────────────→ ActionLog.publish()
(Full action data)                   ↓
                                  Validate actions_root
                                     ↓
                                  Store blob reference
```

## Security Properties

### 1. State Transition Integrity
- **Property**: Every state update is mathematically proven valid
- **Enforcement**: Groth16 verification in `GameSession.update()`
- **Guarantees**: No invalid moves, damage calculations, or rule violations

### 2. Action-State Binding
- **Property**: Actions logged in ActionLog correspond to actual gameplay
- **Enforcement**: `actions_root` in ZK proof public inputs
- **Attack Prevention**: Cannot submit fake "legitimate" actions for challenges

### 3. RNG Fairness
- **Property**: Random outcomes cannot be predicted or manipulated
- **Enforcement**: `seed_commitment` committed before game start
- **Guarantees**: Seed reveal verifies pre-commitment

### 4. Oracle Integrity
- **Property**: Game content cannot change mid-game
- **Enforcement**: `oracle_root` immutable in GameSession
- **Guarantees**: Deterministic replay, content-addressed data

### 5. Challenge System Integrity
- **Property**: AI behavior can be verified without revealing hidden information
- **Enforcement**: Full action log in Walrus + actions_root verification
- **Use Case**: Detect if player manipulated AI to play poorly

## Challenge System Design (Future)

The separated ActionLog module enables a future challenge system:

### Challenge Flow
1. **Session Completion**: Player finalizes GameSession + ActionLog
2. **Challenge Submission**: Challenger posts bond, references ActionLog
3. **Data Retrieval**: Download full actions from Walrus via `actions_blob_id`
4. **Verification**:
   - Compute actions root from downloaded data
   - Compare with ActionLog.actions_root (ZK-verified)
   - Replay actions, check AI behavior validity
5. **Resolution**:
   - If actions valid: Challenger loses bond
   - If actions invalid: Player penalized, challenger rewarded

### Why Actions Root is Critical
Without `actions_root` in ZK proof, player could:
1. Play with modified AI (e.g., enemies always miss)
2. Generate valid proof (state transition is technically correct)
3. Upload fake "legitimate" actions to Walrus
4. Pass challenge verification (fake actions appear valid)

**With** `actions_root` in proof:
1. ZK proof commits to specific action sequence
2. ActionLog must reference same action sequence
3. Any deviation detected during challenge verification

## Storage Strategy

### On-chain (Sui)
- **GameSession**: State commitments only (oracle_root, state_root, actions_root, seed_commitment)
- **ActionLog**: Walrus blob reference only (actions_blob_id)
- **Size**: ~200 bytes per session + ~100 bytes per log
- **Cost**: Minimal storage fees

### Off-chain (Walrus)
- **Oracle Data**: Maps, items, NPCs, loot tables (JSON/CBOR)
- **Full Actions**: Complete action sequence (JSON/CBOR)
- **Full State** (optional): Checkpoints for faster replay
- **Size**: Megabytes of rich game data
- **Cost**: Decentralized storage fees

### Why This Split?
- **Verification needs commitments**: Merkle roots are sufficient for ZK proofs
- **Challenges need full data**: Action replay requires complete action sequence
- **Gas optimization**: Only pay for on-chain commitments, not full data
- **Content addressing**: Walrus retrieval via blob ID, integrity verified via roots

## Proof Generation Pipeline (Future Implementation)

```
Rust (game-core)                 RISC0 zkVM              Sui Contract
────────────────                ───────────              ────────────

Execute actions
  ↓
Compute state delta
  ↓
Serialize inputs ─────────────→ Guest program
                                   ↓
                               Verify transitions
                                   ↓
                               Compute new roots
                                   ↓
                               Output PublicInputs ───→ GameSession.update()
                                   ↓                      ↓
                               Generate STARK          Verify Groth16
                                   ↓                      ↓
                               STARK → Groth16         Update state
                                (~200KB → ~200 bytes)
```

**RISC0 Guest Program** (to be implemented in `crates/zk/guest/`):
1. Read oracle_root, seed_commitment from public inputs
2. Deserialize initial state + action sequence
3. Execute actions using game-core engine
4. Verify state transition correctness
5. Compute new state_root, actions_root, nonce
6. Commit PublicInputs (8 field elements)
7. Generate STARK proof
8. Wrapper converts STARK → Groth16 (via `risc0_groth16::stark_to_snark`)

## Development Roadmap

### Phase 1: Contract Foundation ✅
- [x] proof_verifier module with Groth16 wrapper
- [x] game_session module with state management
- [x] action_log module with Walrus integration
- [x] PublicInputs schema with actions_root

### Phase 2: ZK Proof Pipeline (Next)
- [ ] RISC0 guest program matching PublicInputs schema
- [ ] Proof generation in ProverWorker (runtime)
- [ ] Groth16 wrapper integration (stark_to_snark)
- [ ] End-to-end test: client → proof → Sui verification

### Phase 3: Walrus Integration
- [ ] Upload actions to Walrus after execution
- [ ] Store blob IDs in ActionLog
- [ ] Download and verify action data

### Phase 4: Challenge System
- [ ] Challenge contract module
- [ ] Bond/penalty mechanism
- [ ] AI behavior verification logic
- [ ] Dispute resolution

### Phase 5: Production Features
- [ ] Oracle registry for content versions
- [ ] Leaderboard contracts
- [ ] NFT rewards for achievements
- [ ] Gas optimization and security audit

## Testing Strategy

### Unit Tests (Not Implemented)
Per CLAUDE.md policy, no small unit tests in committed code.

### Integration Tests (Future)
To be added in `contracts/move/tests/`:
- `test_session_lifecycle()` - Create, update, finalize, delete
- `test_proof_verification()` - Valid/invalid proof handling
- `test_action_log_validation()` - Actions root verification
- `test_ownership_enforcement()` - Access control
- `test_challenge_workflow()` - End-to-end challenge

### Local Testing
```bash
# Build contracts
sui move build

# Run tests (when implemented)
sui move test

# Deploy to local network
sui client publish --gas-budget 100000000
```

## Gas Optimization Notes

1. **No timestamps in structs**: Reduces storage by 16 bytes per session
2. **u64 for nonce**: Cheaper than u256 (8 bytes vs 32 bytes on-chain)
3. **vector<u8> for roots**: Move's native dynamic type, no overhead
4. **Separate ActionLog**: Optional feature, players don't pay if not used
5. **Event-based indexing**: Off-chain indexers can track history without on-chain queries

## Security Considerations

1. **Proof verification is atomic**: Update only happens if proof is valid
2. **Ownership strictly enforced**: Only session owner can update/finalize/delete
3. **Finalization is one-way**: Cannot unfinalize, prevents replay attacks
4. **Actions root prevents spoofing**: Cryptographically bound to state transition
5. **Oracle immutability**: Cannot change game content mid-session

## Known Limitations

1. **Vector<u8> for roots**: Move doesn't support fixed-size arrays in structs (ergonomics limitation, no security impact)
2. **No on-chain RNG reveal verification**: Planned for Phase 4
3. **No automated challenge resolution**: Manual verification for now
4. **Groth16 trusted setup**: Inherits RISC0's setup (industry standard, acceptable risk)

## References

- [Sui Move Documentation](https://docs.sui.io/concepts/sui-move-concepts)
- [Sui Groth16 Verifier](https://docs.sui.io/standards/cryptography/groth16)
- [RISC0 Documentation](https://dev.risczero.com/api)
- [Walrus Storage](https://docs.walrus.site/)
- [zkDungeon Codebase](../../../CLAUDE.md)
