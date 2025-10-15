# ZK Feature Flags Guide

This document explains how to use Cargo feature flags to control which ZK proving backend is compiled.

## Quick Reference

```bash
# Default: zkVM stub (no real proving yet)
cargo build

# With SP1 (when implemented)
cargo build --features sp1

# Custom circuit only (Phase 2+)
cargo build --no-default-features --features custom-circuit

# Hybrid: both zkVM and custom circuit
cargo build --features sp1,custom-circuit

# Check what gets compiled
cargo tree -p zk --features sp1
```

## Feature Flags

### `zkvm` (default)

Enables zkVM support. Currently includes a stub prover.

**Compiles:**
- `zk/src/zkvm/mod.rs` ✅
- `zk/src/circuit/mod.rs` ❌ (excluded)

**Dependencies:**
- `game-core`
- `thiserror`

**No optional dependencies are pulled in.**

### `sp1`

Enables SP1 zkVM backend. Implies `zkvm`.

**Compiles:**
- `zk/src/zkvm/mod.rs` ✅
- `zk/src/zkvm/sp1.rs` ✅ (when implemented)
- `zk/src/circuit/mod.rs` ❌ (excluded)

**Dependencies:**
- All from `zkvm` feature
- `sp1-sdk` (optional dependency)

### `risc0`

Enables RISC0 zkVM backend. Implies `zkvm`.

**Compiles:**
- `zk/src/zkvm/mod.rs` ✅
- `zk/src/zkvm/risc0.rs` ✅ (when implemented)
- `zk/src/circuit/mod.rs` ❌ (excluded)

**Dependencies:**
- All from `zkvm` feature
- `risc0-zkvm` (optional dependency)

### `custom-circuit`

Enables custom circuit proving with Merkle trees.

**Compiles:**
- `zk/src/zkvm/mod.rs` ❌ (excluded)
- `zk/src/circuit/mod.rs` ✅
- `zk/src/circuit/merkle/` ✅ (when implemented)
- `zk/src/circuit/witness.rs` ✅ (when implemented)

**Dependencies:**
- `game-core`
- `thiserror`
- `blake3` (optional, for hashing)
- `serde` (optional, for serialization)
- `bincode` (optional, for encoding)

## Code Examples

### Conditional Compilation in zk Crate

```rust
// Only compiled with zkvm feature (default)
#[cfg(feature = "zkvm")]
pub mod zkvm;

// Only compiled with custom-circuit feature
#[cfg(feature = "custom-circuit")]
pub mod circuit;

// Available with zkvm feature
#[cfg(feature = "zkvm")]
pub use zkvm::*;
```

### Using Different Backends

```rust
// In ProverWorker or application code

#[cfg(feature = "sp1")]
use zk::zkvm::Sp1Prover;

#[cfg(feature = "risc0")]
use zk::zkvm::Risc0Prover;

#[cfg(feature = "custom-circuit")]
use zk::circuit::CircuitProver;

// With default features (stub)
#[cfg(all(feature = "zkvm", not(feature = "sp1"), not(feature = "risc0")))]
use zk::zkvm::StubZkvmProver as Prover;
```

### Hybrid Setup (Both zkVM and Custom Circuit)

```rust
pub enum ProverBackend {
    #[cfg(feature = "zkvm")]
    Zkvm(Box<dyn ZkvmProver>),

    #[cfg(feature = "custom-circuit")]
    Circuit(CircuitProver),
}

impl ProverBackend {
    pub fn prove(&self, ...) -> Result<ProofData, ProofError> {
        match self {
            #[cfg(feature = "zkvm")]
            ProverBackend::Zkvm(prover) => prover.prove(...),

            #[cfg(feature = "custom-circuit")]
            ProverBackend::Circuit(prover) => prover.prove(...),
        }
    }
}
```

## CI Configuration

### GitHub Actions

```yaml
# .github/workflows/ci.yml

jobs:
  test-zkvm-default:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test -p zk
      # Tests default zkvm feature

  test-custom-circuit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test -p zk --no-default-features --features custom-circuit
      # Tests custom circuit (when implemented)

  test-hybrid:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test -p zk --features sp1,custom-circuit
      # Tests both backends together
```

## Deployment Scenarios

### Development (Current)

```toml
# Use stub prover for testing
zk = { path = "../zk" }
```

**Result:** No real proving, fast compilation, good for development.

### Production - zkVM Only

```toml
# Use SP1 for real proofs
zk = { path = "../zk", features = ["sp1"] }
```

**Result:** Real ZK proofs, slower generation (5-60s), simpler implementation.

### Production - Custom Circuit Only

```toml
# Use custom circuit for performance
zk = { path = "../zk", default-features = false, features = ["custom-circuit"] }
```

**Result:** Fast proofs (10-100ms), requires Phase 2+ implementation.

### Production - Hybrid

```toml
# Support both backends
zk = { path = "../zk", features = ["sp1", "custom-circuit"] }
```

**Result:**
- Use zkVM during development/testing
- Use custom circuit for production actions
- Fallback between them based on load

## Checking Compiled Code

### View Dependencies

```bash
# See what dependencies are included
cargo tree -p zk

# With sp1 feature
cargo tree -p zk --features sp1

# Custom circuit only
cargo tree -p zk --no-default-features --features custom-circuit
```

### Check Compiled Modules

```bash
# See what modules are included in the binary
cargo build -p zk --message-format=json | \
  jq -r 'select(.reason == "compiler-artifact") | .target.src_path'
```

### Binary Size Impact

```bash
# Default (zkvm stub)
cargo build --release -p zk
ls -lh target/release/deps/libzk-*.rlib

# With SP1 (when implemented)
cargo build --release -p zk --features sp1
ls -lh target/release/deps/libzk-*.rlib

# Compare sizes
```

## Common Issues

### Issue: Custom Circuit Code Compiles Unexpectedly

**Problem:** You have `default-features = true` and custom-circuit code is compiling.

**Solution:**
```toml
# Wrong
zk = { path = "../zk", features = ["custom-circuit"] }
# This enables BOTH zkvm (default) and custom-circuit

# Right
zk = { path = "../zk", default-features = false, features = ["custom-circuit"] }
# This only enables custom-circuit
```

### Issue: Missing ProofBackend Variant

**Problem:** Compilation error about missing enum variant.

**Cause:** `ProofBackend` enum uses `#[cfg(feature = "...")]` on variants.

**Solution:** Ensure at least one feature is enabled:
```bash
cargo build -p zk  # Has default feature (zkvm)
```

### Issue: Want to Test Both Backends

**Solution:**
```bash
# Test default zkvm
cargo test -p zk

# Test custom circuit
cargo test -p zk --no-default-features --features custom-circuit

# Test both
cargo test -p zk --features custom-circuit
```

## See Also

- [zk/README.md](../crates/zk/README.md) - ZK crate documentation
- [Cargo Features](https://doc.rust-lang.org/cargo/reference/features.html) - Official Cargo documentation
- [State Delta Architecture](./state-delta-architecture.md) - Design rationale
