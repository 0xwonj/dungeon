# SP1 Implementation Plan

## Quick Reference

**Goal:** Add SP1 zkVM as alternative proving backend alongside RISC0

**Scope:** API-layer changes only, zero changes to proof structure or on-chain verification

**Timeline:** 14-22 hours (2-3 days of focused work)

**Architecture Doc:** See [sp1-architecture.md](./sp1-architecture.md) for design details

## Prerequisites

```bash
# Install SP1 toolchain
curl -L https://sp1.succinct.xyz | bash
sp1up

# Verify installation
cargo prove --version
```

## Implementation Phases

### Phase 1: Feature Flags Infrastructure (1-2 hours)

**Goal:** Add SP1 feature flags without implementation

**Files to modify:**
- `crates/zk/Cargo.toml`
- `crates/zk/src/lib.rs`
- `crates/zk/src/prover.rs`

**Tasks:**

- [ ] Add SP1 feature to `crates/zk/Cargo.toml`:
  ```toml
  [features]
  sp1 = ["zkvm", "dep:sp1-sdk", "dep:sp1-build"]

  [dependencies]
  sp1-sdk = { version = "5.2", optional = true }

  [build-dependencies]
  sp1-build = { version = "5.2", optional = true }
  ```

- [ ] Add `ProofBackend::Sp1` enum variant in `crates/zk/src/prover.rs`:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
  pub enum ProofBackend {
      #[cfg(feature = "stub")]
      Stub,
      #[cfg(feature = "risc0")]
      Risc0,
      #[cfg(feature = "sp1")]
      Sp1,
      #[cfg(feature = "arkworks")]
      Arkworks,
  }
  ```

- [ ] Add SP1 module declaration in `crates/zk/src/lib.rs`:
  ```rust
  #[cfg(feature = "sp1")]
  pub mod sp1;

  #[cfg(feature = "sp1")]
  pub use sp1::*;

  #[cfg(feature = "sp1")]
  pub type ZkProver = Sp1Prover;
  ```

- [ ] Add feature conflict check in `crates/zk/src/lib.rs`:
  ```rust
  #[cfg(any(
      all(feature = "risc0", feature = "sp1"),
      all(feature = "stub", feature = "sp1"),
      // ... other combinations
  ))]
  compile_error!("Enable exactly one backend: risc0, sp1, stub, or arkworks");
  ```

**Verification:**
```bash
# Should fail to build (no implementation yet)
cargo build --no-default-features --features sp1
```

### Phase 2: Guest Program (2-3 hours)

**Goal:** Add SP1 I/O to state-transition guest program

**Files to modify:**
- `crates/zk/methods/state-transition/Cargo.toml`
- `crates/zk/methods/state-transition/src/main.rs`

**Tasks:**

- [ ] Add SP1 dependencies to guest `Cargo.toml`:
  ```toml
  [dependencies]
  sp1-zkvm = { version = "5.2", optional = true }

  [features]
  risc0 = ["dep:risc0-zkvm"]
  sp1 = ["dep:sp1-zkvm"]
  ```

- [ ] Add SP1 entry point in `main.rs`:
  ```rust
  #[cfg(feature = "risc0")]
  risc0_zkvm::guest::entry!(main);

  #[cfg(feature = "sp1")]
  sp1_zkvm::entrypoint!(main);
  ```

- [ ] Add SP1 I/O in input reading section:
  ```rust
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
  ```

- [ ] Add SP1 output commit:
  ```rust
  // After building 168-byte public_values buffer

  #[cfg(feature = "risc0")]
  risc0_zkvm::guest::env::commit_slice(&public_values);

  #[cfg(feature = "sp1")]
  sp1_zkvm::io::commit_slice(&public_values);
  ```

- [ ] Verify guest logic unchanged (only I/O differs)

**Verification:**
```bash
# Guest program should compile
cargo build -p state-transition --no-default-features --features sp1
```

### Phase 3: Build System (2-3 hours)

**Goal:** SP1 guest compilation integration

**Files to modify:**
- `crates/zk/build.rs`
- `Cargo.toml` (workspace root)

**Tasks:**

- [ ] Add SP1 SDK to workspace `Cargo.toml`:
  ```toml
  [workspace.dependencies]
  sp1-sdk = { version = "5.2" }
  sp1-build = { version = "5.2" }
  ```

- [ ] Update `crates/zk/build.rs` with SP1 build logic:
  ```rust
  fn main() {
      #[cfg(feature = "risc0")]
      build_risc0();

      #[cfg(feature = "sp1")]
      build_sp1();
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

- [ ] Create `crates/zk/src/sp1/mod.rs`:
  ```rust
  //! SP1 zkVM backend module.

  mod prover;
  pub use prover::Sp1Prover;

  // SP1-specific modules
  #[cfg(feature = "sp1")]
  pub mod groth16;

  #[cfg(feature = "sp1")]
  pub mod plonk;
  ```

- [ ] Handle ELF/ImageID generation (similar to RISC0's `methods.rs`)

**Verification:**
```bash
# Should compile and generate SP1 ELF
cargo build --no-default-features --features sp1
```

### Phase 4: Host Prover (4-6 hours)

**Goal:** Implement `Sp1Prover` with `Prover` trait

**Files to create:**
- `crates/zk/src/sp1/prover.rs`

**Tasks:**

- [ ] Create `Sp1Prover` struct:
  ```rust
  use sp1_sdk::{ProverClient, SP1Stdin, SP1ProofWithPublicValues, SP1ProvingKey, SP1VerifyingKey};

  #[derive(Clone)]
  pub struct Sp1Prover {
      oracle_snapshot: OracleSnapshot,
      client: ProverClient,
      pk: SP1ProvingKey,
      vk: SP1VerifyingKey,
  }
  ```

- [ ] Implement `new()` constructor:
  ```rust
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
  ```

- [ ] Copy `compute_seed_commitment()` from RISC0 prover (unchanged)

- [ ] Copy `verify_journal_fields()` from RISC0 prover (unchanged)

- [ ] Implement `Prover::prove()`:
  ```rust
  fn prove(
      &self,
      start_state: &GameState,
      actions: &[Action],
      expected_end_state: &GameState,
  ) -> Result<ProofData, ProofError> {
      let seed_commitment = Self::compute_seed_commitment(start_state);

      // Build stdin (SP1's ExecutorEnv equivalent)
      let mut stdin = SP1Stdin::new();
      stdin.write(&self.oracle_snapshot);
      stdin.write(&seed_commitment);
      stdin.write(start_state);
      stdin.write(&actions.to_vec());

      // Generate proof
      let proof = self.client.prove(&self.pk, stdin)?;

      // Extract public values (168 bytes journal)
      let journal = proof.public_values.to_vec();

      // Verify journal fields
      self.verify_journal_fields(&journal, start_state, actions, expected_end_state)?;

      // Compute digest
      let journal_digest = compute_journal_digest(&journal);

      // Serialize
      let bytes = bincode::serialize(&proof)?;

      Ok(ProofData {
          bytes,
          backend: ProofBackend::Sp1,
          journal,
          journal_digest,
      })
  }
  ```

- [ ] Implement `Prover::verify()`:
  ```rust
  fn verify(&self, proof: &ProofData) -> Result<bool, ProofError> {
      if proof.backend != ProofBackend::Sp1 {
          return Err(ProofError::ZkvmError(format!(
              "Expected SP1 backend, got {:?}",
              proof.backend
          )));
      }

      let sp1_proof: SP1ProofWithPublicValues = bincode::deserialize(&proof.bytes)?;
      self.client.verify(&sp1_proof, &self.vk)?;

      Ok(true)
  }
  ```

**Verification:**
```bash
# Should generate proofs successfully
cargo test --no-default-features --features sp1 -- --nocapture
```

### Phase 5: Groth16 Support (3-4 hours)

**Goal:** Implement Groth16 compression for on-chain use

**Files to create:**
- `crates/zk/src/sp1/groth16.rs`

**Tasks:**

- [ ] Implement `compress_to_groth16()`:
  ```rust
  pub fn compress_to_groth16(core_proof: &ProofData) -> Result<ProofData, ProofError> {
      if core_proof.backend != ProofBackend::Sp1 {
          return Err(ProofError::ZkvmError(format!(
              "Expected SP1 proof, got {:?}",
              core_proof.backend
          )));
      }

      let sp1_proof: SP1ProofWithPublicValues = bincode::deserialize(&core_proof.bytes)?;

      let client = ProverClient::new();
      let (_, vk) = client.setup(STATE_TRANSITION_ELF);

      let groth16_proof = client.prove_groth16(&sp1_proof, &vk)?;

      let bytes = bincode::serialize(&groth16_proof)?;

      Ok(ProofData {
          bytes,
          backend: ProofBackend::Sp1,
          journal: core_proof.journal.clone(),
          journal_digest: core_proof.journal_digest,
      })
  }
  ```

- [ ] Add Groth16 compression tests

- [ ] Document platform support (all platforms, unlike RISC0)

**Verification:**
```bash
# Test Groth16 compression
cargo test --no-default-features --features sp1 sp1_groth16
```

### Phase 6: PLONK Support (Optional, 2-3 hours)

**Goal:** Implement PLONK compression (no trusted setup)

**Files to create:**
- `crates/zk/src/sp1/plonk.rs`

**Tasks:**

- [ ] Implement `compress_to_plonk()` (similar to Groth16)

- [ ] Document tradeoffs:
  - Larger proof size (~868 bytes vs ~260 bytes)
  - Higher gas cost (~300k vs ~270k)
  - No trusted setup requirement (unique benefit)

- [ ] Add PLONK tests

**Verification:**
```bash
cargo test --no-default-features --features sp1 sp1_plonk
```

### Phase 7: Integration Tests (2-4 hours)

**Goal:** Comprehensive testing and cross-backend validation

**Files to create:**
- `crates/zk/tests/sp1_integration.rs`
- `crates/zk/tests/cross_backend.rs`

**Tasks:**

- [ ] Implement cross-backend compatibility test:
  ```rust
  #[test]
  fn test_journal_compatibility() {
      let oracle = OracleSnapshot::default();
      let state = GameState::new_test();
      let actions = vec![Action::Wait];
      let expected = state.clone();

      #[cfg(feature = "risc0")]
      let proof = Risc0Prover::new(oracle.clone()).prove(&state, &actions, &expected)?;

      #[cfg(feature = "sp1")]
      let proof = Sp1Prover::new(oracle.clone()).prove(&state, &actions, &expected)?;

      // Verify journal structure
      assert_eq!(proof.journal.len(), 168);
      let fields = parse_journal(&proof.journal)?;

      // Verify digest computation
      let computed_digest = compute_journal_digest(&proof.journal);
      assert_eq!(computed_digest, proof.journal_digest);
  }
  ```

- [ ] Add end-to-end proof generation tests

- [ ] Add proof verification tests

- [ ] Add failure case tests (invalid journal, mismatched digest)

**Verification:**
```bash
# Run all SP1 tests
cargo test --no-default-features --features sp1

# Run cross-backend tests
cargo test --no-default-features --features risc0
cargo test --no-default-features --features sp1
```

### Phase 8: Justfile Integration (1 hour)

**Goal:** Add convenient build commands

**Files to modify:**
- `justfile`

**Tasks:**

- [ ] Add SP1 build commands:
  ```makefile
  # SP1 backend
  build-sp1:
      cargo build --workspace --no-default-features --features sp1

  run-sp1:
      cargo run -p client-cli --no-default-features --features sp1

  test-sp1:
      cargo test --workspace --no-default-features --features sp1

  lint-sp1:
      cargo clippy --workspace --all-targets --no-default-features --features sp1

  # Fast mode
  run-fast-sp1:
      cargo run -p client-cli --no-default-features --features sp1 -- --fast-mode
  ```

- [ ] Update `check-all` command:
  ```makefile
  check-all:
      @echo "Building stub backend..."
      just build stub
      @echo "Building RISC0 backend..."
      just build risc0
      @echo "Building SP1 backend..."
      just build sp1
      @echo "All backends compiled successfully!"
  ```

**Verification:**
```bash
just build-sp1
just test-sp1
just check-all
```

### Phase 9: Documentation (1-2 hours)

**Goal:** Update documentation and examples

**Tasks:**

- [ ] Update `CLAUDE.md` with SP1 build commands

- [ ] Add SP1 section to `README.md`

- [ ] Update `docs/zk-workflow.md` with SP1 examples

- [ ] Add performance comparison notes (after benchmarking)

- [ ] Document platform differences:
  - RISC0 Groth16: Linux x86_64 only
  - SP1 Groth16: All platforms
  - SP1 PLONK: All platforms, no trusted setup

**Verification:**
- Documentation accurately reflects implementation
- All code examples compile
- Build commands work as documented

## Verification Checklist

After completing all phases:

- [ ] `cargo build --no-default-features --features sp1` succeeds
- [ ] `cargo test --no-default-features --features sp1` passes
- [ ] `just build-sp1` works
- [ ] `just test-sp1` passes
- [ ] `just run-sp1` launches client successfully
- [ ] `just check-all` verifies all backends compile
- [ ] Journal structure identical between RISC0 and SP1 (168 bytes)
- [ ] Journal digest computation matches
- [ ] Proof generation succeeds
- [ ] Proof verification succeeds
- [ ] Groth16 compression works
- [ ] Documentation updated

## Performance Benchmarking (Optional)

After implementation, benchmark both backends:

```bash
# Create benchmark
cat > crates/zk/benches/prover_comparison.rs << 'EOF'
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_provers(c: &mut Criterion) {
    let oracle = zk::OracleSnapshot::default();
    let state = game_core::GameState::new_test();
    let actions = vec![game_core::Action::Wait; 10];
    let expected = state.clone();

    #[cfg(feature = "risc0")]
    {
        let prover = zk::Risc0Prover::new(oracle.clone());
        c.bench_function("risc0_10_actions", |b| {
            b.iter(|| prover.prove(black_box(&state), black_box(&actions), black_box(&expected)))
        });
    }

    #[cfg(feature = "sp1")]
    {
        let prover = zk::Sp1Prover::new(oracle.clone());
        c.bench_function("sp1_10_actions", |b| {
            b.iter(|| prover.prove(black_box(&state), black_box(&actions), black_box(&expected)))
        });
    }
}

criterion_group!(benches, benchmark_provers);
criterion_main!(benches);
EOF

# Run benchmarks
cargo bench --no-default-features --features risc0
cargo bench --no-default-features --features sp1

# Compare results
```

## Troubleshooting

### Common Issues

**SP1 build fails:**
```bash
# Ensure SP1 toolchain installed
sp1up

# Verify installation
cargo prove --version
```

**Guest program compilation errors:**
```bash
# Check feature flags
cargo tree --features sp1 | grep sp1

# Verify guest dependencies
cd crates/zk/methods/state-transition
cargo check --no-default-features --features sp1
```

**Journal size mismatch:**
```rust
// Verify commit order matches RISC0:
// 1. oracle_root (32 bytes)
// 2. seed_commitment (32 bytes)
// 3. prev_state_root (32 bytes)
// 4. actions_root (32 bytes)
// 5. new_state_root (32 bytes)
// 6. new_nonce (8 bytes)
// Total: 168 bytes
```

**Proof verification fails:**
```bash
# Enable debug logging
RUST_LOG=debug cargo test --no-default-features --features sp1 -- --nocapture

# Check journal digest computation
# Should match between host and guest
```

## Next Steps After Implementation

1. **Performance Analysis:**
   - Benchmark RISC0 vs SP1 proving times
   - Compare proof sizes
   - Measure gas costs (if different)

2. **Production Readiness:**
   - Test with large action batches (100+ actions)
   - Stress test proof generation
   - Verify memory usage

3. **On-Chain Integration:**
   - Deploy SP1 Groth16 verifier on Sui testnet
   - Test end-to-end verification
   - Compare gas costs with RISC0

4. **Documentation:**
   - Add migration guide for users
   - Document when to use SP1 vs RISC0
   - Performance comparison matrix

5. **Future Enhancements:**
   - SP1 Hypercube integration (when stable)
   - Advanced precompile usage
   - Custom optimization strategies

## Success Criteria

Implementation is complete when:

1. ✅ All phases pass verification
2. ✅ Tests pass for both RISC0 and SP1
3. ✅ Journal structure identical (168 bytes)
4. ✅ Journal digest matches across backends
5. ✅ On-chain verification works (same contract)
6. ✅ Documentation complete and accurate
7. ✅ Justfile commands functional
8. ✅ CI/CD updated (if applicable)

## Estimated Timeline

| Phase | Time | Cumulative |
|-------|------|------------|
| Phase 1: Feature Flags | 1-2 hours | 1-2 hours |
| Phase 2: Guest Program | 2-3 hours | 3-5 hours |
| Phase 3: Build System | 2-3 hours | 5-8 hours |
| Phase 4: Host Prover | 4-6 hours | 9-14 hours |
| Phase 5: Groth16 | 3-4 hours | 12-18 hours |
| Phase 6: PLONK (optional) | 2-3 hours | 14-21 hours |
| Phase 7: Integration Tests | 2-4 hours | 16-25 hours |
| Phase 8: Justfile | 1 hour | 17-26 hours |
| Phase 9: Documentation | 1-2 hours | 18-28 hours |

**Total:** 18-28 hours (2.5-4 days of focused work)

**Recommended approach:** Complete in order, verify each phase before proceeding to next.
