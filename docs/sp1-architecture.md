# SP1 zkVM Integration Architecture

## Overview

This document describes the architecture for integrating SP1 zkVM as an alternative proving backend alongside RISC0. SP1 provides comparable functionality to RISC0 with different performance characteristics and proof types.

## Executive Summary

**Key Insight:** SP1's `public values` are conceptually identical to RISC0's `journal`. The two-stage verification model (digest-based Groth16 + explicit public values validation) is not only compatible with SP1 but is actually **the official recommended pattern** for SP1 on-chain verification.

**Recommendation:** Keep the existing 168-byte journal structure unchanged. SP1 integration requires only API-layer changes, with zero modifications to the proof structure or on-chain verification logic.

## RISC0 vs SP1 Terminology Mapping

### Core Concepts (1:1 Mapping)

| Concept | RISC0 | SP1 | Notes |
|---------|-------|-----|-------|
| **Public Outputs** | `journal` | `public values` | Identical semantics |
| **Guest Commit API** | `env::commit(&data)` | `sp1_zkvm::io::commit(&data)` | Same behavior |
| **Guest Commit Raw** | `env::commit_slice(&bytes)` | `sp1_zkvm::io::commit_slice(&bytes)` | Same behavior |
| **Public Digest** | `journal_digest` (manual) | `PublicValuesDigest` (built-in) | SP1 auto-computes |
| **Guest Entry** | `risc0_zkvm::guest::entry!(main)` | `sp1_zkvm::entrypoint!(main)` | Different macros |
| **Guest Read Input** | `env::read::<T>()` | `sp1_zkvm::io::read::<T>()` | Same semantics |

### Architecture Equivalence

```
RISC0                           SP1
─────────────────────────────────────────────────────
journal bytes (168)        ←→   public_values bytes (168)
SHA256(journal)            ←→   PublicValuesDigest
env::commit_slice()        ←→   sp1_zkvm::io::commit_slice()
Receipt                    ←→   SP1ProofWithPublicValues
Groth16 wrapper            ←→   Groth16 mode (native)
```

## Proof Types Comparison

### RISC0 Proof Flow

```
Guest Execution
    ↓
Composite Receipt (multi-segment STARK, several MB)
    ↓ compress(Succinct)
Succinct Receipt (single STARK, ~200-300 KB)
    ↓ compress(Groth16) [Linux x86_64 only]
Groth16 Receipt (~200 bytes)
    ↓
On-chain Verification
```

### SP1 Proof Flow

```
Guest Execution
    ↓
Core Proof (variable size STARK)
    ↓ compress() [optional]
Compressed Proof (constant size STARK)
    ↓ prove_groth16() or prove_plonk()
Groth16 Proof (~260 bytes) or PLONK Proof (~868 bytes)
    ↓
On-chain Verification
```

### Proof Type Matrix

| Backend | Core/STARK | Compressed | Groth16 | PLONK | Notes |
|---------|-----------|------------|---------|-------|-------|
| **RISC0** | Composite (multi-MB) | Succinct (~300 KB) | ~200 bytes (~270k gas) | ❌ | Groth16: Linux x86_64 only |
| **SP1** | Core (variable) | Compressed (constant) | ~260 bytes (~270k gas) | ~868 bytes (~300k gas) | PLONK: no trusted setup |

**Key Differences:**

1. **Platform Support:**
   - RISC0 Groth16: Linux x86_64 only (GPU/Docker required)
   - SP1 Groth16: All platforms supported

2. **Trusted Setup:**
   - RISC0 Groth16: Uses RISC0's trusted setup
   - SP1 Groth16: Uses Aztec Ignition + Succinct contributions
   - SP1 PLONK: **No trusted setup required** (unique to SP1)

3. **Performance:**
   - SP1: Generally faster proving (especially with Hypercube)
   - RISC0: More mature, battle-tested

## Two-Stage Verification Model

Both RISC0 and SP1 use the same two-stage verification pattern for Groth16/PLONK proofs:

### Stage 1: Cryptographic Proof Verification

**RISC0:**
```
Groth16.verify(
    public_input = journal_digest,  // SHA256(journal_bytes)
    proof = seal
)
```

**SP1:**
```
Groth16.verify(
    public_input = PublicValuesDigest,  // SHA256(public_values_bytes)
    proof = groth16_seal
)
```

### Stage 2: Public Values Content Verification

**RISC0:**
```solidity
// On-chain (Ethereum/Sui)
bytes32 computed_digest = sha256(journal_bytes);
require(computed_digest == journal_digest);
// Extract and validate 6 fields from journal_bytes
```

