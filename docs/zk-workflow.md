# ZK Proof Generation and On-Chain Verification Workflow

Complete end-to-end flow for playing the game, generating ZK proofs, uploading to Walrus, and verifying on Sui blockchain.

---

## Overview

```
Player (Off-chain)                Walrus (Storage)              Sui Blockchain (On-chain)
     │                                  │                              │
     ├─ Play Game                       │                              │
     ├─ Collect Actions                 │                              │
     ├─ Generate ZK Proof ──────────────┤                              │
     │   (computes actions_root)        │                              │
     ├─ Upload Actions ─────────────────►                              │
     │   (get Blob object)               │                              │
     └─ Submit Proof ───────────────────┴──────────────────────────────►
         (verify + store)
```

---

## Phase 1: Game Setup (One-time)

### 1.1 Deploy Contracts

```bash
# Deploy Sui Move contracts
cd contracts/move
sui move build
sui client publish --gas-budget 100000000
```

**Deployed Objects:**
- `dungeon` package
- Modules: `proof_verifier`, `game_session`

### 1.2 Initialize Verifying Key (Shared Object)

```bash
# Generate verifying key from RISC0 (off-chain)
cargo build --release --features risc0
./target/release/generate-vk > verifying_key.bin

# Deploy verifying key on-chain (creates shared object)
sui client call \
  --package $PACKAGE_ID \
  --module proof_verifier \
  --function create_verifying_key \
  --args \
    "$(cat verifying_key.bin | base64)" \
    1 \
  --gas-budget 10000000
```

**Result:**
- `VerifyingKey` shared object created
- All players can reference this shared object for verification
- `vk_id = 0xVERIFYING_KEY_ID` (save this!)

### 1.3 Create Oracle Snapshot (Optional)

If using static game content (maps, items, NPCs):

```rust
// Off-chain: Create oracle snapshot
let oracle_snapshot = OracleSnapshot::from_content(&game_content);
let oracle_root = compute_oracle_root(&oracle_snapshot);

// Store oracle_root for session creation
// Can optionally upload oracle_snapshot to Walrus for reproducibility
```

---

## Phase 2: Start Game Session

### 2.1 Compute Initial Commitments

```rust
// Off-chain: Before starting game
let oracle_root = compute_oracle_root(&oracle_snapshot);       // 32 bytes
let initial_state = GameState::new(...);
let initial_state_root = compute_state_root(&initial_state);   // 32 bytes
let seed = generate_random_seed();
let seed_commitment = compute_seed_commitment(&seed);          // 32 bytes
```

### 2.2 Create Game Session (On-chain)

```bash
sui client call \
  --package $PACKAGE_ID \
  --module game_session \
  --function create \
  --args \
    "$(echo $oracle_root | xxd -r -p | base64)" \
    "$(echo $initial_state_root | xxd -r -p | base64)" \
    "$(echo $seed_commitment | xxd -r -p | base64)" \
  --gas-budget 10000000
```

**Result:**
- `GameSession` object created (owned by player)
- `session_id = 0xSESSION_ID` (save this!)
- Initial state: `nonce = 0`, `state_root = initial_state_root`

**Event emitted:**
```move
SessionStartedEvent {
    session_id: 0xSESSION_ID,
    player: 0xPLAYER_ADDRESS,
    oracle_root: [32 bytes],
    started_at: epoch,
}
```

---

## Phase 3: Play Game (Off-chain)

### 3.1 Run Game Locally

```bash
# Start game client with runtime
cargo run --package client-cli --features stub

# Or for production with RISC0
cargo run --package client-cli --features risc0
```

**Game Runtime:**
```rust
// Runtime executes game loop
loop {
    // 1. Get player input or AI action
    let action = get_next_action(&state, &env);

    // 2. Execute action through GameEngine
    let mut engine = GameEngine::new(&mut state);
    engine.execute(env, &action)?;

    // 3. Store action in action log
    action_log.push(action);

    // 4. Update state
    state.turn.clock += 1;

    // Check for game end conditions
    if state.is_game_over() {
        break;
    }
}
```

