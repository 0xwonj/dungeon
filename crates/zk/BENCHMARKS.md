# Arkworks Benchmark Guide

## Running Benchmarks

### Basic Usage
```bash
# Run all benchmarks with arkworks feature (IMPORTANT: use --no-default-features)
cargo bench --package zk --no-default-features --features arkworks

# Run specific benchmark
cargo bench --package zk --no-default-features --features arkworks merkle_tree

# Run with stub backend (fast, for comparison)
cargo bench --package zk --no-default-features --features stub
```

### Available Benchmarks

1. **merkle_tree** - Merkle tree construction time
   - Tests with 1, 5, 10, 20, 50 actors
   - Measures tree building performance

2. **state_root** - State root computation
   - Hash all entities into a single root
   - Critical for proof generation

3. **witness_generation** - Convert StateDelta to witnesses
   - Tests with 1, 5, 10 actors
   - Measures sparse witness extraction

4. **state_transition** - Full StateTransition::from_delta()
   - End-to-end pipeline benchmark
   - Includes tree building + witness generation

5. **poseidon_hash** - Cryptographic hash performance
   - hash_one: Single field element
   - hash_two: Two field elements (Merkle nodes)

6. **merkle_proof** - Proof generation for single entity
   - Tests with 5, 10, 20 actors
   - Measures authentication path creation

### Understanding Results

Criterion outputs:
- **time**: Mean execution time
- **thrpt**: Throughput (operations/second)
- **change**: Performance change vs. previous run

### HTML Reports

After running benchmarks:
```bash
# Open the HTML report
open target/criterion/report/index.html

# Or navigate to specific benchmark
open target/criterion/merkle_tree/report/index.html
```

### Comparing Backends

```bash
# Benchmark RISC0 (for comparison)
cargo bench --package zk --no-default-features --features risc0

# Benchmark stub (baseline)
cargo bench --package zk --no-default-features --features stub

# Benchmark arkworks
cargo bench --package zk --no-default-features --features arkworks
```

### Performance Tips

1. **Run on isolated system**: Close other applications
2. **Disable frequency scaling**: For consistent results
3. **Multiple runs**: Criterion automatically does this
4. **Baseline**: Save a baseline for comparison
   ```bash
   cargo bench --package zk --no-default-features --features arkworks -- --save-baseline main
   cargo bench --package zk --no-default-features --features arkworks -- --baseline main
   ```

### Expected Performance

Arkworks should be:
- **Faster witness generation** than zkVM (no ELF overhead)
- **Faster proof generation** (optimized circuits vs. zkVM)
- **Slower than stub** (stub has no cryptography)

Typical ranges (on modern hardware):
- Merkle tree (10 actors): ~100-500 µs
- State root: ~200-800 µs
- Witness generation: ~500 µs - 2 ms
- Poseidon hash: ~10-50 µs

## Quick Start

```bash
# 1. Run benchmarks
cargo bench --package zk --no-default-features --features arkworks

# 2. View results in terminal (auto-displayed)

# 3. Open HTML report for detailed graphs
xdg-open target/criterion/report/index.html  # Linux
open target/criterion/report/index.html      # macOS
start target/criterion/report/index.html     # Windows
```

## Interpreting Results

Example output:
```
merkle_tree/5_actors    time:   [245.23 µs 248.91 µs 253.12 µs]
                        change: [-2.3421% +0.1234% +2.8901%] (p = 0.89 > 0.05)
```

- **time**: Mean is 248.91 µs, confidence interval [245.23, 253.12]
- **change**: Performance is within ±2.89% of previous run
- **p > 0.05**: No significant performance change detected