**SP1:**
```solidity
// On-chain (Ethereum/Sui)
bytes32 computed_digest = sha256(public_values_bytes);
require(computed_digest == public_values_digest);
// Extract and validate 6 fields from public_values_bytes
```

**Critical Insight:** This pattern is **not a workaround** but the **official SP1 best practice** per Veridise audit:

> "The SP1 verifier verifies the proof. The application must verify the accuracy of public information (public values) separately."

## Journal/Public Values Structure (Unchanged)

The existing 168-byte structure is optimal for both backends:

```rust
// Structure (identical for RISC0 and SP1)
struct PublicOutputs {
    oracle_root: [u8; 32],        // offset 0..32
    seed_commitment: [u8; 32],    // offset 32..64
    prev_state_root: [u8; 32],    // offset 64..96
    actions_root: [u8; 32],       // offset 96..128 (Walrus blob_id)
    new_state_root: [u8; 32],     // offset 128..160
    new_nonce: u64,               // offset 160..168
}

// Total: 168 bytes
// Digest: SHA256(all 168 bytes)
```

**Why No Changes Needed:**

1. ✅ **Already optimal:** 168 bytes is minimal and efficient
2. ✅ **Backend agnostic:** Works identically in RISC0 and SP1
3. ✅ **On-chain compatible:** Sui Move contracts expect this structure
4. ✅ **Digest-based:** Both use SHA-256 digest as SNARK public input
5. ✅ **Precompile support:** Both have SHA-256 hardware acceleration

### Potential Future Extensions (Optional)

```rust
// If needed, extend with additional context (not recommended initially)
struct ExtendedPublicOutputs {
    // Existing 168 bytes
    oracle_root: [u8; 32],
    seed_commitment: [u8; 32],
    prev_state_root: [u8; 32],
    actions_root: [u8; 32],
    new_state_root: [u8; 32],
    new_nonce: u64,

    // Additional context (future)
    chain_id: u32,                // 4 bytes - bind to specific chain
    epoch: u64,                   // 8 bytes - bind to epoch
    session_id: [u8; 32],         // 32 bytes - bind to game session

    // Total: 212 bytes
}
```

**Current Recommendation:** Do not extend. Keep 168 bytes for:
- Simplicity
- Gas efficiency
- Cross-backend compatibility
- Minimal on-chain calldata

## SP1 Guest Program Implementation

### Option A: Unified Guest with Feature Flags (Recommended)

**Pros:**
- Single source of truth
- Logic guaranteed to stay synchronized
- Easier maintenance
- Smaller codebase

**Cons:**
- More `#[cfg]` annotations
- Slightly more complex to read

**Implementation:**

```rust
// crates/zk/methods/state-transition/src/main.rs

#![no_main]
#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use game_core::{Action, GameEngine, GameState, OracleSnapshot, SnapshotOracleBundle, compute_actions_root};

// Backend-specific entry points
#[cfg(feature = "risc0")]
risc0_zkvm::guest::entry!(main);

#[cfg(feature = "sp1")]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    // ========================================================================
    // 1. READ INPUTS (backend-specific I/O)
    // ========================================================================

    #[cfg(feature = "risc0")]
    {
        let oracle_snapshot: OracleSnapshot = risc0_zkvm::guest::env::read();
        let seed_commitment: [u8; 32] = risc0_zkvm::guest::env::read();
        let mut state: GameState = risc0_zkvm::guest::env::read();
        let actions: Vec<Action> = risc0_zkvm::guest::env::read();
    }

    #[cfg(feature = "sp1")]
    {
        let oracle_snapshot: OracleSnapshot = sp1_zkvm::io::read();
        let seed_commitment: [u8; 32] = sp1_zkvm::io::read();
        let mut state: GameState = sp1_zkvm::io::read();
        let actions: Vec<Action> = sp1_zkvm::io::read();
    }

    // ========================================================================
    // 2. COMPUTE ROOTS BEFORE EXECUTION (backend-agnostic)
    // ========================================================================

    let oracle_root = oracle_snapshot.compute_oracle_root();
    let prev_state_root = state.compute_state_root();
    let actions_root = compute_actions_root(&actions);

    // ========================================================================
    // 3. EXECUTE ACTIONS (backend-agnostic)
    // ========================================================================

    let oracle_bundle = SnapshotOracleBundle::new(&oracle_snapshot);
    let env = oracle_bundle.as_env();
    let mut engine = GameEngine::new(&mut state);

    for (index, action) in actions.iter().enumerate() {
        engine.execute(env.as_game_env(), action).unwrap_or_else(|e| {
            panic!("Action {}/{} failed: {:?}", index + 1, actions.len(), e)
        });
    }

    // ========================================================================
    // 4. COMPUTE ROOTS AFTER EXECUTION (backend-agnostic)
    // ========================================================================

    let new_state_root = state.compute_state_root();
    let new_nonce = state.nonce();

    // ========================================================================
    // 5. COMMIT PUBLIC VALUES (backend-specific I/O)
    // ========================================================================

    // Build 168-byte public values buffer
    let mut public_values = [0u8; 168];
    public_values[0..32].copy_from_slice(&oracle_root);
    public_values[32..64].copy_from_slice(&seed_commitment);
    public_values[64..96].copy_from_slice(&prev_state_root);
    public_values[96..128].copy_from_slice(&actions_root);
    public_values[128..160].copy_from_slice(&new_state_root);
    public_values[160..168].copy_from_slice(&new_nonce.to_le_bytes());

    // Commit to journal/public_values
    #[cfg(feature = "risc0")]
    risc0_zkvm::guest::env::commit_slice(&public_values);

    #[cfg(feature = "sp1")]
    sp1_zkvm::io::commit_slice(&public_values);
}
```