**Action Log Collection:**
```rust
// Collect all actions executed during gameplay
let actions: Vec<Action> = vec![
    Action::Character(CharacterActionKind::Move { target: (5, 5) }),
    Action::Character(CharacterActionKind::Attack { target_id: 123 }),
    Action::Character(CharacterActionKind::Wait),
    // ... hundreds or thousands of actions
];

let prev_state_root = initial_state_root;  // or last checkpoint
let new_state_root = compute_state_root(&final_state);
let new_nonce = final_state.turn.clock;
```

---

## Phase 4: Generate ZK Proof

### 4.1 Serialize Actions

```rust
// Serialize actions to bytes
let actions_bytes = bincode::serialize(&actions)?;

// Compute actions_root (must match Walrus blob_id computation)
let actions_root = compute_actions_root(&actions);  // 32 bytes
// Implementation: sha256(actions_bytes) or blake3(actions_bytes)
```

### 4.2 Run zkVM to Generate Proof

```rust
// Off-chain: Host calls prover
let prover = Risc0Prover::new(oracle_snapshot);

let proof_data = prover.prove_batch(
    &prev_state,
    &actions,
    oracle_root,
    seed_commitment,
)?;

// proof_data contains:
// - Groth16 proof bytes
// - Public inputs extracted from journal
```

**zkVM Guest Program Execution:**

```rust
// Inside RISC0 zkVM (crates/zk/methods/guest/src/main.rs)
risc0_zkvm::guest::entry!(main);

pub fn main() {
    // 1. Read private inputs
    let oracle_snapshot: OracleSnapshot = env::read();
    let mut state: GameState = env::read();
    let actions: Vec<Action> = env::read();

    // 2. Compute public inputs (commitments)
    let oracle_root = compute_oracle_root(&oracle_snapshot);
    let seed_commitment = compute_seed_commitment();
    let prev_state_root = compute_state_root(&state);
    let actions_root = compute_actions_root(&actions);  // ← 핵심!

    // 3. Execute all actions deterministically
    let oracle_bundle = SnapshotOracleBundle::new(&oracle_snapshot);
    let mut engine = GameEngine::new(&mut state);

    for action in actions {
        engine.execute(oracle_bundle.as_env().into_game_env(), &action)
            .expect("Action execution failed in zkVM");
    }

    // 4. Compute final state
    let new_state_root = compute_state_root(&state);
    let new_nonce = state.turn.clock;  // or explicit nonce field

    // 5. Commit public outputs to journal (will become Groth16 public inputs)
    env::commit(&oracle_root);
    env::commit(&seed_commitment);
    env::commit(&prev_state_root);
    env::commit(&actions_root);    // ← This will be compared with blob_id!
    env::commit(&new_state_root);
    env::commit(&new_nonce);
}
```

**Performance:**
- Stub backend: ~1ms (no real proof)
- RISC0 dev mode: ~100ms (fast proof)
- RISC0 production: ~30-60s (Groth16 proof with Metal acceleration)

### 4.3 Extract Proof and Public Inputs

```rust
// Host extracts from zkVM receipt
let receipt = prove_result.receipt;

// Decode public inputs from journal (in order)
let mut journal_decoder = receipt.journal.bytes.as_slice();
let oracle_root: [u8; 32] = bincode::deserialize_from(&mut journal_decoder)?;
let seed_commitment: [u8; 32] = bincode::deserialize_from(&mut journal_decoder)?;
let prev_state_root: [u8; 32] = bincode::deserialize_from(&mut journal_decoder)?;
let actions_root: [u8; 32] = bincode::deserialize_from(&mut journal_decoder)?;
let new_state_root: [u8; 32] = bincode::deserialize_from(&mut journal_decoder)?;
let new_nonce: u64 = bincode::deserialize_from(&mut journal_decoder)?;

// Serialize Groth16 proof
let groth16_proof_bytes = extract_groth16_proof(&receipt)?;
```

---

## Phase 5: Upload Actions to Walrus

### 5.1 Upload to Walrus Storage

**Parallel to zkVM execution** (can happen before, during, or after proof generation):

```bash
# Upload actions blob to Walrus
walrus store actions.bin \
  --send-object-to $PLAYER_ADDRESS \
  --epochs 10 \
  --gas-budget 50000000

# Or via HTTP API
curl -X PUT \
  "$WALRUS_PUBLISHER/v1/blobs?send_object_to=$PLAYER_ADDRESS&epochs=10" \
  --upload-file actions.bin
```

