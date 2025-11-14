# ZK Proof Architecture - RISC0 Groth16 Integration

## Overview

This document describes the architecture for generating and verifying ZK proofs of game state transitions using RISC0 zkVM with Groth16 wrapper for on-chain verification on Sui blockchain.

## RISC0 Groth16 Architecture

### Proof Generation Flow

```
Guest Program (RISC0 zkVM)
    ↓
STARK Proof
    ↓
Groth16 Wrapper (stark_to_snark)
    ↓
Groth16 Proof + Journal Digest
    ↓
On-chain Verification (Sui)
```

### Key Constraints

**RISC0 Groth16 Wrapper Public Inputs:**
- Only **3 public inputs** in the Groth16 proof:
  1. `CONTROL_ROOT` - RISC0 control root (allows zkVM updates without new trusted setup)
  2. `CLAIM_DIGEST` - Hash of the execution claim
  3. `CONTROL_ID` - Control identifier

**Claim Digest Structure:**
```
CLAIM_DIGEST = hash(IMAGE_ID, JOURNAL_DIGEST)
JOURNAL_DIGEST = SHA256(journal_bytes)
```

**Important:** Individual journal fields are **not** directly accessible as public inputs in Groth16. Only the digest of the entire journal is available.

### Ethereum Verifier Reference

RISC0's Ethereum verifier signature (for reference):
```solidity
function verify(
    bytes calldata seal,        // Groth16 proof
    bytes32 imageId,            // Guest program identifier
    bytes32 journalDigest       // SHA-256(journal bytes)
) external view;
```

## Our Implementation Strategy

### Two-Stage Verification

Since Groth16 only provides `journalDigest` as a public input, we use a two-stage verification approach:

1. **Stage 1 - Groth16 Proof Verification:**
   - Verify the Groth16 proof is valid
   - Verify the `journalDigest` matches the proof
   - This proves: "Some valid execution produced this journal digest"

2. **Stage 2 - Journal Content Verification:**
   - Verify the provided journal data hashes to `journalDigest`
   - Extract and validate individual fields from journal
   - This proves: "The journal contains these specific values"

### Journal Structure

The guest program commits 6 fields to the journal in a specific order:

```rust
// In guest program (methods/state-transition/src/main.rs)
env::commit(&oracle_root);           // 32 bytes
env::commit(&seed_commitment);       // 32 bytes
env::commit(&prev_state_root);       // 32 bytes
env::commit(&actions_root);          // 32 bytes (Walrus blob_id or hash)
env::commit(&new_state_root);        // 32 bytes
env::commit(&new_nonce);             // 8 bytes (u64)
```

**Total journal size:** 168 bytes (5 × 32 + 8)

**Journal digest:** `SHA256(oracle_root || seed_commitment || prev_state_root || actions_root || new_state_root || new_nonce)`

## Sui Contract Architecture

### Current Contract (Needs Modification)

**File:** `contracts/move/sources/proof_verifier.move`

**Current structure (❌ Incompatible):**
```move
public struct PublicInputs has copy, drop, store {
    oracle_root: vector<u8>,        // 32 bytes
    seed_commitment: vector<u8>,    // 32 bytes
    prev_state_root: vector<u8>,    // 32 bytes
    actions_root: vector<u8>,       // 32 bytes
    new_state_root: vector<u8>,     // 32 bytes
    new_nonce: u64,                 // 8 bytes
}

public fun verify_game_proof(
    vk: &VerifyingKey,
    public_inputs: &PublicInputs,
    proof_bytes: vector<u8>,
)
```

This approach assumes 6 separate public inputs in Groth16, which is not how RISC0's wrapper works.

### Proposed Contract (✅ RISC0 Compatible)