### Option B: Separate Guest Files (Not Recommended)

**Pros:**
- Clean separation
- Backend-specific optimizations easier
- No feature flags in logic

**Cons:**
- Code duplication
- Synchronization risk
- Harder to maintain

**Structure:**
```
methods/
├── risc0-state-transition/
│   ├── Cargo.toml
│   └── src/main.rs
└── sp1-state-transition/
    ├── Cargo.toml
    └── src/main.rs
```

**Verdict:** Not recommended unless backends diverge significantly.

## SP1 Host-Side Prover Implementation

### Prover Interface (Unchanged)

```rust
// crates/zk/src/prover.rs

pub trait Prover: Send + Sync {
    fn prove(
        &self,
        start_state: &GameState,
        actions: &[Action],
        end_state: &GameState,
    ) -> Result<ProofData, ProofError>;

    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError>;
}
```

### SP1 Prover Implementation

```rust
// crates/zk/src/sp1/prover.rs

use sp1_sdk::{ProverClient, SP1Stdin, SP1ProofWithPublicValues, SP1ProvingKey, SP1VerifyingKey};
use crate::prover::{ProofBackend, ProofData, ProofError, Prover};
use crate::{OracleSnapshot, compute_journal_digest, parse_journal};
use game_core::{Action, GameState};

/// SP1 zkVM prover.
///
/// Maintains a cached oracle snapshot that is reused across all proof generations.
#[derive(Clone)]
pub struct Sp1Prover {
    oracle_snapshot: OracleSnapshot,
    client: ProverClient,
    pk: SP1ProvingKey,
    vk: SP1VerifyingKey,
}

impl Sp1Prover {
    /// Creates a new SP1 prover with the given oracle snapshot.
    pub fn new(oracle_snapshot: OracleSnapshot) -> Self {
        let client = ProverClient::new();
        let (pk, vk) = client.setup(STATE_TRANSITION_ELF);

        Self {
            oracle_snapshot,
            client,
            pk,
            vk,
        }
    }

    /// Computes seed commitment from GameState's game_seed.
    fn compute_seed_commitment(state: &GameState) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(state.game_seed.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verifies journal fields match expected values.
    fn verify_journal_fields(
        &self,
        journal_bytes: &[u8],
        start_state: &GameState,
        actions: &[Action],
        expected_end_state: &GameState,
    ) -> Result<(), ProofError> {
        // Parse 168-byte journal into 6 fields
        let fields = parse_journal(journal_bytes)?;

        // Compute expected values
        let expected_oracle_root = self.oracle_snapshot.compute_oracle_root();
        let expected_seed_commitment = Self::compute_seed_commitment(start_state);
        let expected_prev_state_root = start_state.compute_state_root();
        let expected_actions_root = game_core::compute_actions_root(actions);
        let expected_new_state_root = expected_end_state.compute_state_root();
        let expected_new_nonce = expected_end_state.nonce();

        // Verify all fields (same logic as RISC0)
        if fields.oracle_root != expected_oracle_root {
            return Err(ProofError::StateInconsistency(format!(
                "oracle_root mismatch: zkVM computed {:?}, expected {:?}",
                fields.oracle_root, expected_oracle_root
            )));
        }

        if fields.seed_commitment != expected_seed_commitment {
            return Err(ProofError::StateInconsistency(format!(
                "seed_commitment mismatch: zkVM committed {:?}, expected {:?}",
                fields.seed_commitment, expected_seed_commitment
            )));
        }

        if fields.prev_state_root != expected_prev_state_root {
            return Err(ProofError::StateInconsistency(format!(
                "prev_state_root mismatch: zkVM computed {:?}, expected {:?}",
                fields.prev_state_root, expected_prev_state_root
            )));
        }

        if fields.actions_root != expected_actions_root {
            return Err(ProofError::StateInconsistency(format!(
                "actions_root mismatch: zkVM computed {:?}, expected {:?}",
                fields.actions_root, expected_actions_root
            )));
        }

        if fields.new_state_root != expected_new_state_root {
            return Err(ProofError::StateInconsistency(format!(
                "new_state_root mismatch: zkVM computed {:?}, expected {:?}. \
                 This indicates non-determinism. zkVM nonce={}, expected nonce={}",
                fields.new_state_root,
                expected_new_state_root,
                fields.new_nonce,
                expected_new_nonce
            )));
        }

        if fields.new_nonce != expected_new_nonce {
            return Err(ProofError::StateInconsistency(format!(
                "new_nonce mismatch: zkVM computed {}, expected {}",
                fields.new_nonce, expected_new_nonce
            )));
        }

        Ok(())
    }
}

impl Prover for Sp1Prover {
    fn prove(
        &self,
        start_state: &GameState,
        actions: &[Action],
        expected_end_state: &GameState,
    ) -> Result<ProofData, ProofError> {
        // Compute seed commitment
        let seed_commitment = Self::compute_seed_commitment(start_state);

        // Build stdin (SP1's equivalent of ExecutorEnv)
        let mut stdin = SP1Stdin::new();
        stdin.write(&self.oracle_snapshot);
        stdin.write(&seed_commitment);
        stdin.write(start_state);
        stdin.write(&actions.to_vec());

        // Generate proof (Core proof for development)
        // For production, use prove_groth16() or prove_plonk()
        let proof = self.client
            .prove(&self.pk, stdin)
            .map_err(|e| ProofError::ZkvmError(format!("SP1 proof generation failed: {}", e)))?;

        // Extract public values (168 bytes)
        let journal = proof.public_values.to_vec();

        // Verify journal fields match expected values
        self.verify_journal_fields(&journal, start_state, actions, expected_end_state)?;

        // Compute journal digest
        let journal_digest = compute_journal_digest(&journal);

        // Serialize proof
        let bytes = bincode::serialize(&proof)
            .map_err(|e| ProofError::SerializationError(e.to_string()))?;

        Ok(ProofData {
            bytes,
            backend: ProofBackend::Sp1,
            journal,
            journal_digest,
        })
    }

    fn verify(&self, proof: &ProofData) -> Result<bool, ProofError> {
        // Verify backend matches
        if proof.backend != ProofBackend::Sp1 {
            return Err(ProofError::ZkvmError(format!(
                "Expected SP1 backend, got {:?}",
                proof.backend
            )));
        }

        // Deserialize proof
        let sp1_proof: SP1ProofWithPublicValues = bincode::deserialize(&proof.bytes)
            .map_err(|e| ProofError::SerializationError(e.to_string()))?;

        // Verify proof
        self.client
            .verify(&sp1_proof, &self.vk)
            .map_err(|e| ProofError::ZkvmError(format!("SP1 proof verification failed: {:?}", e)))?;

        Ok(true)
    }
}
```

