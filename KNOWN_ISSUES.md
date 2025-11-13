# Known Issues

## Arkworks Groth16 Proof Verification Failure

**Status**: UNRESOLVED
**Test**: `crates/zk/tests/arkworks_game_transition.rs::test_move_action_proof_verification`
**Severity**: High (blocks production Arkworks backend)

### Summary

Groth16 proof generation succeeds but verification fails when using the GameTransitionCircuit with real game state values.

### Investigation Details

**VERIFIED WORKING:**
- ✅ Hash functions match between native and circuit implementations
- ✅ Correct roots used (entity tree roots, not combined state roots)
- ✅ Dummy circuit has correct Merkle depth (10 levels)
- ✅ Dummy circuit witness data is consistent with position_delta
- ✅ Dummy circuit Merkle paths are valid
- ✅ Real circuit constraints are satisfied (6348 constraints)
- ✅ Real circuit Merkle paths are valid
- ✅ Public input order matches circuit allocation order
- ✅ Constraint counts match between dummy and real circuits

**STILL FAILS:**
- ❌ Proof verification returns `false`

### Root Cause Analysis

The issue appears to be a fundamental incompatibility between:
1. Keys generated with dummy circuit (simple/default witness values)
2. Proof generated with real circuit (actual game state values)

Even though both circuits have identical structure (same constraint count, same public inputs, same Merkle depth), Groth16 verification fails.

### Possible Causes

1. **Subtle circuit structure difference**: The constraint count matches, but there may be a difference in constraint formulation or variable ordering that isn't captured by the count alone.

2. **Arkworks circuit parameter handling**: Arkworks may encode circuit-specific information during key generation that makes keys incompatible with different witness values.

3. **Missing or incorrect constraint**: There may be a constraint that appears satisfied during constraint generation but fails during verification due to field arithmetic edge cases.

### Workarounds

1. **Use RISC0 backend**: The RISC0 zkVM backend works correctly and is the recommended production backend.
2. **Use stub backend**: For testing, the stub backend provides instant proof generation without cryptographic guarantees.

### Next Steps

1. Deep dive into Groth16 circuit compatibility in arkworks
2. Compare with working HelloWorldCircuit to identify structural differences
3. Consider implementing a simpler test circuit to isolate the issue
4. Investigate arkworks documentation for circuit parameter requirements

### Files Involved

- `crates/zk/src/circuit/game_transition.rs` - Main circuit implementation
- `crates/zk/src/circuit/gadgets.rs` - R1CS gadgets
- `crates/zk/src/circuit/merkle.rs` - Merkle tree implementation
- `crates/zk/src/circuit/commitment.rs` - Poseidon hash functions
- `crates/zk/tests/arkworks_game_transition.rs` - Test file

### Debug Tests Created

Several debug tests were created during investigation (can be removed):
- `debug_dummy_circuit.rs` - Checks dummy circuit constraint satisfaction
- `debug_real_circuit_constraints.rs` - Checks real circuit constraint satisfaction
- `debug_dummy_merkle.rs` - Validates dummy circuit Merkle paths
- `debug_real_merkle_paths.rs` - Validates real circuit Merkle paths
- `debug_key_circuit_mismatch.rs` - Tests key/circuit compatibility
- `debug_constraint_counts.rs` - Compares constraint counts
- `debug_helloworld_dummy.rs` - Tests HelloWorldCircuit dummy behavior