```move
/// Journal data structure - matches guest program output
public struct JournalData has copy, drop, store {
    oracle_root: vector<u8>,        // 32 bytes
    seed_commitment: vector<u8>,    // 32 bytes
    prev_state_root: vector<u8>,    // 32 bytes
    actions_root: vector<u8>,       // 32 bytes (Walrus blob_id)
    new_state_root: vector<u8>,     // 32 bytes
    new_nonce: u64,                 // 8 bytes
}

/// Verify a game state transition proof with journal validation
///
/// Two-stage verification:
/// 1. Verify Groth16 proof with journal_digest as public input
/// 2. Verify journal_data hashes to journal_digest
public fun verify_game_proof(
    vk: &VerifyingKey,
    journal_digest: vector<u8>,     // 32 bytes - SHA256 of journal
    journal_data: &JournalData,     // Actual journal content
    proof_bytes: vector<u8>,
) {
    // Stage 1: Verify Groth16 proof
    let curve = groth16::bn254();
    let proof_points = groth16::proof_points_from_bytes(proof_bytes);

    // Public input is just the journal digest
    let public_inputs = groth16::public_proof_inputs_from_bytes(journal_digest);

    let valid = groth16::verify_groth16_proof(
        &curve,
        &vk.prepared_vk,
        &public_inputs,
        &proof_points,
    );
    assert!(valid, EInvalidProof);

    // Stage 2: Verify journal content matches digest
    let computed_digest = compute_journal_digest(journal_data);
    assert!(computed_digest == journal_digest, EJournalMismatch);
}

/// Compute SHA256 digest of journal data
///
/// Must match the serialization order in guest program
fun compute_journal_digest(journal: &JournalData): vector<u8> {
    let mut bytes = vector::empty<u8>();

    vector::append(&mut bytes, journal.oracle_root);
    vector::append(&mut bytes, journal.seed_commitment);
    vector::append(&mut bytes, journal.prev_state_root);
    vector::append(&mut bytes, journal.actions_root);
    vector::append(&mut bytes, journal.new_state_root);
    vector::append(&mut bytes, u64_to_bytes(journal.new_nonce));

    sui::hash::sha256(&bytes)
}
```

## Guest Program Implementation

### State Transition (Unified)

**File:** `crates/zk/methods/state-transition/src/main.rs`

Proves execution of actions (single or batch) in sequence:

```rust
use risc0_zkvm::guest::env;

fn main() {
    // Read inputs
    let oracle_snapshot: OracleSnapshot = env::read();
    let seed_commitment: [u8; 32] = env::read();
    let mut state: GameState = env::read();
    let actions: Vec<Action> = env::read();

    // Setup environment
    let oracle_bundle = SnapshotOracleBundle::new(&oracle_snapshot);
    let game_env = oracle_bundle.as_env().into_game_env();

    // Compute roots before execution
    let oracle_root = oracle_snapshot.compute_oracle_root();
    let prev_state_root = state.compute_state_root();

    // Compute actions root (Walrus blob_id simulation)
    let actions_root = compute_actions_root(&actions);

    // Execute all actions sequentially
    let mut engine = GameEngine::new(&mut state);
    for (index, action) in actions.iter().enumerate() {
        engine.execute(game_env, action)
            .unwrap_or_else(|e| {
                panic!("Action {}/{} failed: {:?}", index + 1, actions.len(), e)
            });
    }

    // Compute roots after execution
    let new_state_root = state.compute_state_root();
    let new_nonce = state.nonce();

    // Commit to journal (in order - must match contract)
    env::commit(&oracle_root);
    env::commit(&seed_commitment);
    env::commit(&prev_state_root);
    env::commit(&actions_root);
    env::commit(&new_state_root);
    env::commit(&new_nonce);
}
```

## Host-Side Implementation

### State Root Computation

**File:** `crates/game/core/src/state.rs`

```rust
impl GameState {
    /// Compute deterministic state root using SHA256
    ///
    /// This is optimized for zkVM - simple hash is most efficient.
    /// Merkle trees only provide benefits for custom circuits.
    pub fn compute_state_root(&self) -> [u8; 32] {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();

        // Hash all state components in deterministic order
        // TODO: Implement proper serialization
        hasher.update(&self.nonce().to_le_bytes());
        // ... hash entities, world, turn state ...

        hasher.finalize().into()
    }
}
```

### Oracle Root Computation

**File:** `crates/zk/src/oracle.rs`

```rust
impl OracleSnapshot {
    /// Compute deterministic oracle root using SHA256
    pub fn compute_oracle_root(&self) -> [u8; 32] {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();

        // Hash all oracle components in deterministic order
        hasher.update(&bincode::serialize(&self.map).unwrap());
        hasher.update(&bincode::serialize(&self.items).unwrap());
        hasher.update(&bincode::serialize(&self.actors).unwrap());
        hasher.update(&bincode::serialize(&self.tables).unwrap());
        hasher.update(&bincode::serialize(&self.config).unwrap());

        hasher.finalize().into()
    }
}
```

### Actions Root Computation

**File:** `crates/zk/src/lib.rs` or `crates/zk/src/helpers.rs`