## SP1 Groth16 Support

SP1's Groth16 support is simpler than RISC0's because it's platform-independent:

```rust
// crates/zk/src/sp1/groth16.rs

use sp1_sdk::{ProverClient, SP1ProofWithPublicValues};
use crate::prover::{ProofBackend, ProofData, ProofError};

/// Compress a Core/Compressed proof to Groth16.
///
/// Unlike RISC0, SP1's Groth16 works on all platforms (not just Linux x86_64).
///
/// # Arguments
///
/// * `core_proof` - ProofData containing a Core or Compressed proof
///
/// # Returns
///
/// Groth16 proof (~260 bytes) with identical public values
pub fn compress_to_groth16(core_proof: &ProofData) -> Result<ProofData, ProofError> {
    if core_proof.backend != ProofBackend::Sp1 {
        return Err(ProofError::ZkvmError(format!(
            "Expected SP1 proof, got {:?}",
            core_proof.backend
        )));
    }

    // Deserialize Core/Compressed proof
    let sp1_proof: SP1ProofWithPublicValues = bincode::deserialize(&core_proof.bytes)
        .map_err(|e| ProofError::SerializationError(format!("Failed to deserialize SP1 proof: {}", e)))?;

    // Generate Groth16 proof
    let client = ProverClient::new();
    let (_, vk) = client.setup(crate::STATE_TRANSITION_ELF);

    let groth16_proof = client
        .prove_groth16(&sp1_proof, &vk)
        .map_err(|e| ProofError::ZkvmError(format!("Groth16 compression failed: {}", e)))?;

    // Serialize Groth16 proof
    let bytes = bincode::serialize(&groth16_proof)
        .map_err(|e| ProofError::SerializationError(e.to_string()))?;

    Ok(ProofData {
        bytes,
        backend: ProofBackend::Sp1,
        journal: core_proof.journal.clone(),  // Public values unchanged
        journal_digest: core_proof.journal_digest,  // Digest unchanged
    })
}
```