**Response:**
```json
{
  "newlyCreated": {
    "blobObject": {
      "id": "0xBLOB_OBJECT_ID",
      "storedEpoch": 100,
      "blobId": "0x456def...",  // ← Content hash (u256)
      "size": 123456,
      "erasureCodeType": "RedStuff",
      "certifiedEpoch": 100
    },
    "resourceOperation": {...},
    "cost": 1000
  }
}
```

### 5.2 Verify Actions Root Matches Blob ID

```rust
// Off-chain verification before submitting to blockchain
let blob_id: [u8; 32] = response.blob_object.blob_id.to_bytes_be();  // u256 → 32 bytes
let actions_root_from_zkvm: [u8; 32] = proof_data.public_inputs.actions_root;

assert_eq!(
    blob_id,
    actions_root_from_zkvm,
    "blob_id must match zkVM computed actions_root!"
);
// ↑ This ensures the actions uploaded to Walrus are the same as actions proven by zkVM
```

**Critical Security Check:**
- zkVM: `actions_root = hash(actions)` (cryptographically committed in proof)
- Walrus: `blob_id = hash(blob_content)` (content-addressed storage)
- Contract will verify: `blob_id == actions_root`
- → Impossible for player to submit different actions to Walrus vs zkVM!

---

## Phase 6: Submit Proof to Blockchain

### 6.1 Construct Transaction

```rust
// Build Sui transaction
let tx = TransactionBuilder::new()
    .move_call(
        package_id,
        "game_session",
        "update",
        vec![],  // type args
        vec![
            SuiJsonValue::from_object_id(session_id),
            SuiJsonValue::from_object_id(vk_shared_object_id),  // Shared object!
            SuiJsonValue::from_str(&hex::encode(&groth16_proof_bytes))?,
            SuiJsonValue::from_str(&hex::encode(&new_state_root))?,
            SuiJsonValue::from_str(&new_nonce.to_string())?,
            SuiJsonValue::from_object_id(blob_object_id),  // Transfer Blob ownership
        ],
    )
    .build();
```

### 6.2 Execute Transaction

```bash
sui client call \
  --package $PACKAGE_ID \
  --module game_session \
  --function update \
  --args \
    $SESSION_ID \
    $VK_SHARED_OBJECT_ID \
    "$(echo $PROOF_BYTES | base64)" \
    "$(echo $NEW_STATE_ROOT | xxd -r -p | base64)" \
    $NEW_NONCE \
    $BLOB_OBJECT_ID \
  --gas-budget 100000000
```

### 6.3 On-Chain Verification

**Contract Execution Flow:**

```move
// game_session.move::update()
public fun update(
    session: &mut GameSession,
    vk: &VerifyingKey,  // Shared object (immutable reference)
    proof: vector<u8>,
    new_state_root: vector<u8>,
    new_nonce: u64,
    actions_blob: Blob,  // Ownership transferred from player
    ctx: &mut TxContext,
) {
    // 1. Check ownership
    assert!(tx_context::sender(ctx) == session.player, ENotOwner);

    // 2. Extract blob_id from Walrus Blob object
    let blob_id: u256 = walrus::blob::blob_id(&actions_blob);
    let actions_root = u256_to_bytes(blob_id);  // Convert u256 → 32 bytes (big-endian)

    // 3. Construct public inputs for proof verification
    let public_inputs = proof_verifier::new_public_inputs(
        session.oracle_root,
        session.seed_commitment,
        session.state_root,        // prev_state_root
        actions_root,              // ← Walrus blob_id
        new_state_root,
        new_nonce,
    );

    // 4. Verify Groth16 proof
    proof_verifier::verify_game_proof(vk, &public_inputs, proof);
    // ↑ This call:
    //   - Serializes public_inputs to BN254 field elements
    //   - Calls sui::groth16::verify_groth16_proof()
    //   - Aborts if proof is invalid
    //   - Verifies that zkVM computed actions_root matches blob_id

    // 5. Store action log blob as child object
    let action_log = ActionLogBlob {
        id: object::new(ctx),
        blob: actions_blob,  // Blob object stored here
        submitted_at: tx_context::epoch(ctx),
        start_state_root: session.state_root,
    };

    dof::add(&mut session.id, new_nonce, action_log);

    // 6. Update session state
    session.state_root = new_state_root;
    session.nonce = new_nonce;
    session.pending_action_logs = session.pending_action_logs + 1;

    // 7. Unfinalize if previously finalized
    if (session.finalized) {
        session.finalized = false;
    };

    // 8. Emit events
    event::emit(ActionLogPublishedEvent {
        session_id: session_id(session),
        actions_blob_id: blob_id,
        nonce: new_nonce,
        published_at: tx_context::epoch(ctx),
    });

    event::emit(SessionUpdatedEvent {
        session_id: session_id(session),
        new_state_root,
        nonce: new_nonce,
        updated_at: tx_context::epoch(ctx),
    });
}
```