```rust
/// Compute actions root - simulates Walrus blob_id
///
/// In production, this would be the actual Walrus blob_id.
/// For now, we use SHA256 hash as a commitment to the action sequence.
pub fn compute_actions_root(actions: &[Action]) -> [u8; 32] {
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();
    for action in actions {
        hasher.update(&bincode::serialize(action).unwrap());
    }

    hasher.finalize().into()
}
```

### Prover Implementation

**File:** `crates/zk/src/zkvm/risc0.rs`

```rust
impl Prover for Risc0Prover {
    fn prove(
        &self,
        oracle_snapshot: &OracleSnapshot,
        seed_commitment: [u8; 32],
        start_state: &GameState,
        actions: &[Action],
        expected_end_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        // Compute expected roots
        let oracle_root = oracle_snapshot.compute_oracle_root();
        let prev_state_root = start_state.compute_state_root();
        let actions_root = compute_actions_root(actions);
        let expected_new_state_root = expected_end_state.compute_state_root();
        let expected_new_nonce = expected_end_state.nonce();

        // Prepare inputs for guest
        let mut env = ExecutorEnv::builder();
        env.write(&oracle_snapshot)?;
        env.write(&seed_commitment)?;
        env.write(start_state)?;
        env.write(actions)?;
        let env = env.build()?;

        // Generate STARK proof
        let prover = default_prover();
        let prove_info = prover.prove(env, BATCH_STATE_TRANSITION_ELF)?;

        // Extract receipt and journal
        let receipt = prove_info.receipt;
        let journal = receipt.journal.bytes.clone();

        // Verify journal structure (6 fields)
        verify_journal_structure(&journal)?;

        // Parse journal and verify it matches expected values
        let (
            journal_oracle_root,
            journal_seed_commitment,
            journal_prev_state_root,
            journal_actions_root,
            journal_new_state_root,
            journal_new_nonce,
        ) = parse_journal(&journal)?;

        // Verify all journal values match expectations
        assert_eq!(journal_oracle_root, oracle_root, "Oracle root mismatch");
        assert_eq!(journal_seed_commitment, seed_commitment, "Seed commitment mismatch");
        assert_eq!(journal_prev_state_root, prev_state_root, "Prev state root mismatch");
        assert_eq!(journal_actions_root, actions_root, "Actions root mismatch");
        assert_eq!(journal_new_state_root, expected_new_state_root, "New state root mismatch");
        assert_eq!(journal_new_nonce, expected_new_nonce, "New nonce mismatch");

        // Compute journal digest
        let journal_digest = compute_journal_digest(&journal);

        // Convert to Groth16 (in production)
        // let groth16_proof = stark_to_snark(receipt)?;

        Ok(ProofData {
            proof: receipt.serialize()?,
            journal,
            journal_digest,
            public_inputs: vec![
                oracle_root.to_vec(),
                seed_commitment.to_vec(),
                prev_state_root.to_vec(),
                actions_root.to_vec(),
                journal_new_state_root.to_vec(),
                journal_new_nonce.to_le_bytes().to_vec(),
            ],
        })
    }
}

/// Parse journal bytes into individual fields
fn parse_journal(journal: &[u8]) -> Result<([u8; 32], [u8; 32], [u8; 32], [u8; 32], [u8; 32], u64), ProofError> {
    if journal.len() != 168 {
        return Err(ProofError::InvalidJournal(format!(
            "Expected 168 bytes, got {}",
            journal.len()
        )));
    }

    let oracle_root = journal[0..32].try_into().unwrap();
    let seed_commitment = journal[32..64].try_into().unwrap();
    let prev_state_root = journal[64..96].try_into().unwrap();
    let actions_root = journal[96..128].try_into().unwrap();
    let new_state_root = journal[128..160].try_into().unwrap();
    let new_nonce = u64::from_le_bytes(journal[160..168].try_into().unwrap());

    Ok((oracle_root, seed_commitment, prev_state_root, actions_root, new_state_root, new_nonce))
}

/// Compute SHA256 digest of journal
fn compute_journal_digest(journal: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    Sha256::digest(journal).into()
}
```

## ProofData Structure

**File:** `crates/zk/src/prover.rs`

```rust
/// Proof data with journal and digest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofData {
    /// The cryptographic proof (Groth16 seal in production)
    pub proof: Vec<u8>,

    /// The raw journal bytes (all public outputs)
    pub journal: Vec<u8>,

    /// SHA256 digest of journal (the actual Groth16 public input)
    pub journal_digest: [u8; 32],

    /// Parsed public inputs for convenience
    pub public_inputs: Vec<Vec<u8>>,
}
```