## SP1 PLONK Support (Unique Feature)

SP1 offers PLONK as an alternative to Groth16 with **no trusted setup requirement**:

```rust
// crates/zk/src/sp1/plonk.rs

use sp1_sdk::{ProverClient, SP1ProofWithPublicValues};
use crate::prover::{ProofBackend, ProofData, ProofError};

/// Compress a Core/Compressed proof to PLONK.
///
/// PLONK proofs are larger (~868 bytes) than Groth16 (~260 bytes) but require
/// no trusted setup, making them trustless.
///
/// # Arguments
///
/// * `core_proof` - ProofData containing a Core or Compressed proof
///
/// # Returns
///
/// PLONK proof (~868 bytes, ~300k gas on-chain) with identical public values
pub fn compress_to_plonk(core_proof: &ProofData) -> Result<ProofData, ProofError> {
    if core_proof.backend != ProofBackend::Sp1 {
        return Err(ProofError::ZkvmError(format!(
            "Expected SP1 proof, got {:?}",
            core_proof.backend
        )));
    }

    // Deserialize Core/Compressed proof
    let sp1_proof: SP1ProofWithPublicValues = bincode::deserialize(&core_proof.bytes)
        .map_err(|e| ProofError::SerializationError(format!("Failed to deserialize SP1 proof: {}", e)))?;

    // Generate PLONK proof
    let client = ProverClient::new();
    let (_, vk) = client.setup(crate::STATE_TRANSITION_ELF);

    let plonk_proof = client
        .prove_plonk(&sp1_proof, &vk)
        .map_err(|e| ProofError::ZkvmError(format!("PLONK compression failed: {}", e)))?;

    // Serialize PLONK proof
    let bytes = bincode::serialize(&plonk_proof)
        .map_err(|e| ProofError::SerializationError(e.to_string()))?;

    Ok(ProofData {
        bytes,
        backend: ProofBackend::Sp1,
        journal: core_proof.journal.clone(),
        journal_digest: core_proof.journal_digest,
    })
}
```

## On-Chain Verification (Unchanged)

The Sui Move contract verification logic **does not change** for SP1:

```move
// contracts/move/sources/proof_verifier.move

/// Journal data structure - identical for RISC0 and SP1
public struct JournalData has copy, drop, store {
    oracle_root: vector<u8>,        // 32 bytes
    seed_commitment: vector<u8>,    // 32 bytes
    prev_state_root: vector<u8>,    // 32 bytes
    actions_root: vector<u8>,       // 32 bytes (Walrus blob_id)
    new_state_root: vector<u8>,     // 32 bytes
    new_nonce: u64,                 // 8 bytes
}

/// Verify a game state transition proof
///
/// Two-stage verification (works for both RISC0 and SP1):
/// 1. Verify Groth16/PLONK proof with journal_digest
/// 2. Verify journal_data hashes to journal_digest
public fun verify_game_proof(
    vk: &VerifyingKey,
    journal_digest: vector<u8>,     // 32 bytes - public input
    journal_data: &JournalData,     // 168 bytes - calldata
    proof_bytes: vector<u8>,        // Groth16/PLONK proof
) {
    // Stage 1: Verify cryptographic proof
    let curve = groth16::bn254();
    let proof_points = groth16::proof_points_from_bytes(proof_bytes);
    let public_inputs = groth16::public_proof_inputs_from_bytes(journal_digest);

    let valid = groth16::verify_groth16_proof(
        &curve,
        &vk.prepared_vk,
        &public_inputs,
        &proof_points,
    );
    assert!(valid, EInvalidProof);

    // Stage 2: Verify journal content
    let computed_digest = compute_journal_digest(journal_data);
    assert!(computed_digest == journal_digest, EJournalMismatch);
}

/// Compute SHA256 digest of journal data
///
/// Must match serialization order in guest program (RISC0/SP1)
fun compute_journal_digest(journal: &JournalData): vector<u8> {
    let mut bytes = vector::empty<u8>();

    vector::append(&mut bytes, journal.oracle_root);
    vector::append(&mut bytes, journal.seed_commitment);
    vector::append(&mut bytes, journal.prev_state_root);
    vector::append(&mut bytes, journal.actions_root);
    vector::append(&mut bytes, journal.new_state_root);
    vector::append(&mut bytes, u64_to_bytes(journal.new_nonce));

    sui::hash::sha256(&bytes)  // 32 bytes
}
```

