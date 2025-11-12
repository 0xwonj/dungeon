//! Hello World integration test for Arkworks prover.

#[cfg(feature = "arkworks")]
#[test]
fn test_arkworks_hello_world_proof() {
    use ark_bn254::Fr as Fp254;
    use ark_std::test_rng;
    use zk::circuit::{constraints, groth16, merkle};

    // For the hello world circuit, the constraint is simply: leaf == root (trivial)
    // So we use the same value for both
    let value = Fp254::from(42u64);
    let root = value;
    let leaf = value;

    // Create a dummy Merkle path (not actually used in the trivial circuit)
    let path = merkle::MerklePath {
        siblings: vec![Fp254::from(0u64); 4],
        directions: vec![false; 4],
    };

    // Generate Groth16 keys and proof
    let mut rng = test_rng();
    let dummy_circuit = constraints::HelloWorldCircuit::dummy();
    let keys =
        groth16::Groth16Keys::generate(dummy_circuit, &mut rng).expect("Failed to generate keys");
    let circuit = constraints::HelloWorldCircuit::new(root, leaf, path);
    let proof = groth16::prove(circuit, &keys, &mut rng).expect("Failed to prove");

    // Verify Groth16 proof
    let result = groth16::verify(&proof, &[root], &keys.verifying_key).expect("Failed to verify");
    assert!(result);

    println!("✅ Hello World proof verified!");
}