**Verification Steps Inside `proof_verifier::verify_game_proof()`:**

```move
// proof_verifier.move::verify_game_proof()
public fun verify_game_proof(
    vk: &VerifyingKey,
    public_inputs: &PublicInputs,
    proof_bytes: vector<u8>,
) {
    // 1. Serialize public inputs to BN254 field elements (6 × 32 bytes)
    let public_inputs_bytes = serialize_public_inputs(public_inputs);
    //   Order: oracle_root, seed_commitment, prev_state_root,
    //          actions_root, new_state_root, new_nonce (u64 → 32 bytes)

    // 2. Create Groth16 proof points from bytes
    let curve = groth16::bn254();
    let proof_points = groth16::proof_points_from_bytes(proof_bytes);
    let public_proof_inputs = groth16::public_proof_inputs_from_bytes(public_inputs_bytes);

    // 3. Verify Groth16 proof
    let valid = groth16::verify_groth16_proof(
        &curve,
        &vk.prepared_vk,
        &public_proof_inputs,
        &proof_points,
    );

    // 4. Abort if invalid
    assert!(valid, EInvalidProof);
}
```

**Transaction Result:**
- ✅ Proof verified successfully
- ✅ `actions_root` (from zkVM) == `blob_id` (from Walrus)
- ✅ State transition recorded on-chain
- ✅ Action log stored as Dynamic Object Field
- ✅ Events emitted for indexers/explorers

---

## Phase 7: Challenge Period (Optional)

### 7.1 Challengers Verify Actions

Anyone can challenge the gameplay during the challenge period:

```rust
// Off-chain: Download actions from Walrus
let actions_bytes = walrus_client.read(blob_id)?;
let actions: Vec<Action> = bincode::deserialize(&actions_bytes)?;

// Verify by re-executing locally
let mut local_state = initial_state.clone();
let oracle_bundle = get_oracle_snapshot(oracle_root);

for action in &actions {
    local_state.execute(oracle_bundle, action)?;
}

// Compare final state
let computed_state_root = compute_state_root(&local_state);
if computed_state_root != new_state_root {
    // Submit fraud proof or dispute!
    panic!("Fraud detected!");
}
```

### 7.2 Challenge Period Duration

```move
const CHALLENGE_PERIOD_EPOCHS: u64 = 7 * 24 * 60; // ~7 days
```

During this period:
- Action logs are stored on-chain (as Dynamic Object Fields)
- Anyone can download and verify actions from Walrus
- Disputes can be raised (future: challenge mechanism)

---

## Phase 8: Cleanup and Finalization

### 8.1 Remove Expired Action Logs

After challenge period expires, clean up storage:

```bash
# Remove expired action logs (returns Blob objects)
sui client call \
  --package $PACKAGE_ID \
  --module game_session \
  --function remove_expired_action_logs \
  --args \
    $SESSION_ID \
    "[100, 101, 102]" \  # nonces to remove (JSON array)
  --gas-budget 10000000
```

**On-chain:**
```move
public fun remove_expired_action_logs(
    session: &mut GameSession,
    nonces: vector<u64>,
    ctx: &TxContext,
): vector<Blob> {
    // Check each action log has expired
    let current_epoch = tx_context::epoch(ctx);

    for each nonce {
        let action_log = dof::borrow<u64, ActionLogBlob>(&session.id, nonce);
        let challenge_expiry = action_log.submitted_at + CHALLENGE_PERIOD_EPOCHS;
        assert!(current_epoch >= challenge_expiry, EChallengeNotExpired);

        // Remove and unwrap
        let action_log = dof::remove<u64, ActionLogBlob>(&mut session.id, nonce);
        let blob = action_log.blob;
        // ... return blob
    }

    session.pending_action_logs = session.pending_action_logs - len;
}
```