**Key Point:** The Move contract is **backend-agnostic**. It verifies:
1. Groth16/PLONK proof with `journal_digest` as public input
2. `journal_data` hashes to `journal_digest`
3. Individual field constraints (game logic)

Whether the proof came from RISC0 or SP1 is irrelevant to the verifier.

## Build System Integration

### Cargo.toml Updates

```toml
# crates/zk/Cargo.toml

[features]
default = ["risc0"]

# Meta-feature enabled by all zkVM backends
zkvm = []

# Proving backends (mutually exclusive)
risc0 = ["zkvm", "dep:risc0-zkvm", "dep:risc0-build", "dep:risc0-groth16"]
sp1 = ["zkvm", "dep:sp1-sdk", "dep:sp1-build"]
stub = ["zkvm"]

[dependencies]
game-core = { workspace = true, features = ["serde"] }
thiserror = { workspace = true }
serde = { workspace = true }
bincode = { workspace = true }
tracing = { workspace = true }
sha2 = { workspace = true }

# RISC0 dependencies
risc0-zkvm = { workspace = true, optional = true }
risc0-groth16 = { workspace = true, optional = true }

# SP1 dependencies
sp1-sdk = { version = "5.2", optional = true }

[build-dependencies]
risc0-build = { workspace = true, optional = true }
sp1-build = { version = "5.2", optional = true }
```

### build.rs Updates

```rust
// crates/zk/build.rs

fn main() {
    #[cfg(feature = "risc0")]
    build_risc0();

    #[cfg(feature = "sp1")]
    build_sp1();
}

#[cfg(feature = "risc0")]
fn build_risc0() {
    use risc0_build::GuestOptions;

    risc0_build::embed_methods_with_options(std::collections::HashMap::from([(
        "methods/state-transition",
        GuestOptions::default(),
    )]));
}

#[cfg(feature = "sp1")]
fn build_sp1() {
    use sp1_build::{build_program_with_args, BuildArgs};

    println!("cargo:rerun-if-changed=methods/state-transition/src");

    build_program_with_args(
        "methods/state-transition",
        BuildArgs::default(),
    );
}
```

### Justfile Updates

```makefile
# justfile

# SP1 backend commands
build-sp1:
    cargo build --workspace --no-default-features --features sp1

run-sp1:
    cargo run -p client-cli --no-default-features --features sp1

test-sp1:
    cargo test --workspace --no-default-features --features sp1

lint-sp1:
    cargo clippy --workspace --all-targets --no-default-features --features sp1

# Check all backends compile
check-all:
    @echo "Building stub backend..."
    just build stub
    @echo "Building RISC0 backend..."
    just build risc0
    @echo "Building SP1 backend..."
    just build sp1
    @echo "All backends compiled successfully!"
```

## Migration Strategy

### Phase 1: Feature Flags (1-2 hours)

**Goal:** Add SP1 feature infrastructure without implementation

**Tasks:**
1. Add `sp1` feature to `crates/zk/Cargo.toml`
2. Add `ProofBackend::Sp1` to `crates/zk/src/prover.rs`
3. Add conditional compilation in `crates/zk/src/lib.rs`
4. Verify builds: `cargo build --features sp1` (will fail, expected)

**Files:**
- `crates/zk/Cargo.toml`
- `crates/zk/src/prover.rs`
- `crates/zk/src/lib.rs`

### Phase 2: Guest Program (2-3 hours)

**Goal:** Add SP1 I/O to guest program

**Tasks:**
1. Add SP1 dependencies to `methods/state-transition/Cargo.toml`
2. Add `#[cfg(feature = "sp1")]` entry point and I/O
3. Test compilation: `cargo build -p state-transition --features sp1`
4. Verify guest logic unchanged (only I/O differs)

