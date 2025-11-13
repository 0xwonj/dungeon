# Fixes Applied During Arkworks Investigation

## Summary

While investigating the failing Groth16 proof verification test, several important bugs were found and fixed.

## Fixes Applied

### 1. Hash Function Mismatch (CRITICAL FIX)
**File**: `crates/zk/src/circuit/commitment.rs`
**Issue**: `hash_two()` absorbed inputs together while gadget absorbed separately
**Fix**: Changed to absorb inputs separately to match `poseidon_hash_two_gadget()`

### 2. Dummy Circuit Merkle Depth
**File**: `crates/zk/src/circuit/game_transition.rs`
**Issue**: Dummy used depth 4, real circuits use depth 10
**Fix**: Changed dummy to use depth 10

### 3. Dummy Circuit Position Delta
**File**: `crates/zk/src/circuit/game_transition.rs`
**Issue**: Dummy had delta (0,0) which violates Move action constraint
**Fix**: Changed to (0,1) for valid North movement

### 4. Dummy Circuit Merkle Roots
**File**: `crates/zk/src/circuit/game_transition.rs`
**Issue**: Dummy used zeros instead of actual computed roots
**Fix**: Compute actual roots from witness data

### 5. Dummy Circuit Merkle Paths
**File**: `crates/zk/src/circuit/game_transition.rs`
**Issue**: Dummy used all-zero paths instead of actual paths
**Fix**: Generate real Merkle paths using tree.prove()

### 6. Test Root Computation
**File**: `crates/zk/tests/arkworks_game_transition.rs`
**Issue**: Used `compute_state_root()` (combined entity+turn)
**Fix**: Changed to `build_entity_tree().root()` (entity-only for Phase 1)

## Test Status

After fixes:
- ✅ Dummy circuit constraints satisfied
- ✅ Real circuit constraints satisfied
- ✅ Merkle paths valid
- ❌ Groth16 verification still fails (unresolved - see KNOWN_ISSUES.md)