**Result:**
- Blob objects returned to caller
- Storage freed (storage rebate)
- `pending_action_logs` counter decremented

### 8.2 Finalize Session

When game is complete and all action logs are cleaned:

```bash
sui client call \
  --package $PACKAGE_ID \
  --module game_session \
  --function finalize \
  --args $SESSION_ID \
  --gas-budget 1000000
```

**On-chain:**
```move
public fun finalize(
    session: &mut GameSession,
    ctx: &TxContext,
) {
    assert!(tx_context::sender(ctx) == session.player, ENotOwner);
    assert!(session.pending_action_logs == 0, EActionLogsRemaining);

    session.finalized = true;

    event::emit(SessionFinalizedEvent {
        session_id: session_id(session),
        final_state_root: session.state_root,
        final_nonce: session.nonce,
        finalized_at: tx_context::epoch(ctx),
    });
}
```

### 8.3 Delete Session (Optional)

```bash
sui client call \
  --package $PACKAGE_ID \
  --module game_session \
  --function delete \
  --args $SESSION_ID \
  --gas-budget 1000000
```

Removes session from blockchain (storage rebate).

---

## Data Flow Summary

### Off-chain → zkVM → On-chain

```
┌─────────────────────────────────────────────────────────────────┐
│ Off-chain (Player)                                              │
├─────────────────────────────────────────────────────────────────┤
│ 1. Play game locally                                            │
│    → Collect actions: Vec<Action>                               │
│    → prev_state, final_state                                    │
│                                                                 │
│ 2. Serialize actions → actions_bytes                            │
│                                                                 │
│ 3. Upload to Walrus                                             │
│    → walrus store actions_bytes --send-object-to $PLAYER       │
│    → Get Blob object (id, blob_id)                             │
│                                                                 │
│ 4. Generate ZK proof (parallel to step 3)                      │
│    → zkVM computes actions_root = hash(actions)                │
│    → zkVM executes actions deterministically                    │
│    → zkVM outputs: (oracle_root, seed_commitment,              │
│                     prev_state_root, actions_root,              │
│                     new_state_root, new_nonce)                  │
│    → Groth16 proof generated                                   │
│                                                                 │
│ 5. Verify: blob_id == actions_root                             │
│    → Ensures consistency between Walrus and zkVM               │
│                                                                 │
│ 6. Submit transaction to Sui                                    │
│    → game_session::update(session, vk, proof, ...)            │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│ On-chain (Sui Move Contract)                                   │
├─────────────────────────────────────────────────────────────────┤
│ 1. Receive: Blob object, proof, new_state_root, new_nonce     │
│                                                                 │
│ 2. Extract blob_id from Blob object                            │
│    → blob_id = walrus::blob::blob_id(&actions_blob)           │
│                                                                 │
│ 3. Construct public inputs                                      │
│    → (oracle_root, seed_commitment, prev_state_root,           │
│       actions_root = blob_id, new_state_root, new_nonce)       │
│                                                                 │
│ 4. Verify Groth16 proof                                        │
│    → proof_verifier::verify_game_proof(vk, public_inputs, proof)│
│    → Uses shared VerifyingKey object                           │
│    → Aborts if invalid                                          │
│                                                                 │
│ 5. Store Blob as ActionLogBlob (child object)                  │
│    → dof::add(&mut session.id, new_nonce, action_log)         │
│                                                                 │
│ 6. Update session state                                         │
│    → session.state_root = new_state_root                       │
│    → session.nonce = new_nonce                                  │
│                                                                 │
│ 7. Emit events                                                  │
│    → ActionLogPublishedEvent, SessionUpdatedEvent              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Cryptographic Guarantees

### 1. State Transition Validity
- **zkVM proves**: Executing `actions` on `prev_state` produces `new_state`
- **Groth16 proof**: Cryptographically binds all public inputs
- **Contract verifies**: Proof is valid using trusted verifying key

### 2. Actions Integrity
- **zkVM computes**: `actions_root = hash(actions)`
- **Walrus computes**: `blob_id = hash(blob_content)`
- **Contract checks**: `blob_id == actions_root`
- **Result**: Impossible to submit different actions to Walrus vs zkVM

### 3. Oracle Consistency
- **oracle_root**: Commitment to static game content
- **zkVM verifies**: Oracle snapshot hash matches oracle_root
- **Result**: Game rules are deterministic and reproducible

### 4. RNG Fairness
- **seed_commitment**: Commitment to RNG seed at session start
- **zkVM uses**: Seed for all random number generation
- **Result**: Player cannot cherry-pick favorable outcomes

### 5. Challenge Period Security
- **Action logs on-chain**: Stored for challenge period
- **Walrus availability**: Anyone can download and verify
- **Fraud proof**: If state is incorrect, challengers can prove it
- **Economic security**: Storage costs incentivize cleanup after verification

---

## Performance Characteristics

| Operation | Time | Gas Cost | Notes |
|-----------|------|----------|-------|
| Play game (local) | Real-time | 0 | Off-chain, no blockchain interaction |
| Upload to Walrus | 1-5s | ~0.01 SUI | Per blob, depends on size |
| Generate proof (stub) | 1ms | 0 | Testing only, no real proof |
| Generate proof (RISC0 dev) | 100ms | 0 | Fast mode, weak security |
| Generate proof (RISC0 prod) | 30-60s | 0 | Groth16, production security |
| Submit proof on-chain | 3-5s | ~0.5-1 SUI | Depends on proof size + gas price |
| Verify proof (contract) | <100ms | Included above | Groth16 verification on Sui |
| Challenge period | ~7 days | 0 | Storage cost during period |
| Cleanup action logs | 1-2s | ~0.01 SUI | Storage rebate received |

---

## Error Handling

### Off-chain Errors
- **zkVM execution failed**: Action violated game rules → Fix actions or game state
- **Proof generation failed**: zkVM panic or timeout → Check logs, retry
- **Walrus upload failed**: Network issue → Retry with exponential backoff
- **blob_id mismatch**: `blob_id != actions_root` → Critical bug, investigate!

### On-chain Errors
- **ENotOwner**: Caller is not session owner → Use correct wallet
- **EInvalidProof**: Proof verification failed → Regenerate proof with correct inputs
- **EActionLogNotFound**: Action log doesn't exist → Check nonce
- **EChallengeNotExpired**: Challenge period not over yet → Wait longer
- **EActionLogsRemaining**: Can't finalize with pending logs → Clean up first

---

## Future Enhancements

### Batch Proofs
- Submit multiple game sessions in one proof (aggregation)
- Reduce per-session overhead

### Optimistic Verification
- Skip proof verification on-chain initially
- Submit proof only if challenged (optimistic rollup model)

### Compressed State Commitments
- Use Merkle tree for state_root
- Enable partial state proofs for challenges

### AI Challenge Mechanism
- Per-turn AI action verification
- Fraud proof for disputed AI behavior

### Reward Distribution
- Leaderboard contracts
- Token rewards for high scores
- NFT minting for achievements

---

## Security Considerations

### Trusted Setup
- **Verifying key**: Must come from trusted RISC0 setup
- **Shared object**: Single verifying key for all sessions
- **Upgrade path**: Deploy new verifying key if circuit changes

### Data Availability
- **Walrus**: Decentralized storage ensures actions are available
- **Challenge period**: Time window for verification
- **Storage incentives**: Economic model for data retention

### Economic Security
- **Storage costs**: Player pays for Walrus and on-chain storage
- **Gas costs**: Prevents spam attacks
- **Challenge bonds**: Future mechanism to prevent frivolous challenges

### Privacy Considerations
- **Public actions**: All actions are publicly visible (Walrus)
- **Hidden state**: Only state_root is public, not full state
- **Selective disclosure**: Future: ZK proofs for partial state reveal

---

## Conclusion

This workflow demonstrates a complete **trustless, verifiable gaming system** using:
- **RISC0 zkVM**: Proof of correct game execution
- **Walrus**: Decentralized data availability
- **Sui Move**: Efficient on-chain verification and state management

**Key Innovation**: Cryptographic binding of actions (Walrus blob_id) to state transitions (zkVM proof) ensures players cannot cheat while maintaining scalability and low on-chain costs.