**Files:**
- `crates/zk/methods/state-transition/Cargo.toml`
- `crates/zk/methods/state-transition/src/main.rs`

### Phase 3: Host Prover (4-6 hours)

**Goal:** Implement SP1 host-side prover

**Tasks:**
1. Create `crates/zk/src/sp1/mod.rs`
2. Implement `Sp1Prover` struct with `Prover` trait
3. Reuse journal verification logic from RISC0
4. Add integration tests
5. Verify proofs generate successfully

**Files:**
- `crates/zk/src/sp1/mod.rs`
- `crates/zk/src/sp1/prover.rs`
- `crates/zk/tests/sp1_integration.rs`

### Phase 4: Build System (2-3 hours)

**Goal:** SP1 guest compilation

**Tasks:**
1. Update `crates/zk/build.rs` with SP1 build logic
2. Add SP1 SDK dependencies to workspace `Cargo.toml`
3. Test end-to-end build: `cargo build --features sp1`
4. Verify ELF/ImageID generation

**Files:**
- `crates/zk/build.rs`
- `Cargo.toml` (workspace root)

### Phase 5: Groth16/PLONK (3-4 hours)

**Goal:** Add proof compression

**Tasks:**
1. Implement `compress_to_groth16()` in `crates/zk/src/sp1/groth16.rs`
2. Implement `compress_to_plonk()` in `crates/zk/src/sp1/plonk.rs`
3. Add compression tests
4. Document compression tradeoffs

**Files:**
- `crates/zk/src/sp1/groth16.rs`
- `crates/zk/src/sp1/plonk.rs`

### Phase 6: Testing & Documentation (2-4 hours)

**Goal:** Comprehensive testing and docs

**Tasks:**
1. Cross-backend journal compatibility tests
2. Performance benchmarks (RISC0 vs SP1)
3. Update `docs/sp1-architecture.md` with results
4. Add Justfile recipes

**Files:**
- `crates/zk/tests/cross_backend.rs`
- `docs/sp1-architecture.md`
- `justfile`

**Total Estimated Time:** 14-22 hours (2-3 days)

## Testing Strategy

### Cross-Backend Compatibility Test

```rust
// crates/zk/tests/cross_backend.rs

#[cfg(all(feature = "risc0", feature = "sp1"))]
compile_error!("Cannot test both backends simultaneously");

#[test]
fn test_journal_structure_compatibility() {
    use zk::{OracleSnapshot, compute_journal_digest, parse_journal};
    use game_core::{GameState, Action};

    let oracle = OracleSnapshot::default();
    let state = GameState::new_test();
    let actions = vec![Action::Wait];
    let expected = state.clone();

    // Generate proof with active backend
    #[cfg(feature = "risc0")]
    let proof = {
        let prover = zk::Risc0Prover::new(oracle.clone());
        prover.prove(&state, &actions, &expected).unwrap()
    };

    #[cfg(feature = "sp1")]
    let proof = {
        let prover = zk::Sp1Prover::new(oracle.clone());
        prover.prove(&state, &actions, &expected).unwrap()
    };

    // Verify journal structure
    assert_eq!(proof.journal.len(), 168, "Journal must be 168 bytes");

    // Parse journal
    let fields = parse_journal(&proof.journal).unwrap();

    // Verify digest
    let computed_digest = compute_journal_digest(&proof.journal);
    assert_eq!(computed_digest, proof.journal_digest);

    // Verify field contents
    assert_eq!(fields.oracle_root, oracle.compute_oracle_root());
    assert_eq!(fields.prev_state_root, state.compute_state_root());
    assert_eq!(fields.new_state_root, expected.compute_state_root());
    assert_eq!(fields.new_nonce, expected.nonce());
}
```

### Performance Benchmark

```rust
// crates/zk/benches/prover_comparison.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zk::{OracleSnapshot, Prover};
use game_core::{GameState, Action};

fn benchmark_proof_generation(c: &mut Criterion) {
    let oracle = OracleSnapshot::default();
    let state = GameState::new_test();
    let actions = vec![Action::Wait; 10];  // 10 actions
    let expected = state.clone();

    #[cfg(feature = "risc0")]
    {
        let prover = zk::Risc0Prover::new(oracle.clone());
        c.bench_function("risc0_10_actions", |b| {
            b.iter(|| {
                prover.prove(
                    black_box(&state),
                    black_box(&actions),
                    black_box(&expected),
                )
            })
        });
    }

    #[cfg(feature = "sp1")]
    {
        let prover = zk::Sp1Prover::new(oracle.clone());
        c.bench_function("sp1_10_actions", |b| {
            b.iter(|| {
                prover.prove(
                    black_box(&state),
                    black_box(&actions),
                    black_box(&expected),
                )
            })
        });
    }
}

criterion_group!(benches, benchmark_proof_generation);
criterion_main!(benches);
```