## Implementation Checklist

### Phase 1: Core Hash Functions
- [ ] Add `sha2` dependency to `game-core/Cargo.toml`
- [ ] Add `sha2` dependency to `zk/Cargo.toml`
- [ ] Add `sha2` dependency to both guest `Cargo.toml` files
- [ ] Implement `GameState::compute_state_root()` in `game-core`
- [ ] Implement `OracleSnapshot::compute_oracle_root()` in `zk`
- [ ] Implement `compute_actions_root()` helper in `zk`

### Phase 2: Guest Programs
- [x] Implement `state-transition/src/main.rs` with correct journal commits
- [x] Verify guest programs compile with `risc0` feature
- [x] Test guest programs with stub backend

### Phase 3: Host Prover
- [x] Update `ProofData` structure with `journal_digest` field
- [x] Implement `Risc0Prover::prove()` with journal parsing
- [x] Implement `parse_journal()` helper
- [x] Implement `compute_journal_digest()` helper
- [x] Implement `verify()` with journal verification
- [x] Test proof generation with stub backend
- [ ] Test proof generation with RISC0 backend (dev mode)

### Phase 4: Sui Contract
- [ ] Update `proof_verifier.move` with `JournalData` struct
- [ ] Implement `compute_journal_digest()` in Move
- [ ] Update `verify_game_proof()` for two-stage verification
- [ ] Add error code `EJournalMismatch`
- [ ] Update `game_session.move` to use new verification API
- [ ] Write Move tests for journal verification

### Phase 5: Integration Testing
- [ ] Test end-to-end proof generation and verification
- [ ] Test with various action sequences
- [ ] Verify journal digest computation matches between Rust and Move
- [ ] Test failure cases (invalid journal, mismatched digest, etc.)

### Phase 6: Groth16 Wrapper (Future)
- [ ] Integrate `stark_to_snark` conversion
- [ ] Update verifying key generation for Groth16
- [ ] Deploy Groth16 verifier contract on Sui testnet
- [ ] Test on-chain verification with real Groth16 proofs

## Design Rationale

### Why Simple Hash for State Root?

**For zkVM (RISC0, SP1):**
- Simple SHA256 hash is optimal
- Merkle trees provide no performance benefit
- zkVM verifies the entire state transition, not partial state access
- Simpler = fewer constraints = faster proving

**For Custom Circuits (Future Arkworks):**
- Merkle trees become valuable
- Allow proving partial state updates
- Reduce circuit size for selective state access
- Worth the complexity for circuit-based proving

### Why Two-Stage Verification?

**Technical Constraint:**
- RISC0 Groth16 wrapper only exposes `journalDigest` as public input
- Cannot directly access individual journal fields in Groth16 verification

**Benefits of Two-Stage:**
- ✅ Maintains compatibility with RISC0 architecture
- ✅ Preserves all 6 fields as verifiable data
- ✅ Minimal gas overhead (one SHA256 + comparison)
- ✅ Clean separation: cryptographic proof vs data validation

**Tradeoff:**
- ⚠️ Journal data (168 bytes) must be included in transaction calldata
- ⚠️ Slightly more complex contract logic
- ✅ Still much smaller than full state (journal is just roots + nonce)

### Why Journal Instead of Public Inputs?

**RISC0 Architecture:**
- Journal is the standard output mechanism for zkVM programs
- Public inputs in Groth16 are limited and handled by the wrapper
- Journal digest provides cryptographic commitment to all outputs

**Flexibility:**
- Easy to extend journal with additional fields
- No constraint on journal size (reasonable limits apply)
- Journal serialization is deterministic and verifiable

## References

- [RISC0 Groth16 Wrapper](https://crates.io/crates/risc0-groth16)
- [RISC0 Ethereum Verifier](https://github.com/risc0/risc0-ethereum)
- [RISC0 Terminology](https://dev.risczero.com/terminology)
- [Sui Groth16 Module](https://docs.sui.io/concepts/cryptography/groth16)

## Future Enhancements

### Optimistic Verification
- Skip journal verification for trusted provers
- Only verify digest on-chain
- Fraud proof system for disputes

### Batch Verification
- Verify multiple proofs in one transaction
- Amortize verification costs
- Aggregate journal digests

### Custom Circuit Migration
- Implement Arkworks-based custom circuit
- Use Merkle trees for state representation
- Selective state updates with inclusion proofs
- Reduced circuit size and faster proving