## Future Optimizations (Not Recommended Initially)

### SP1 Precompile Acceleration

SP1's flexible precompile system could accelerate cryptographic operations:

```rust
// Hypothetical optimization (don't implement yet)

#[cfg(feature = "sp1")]
use sp1_zkvm::precompiles::sha256;

impl GameState {
    pub fn compute_state_root(&self) -> [u8; 32] {
        #[cfg(feature = "sp1")]
        {
            // Use SP1 hardware-accelerated SHA-256
            let bytes = bincode::serialize(self).unwrap();
            sha256::hash(&bytes)
        }

        #[cfg(not(feature = "sp1"))]
        {
            // Fallback to software implementation
            use sha2::{Sha256, Digest};
            let bytes = bincode::serialize(self).unwrap();
            Sha256::digest(&bytes).into()
        }
    }
}
```

**Why not now:**
1. Premature optimization
2. Adds complexity
3. Backend coupling
4. Unclear performance benefit

**When to consider:**
- After benchmarking shows SHA-256 bottleneck
- For large-scale batch proving (100+ actions)
- Performance-critical production scenarios

## Design Rationale

### Why Keep Existing Journal Structure?

**Technical reasons:**
1. ✅ Already minimal (168 bytes)
2. ✅ SHA-256 digest reduces to 32 bytes (SNARK public input)
3. ✅ Both RISC0 and SP1 have SHA-256 precompiles
4. ✅ On-chain verification identical for both

**Practical reasons:**
1. ✅ No Sui Move contract changes needed
2. ✅ Single verification logic
3. ✅ Backend interoperability
4. ✅ Future-proof for other zkVMs

### Why Two-Stage Verification Is Optimal?

**Not a workaround, but best practice:**

Per Veridise SP1 audit:
> "SP1 verifier verifies the proof. Application must verify public information separately."

**Benefits:**
1. ✅ Separates cryptographic proof from data validation
2. ✅ Maintains all 6 fields as verifiable
3. ✅ Minimal gas overhead (one SHA-256)
4. ✅ Clean abstraction boundary

**Comparison:**

| Approach | Public Input Size | On-Chain Cost | Flexibility |
|----------|------------------|---------------|-------------|
| **Direct fields** | 168 bytes | High (6 field verifications) | Rigid |
| **Digest-based** | 32 bytes | Low (1 hash + compare) | ✅ Extensible |

### Why Unified Guest Program?

**Single source of truth:**
- Core logic guaranteed identical
- Bug fixes apply to all backends
- Easier code review

**Minimal feature flags:**
- Only I/O differs between backends
- ~95% of code is shared
- Clear separation of concerns

**Alternatives considered:**
- Separate guest files: Code duplication risk
- Macro abstraction: Over-engineering
- **Verdict:** Feature flags strike best balance

## Conclusion

**SP1 integration requires:**
1. ✅ API-layer changes only (guest I/O + host SDK)
2. ✅ Zero changes to journal structure
3. ✅ Zero changes to on-chain verification
4. ✅ Reuse of journal validation logic

**Expected benefits:**
- Faster proof generation (SP1 generally faster)
- Platform-independent Groth16 (vs RISC0's Linux-only)
- PLONK option (no trusted setup)
- Backend diversity (reduced vendor lock-in)

**Estimated effort:** 14-22 hours (2-3 days)

**Recommendation:** Implement SP1 support to provide users with backend choice while maintaining a unified verification architecture.

## References

- [SP1 Official Documentation](https://docs.succinct.xyz/docs/sp1)
- [SP1 GitHub Repository](https://github.com/succinctlabs/sp1)
- [SP1 Proof Types](https://docs.succinct.xyz/docs/sp1/generating-proofs/proof-types)
- [SP1 Security Audit (Veridise)](https://blog.sigmaprime.io/sp1-zkvm-security-guide.html)
- [Comparative Analysis: SP1 vs RISC Zero](https://medium.com/@gwrx2005/comparative-analysis-of-sp1-and-risc-zero-zero-knowledge-virtual-machines-4abf806daa70)
- [SP1 Hypercube Announcement](https://blog.succinct.xyz/sp1-hypercube/)
