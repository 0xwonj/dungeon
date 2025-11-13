//! Debug test for dummy circuit Merkle path verification

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use zk::circuit::game_transition::GameTransitionCircuit;
use zk::circuit::merkle::{hash_many, SparseMerkleTree};

#[test]
fn test_dummy_circuit_merkle_paths() {
    let dummy = GameTransitionCircuit::dummy();

    // Extract witness data
    let witnesses = dummy.witnesses.as_ref().unwrap();
    let witness = &witnesses.entities[0];

    println!("=== Dummy Circuit Witness ===");
    println!("Entity ID: {}", witness.id.0);
    println!("Before data: {:?}", witness.before_data);
    println!("After data: {:?}", witness.after_data);
    println!("Before path siblings: {} (all zeros? {})",
        witness.before_path.siblings.len(),
        witness.before_path.siblings.iter().all(|s| *s == Fp254::from(0u64))
    );

    // Compute leaf hashes
    let before_leaf = hash_many(&witness.before_data).unwrap();
    let after_leaf = hash_many(&witness.after_data).unwrap();

    println!("\n=== Leaf Hashes ===");
    println!("Before leaf: {}", before_leaf);
    println!("After leaf: {}", after_leaf);

    // Build trees and compute expected roots
    let mut before_tree = SparseMerkleTree::new(10);
    before_tree.insert(0, before_leaf);
    let expected_before_root = before_tree.root().unwrap();

    let mut after_tree = SparseMerkleTree::new(10);
    after_tree.insert(0, after_leaf);
    let expected_after_root = after_tree.root().unwrap();

    println!("\n=== Expected Roots ===");
    println!("Expected before root: {}", expected_before_root);
    println!("Expected after root: {}", expected_after_root);

    println!("\n=== Dummy Circuit Roots ===");
    println!("Dummy before root: {}", dummy.before_root.unwrap());
    println!("Dummy after root: {}", dummy.after_root.unwrap());

    // Verify they match
    assert_eq!(dummy.before_root.unwrap(), expected_before_root,
        "Dummy before_root doesn't match expected!");
    assert_eq!(dummy.after_root.unwrap(), expected_after_root,
        "Dummy after_root doesn't match expected!");

    // Verify Merkle paths
    println!("\n=== Verifying Merkle Paths ===");
    let before_valid = SparseMerkleTree::verify(
        before_leaf,
        &witness.before_path,
        expected_before_root
    ).unwrap();
    println!("Before path valid: {}", before_valid);
    assert!(before_valid, "Before Merkle path is invalid!");

    let after_valid = SparseMerkleTree::verify(
        after_leaf,
        &witness.after_path,
        expected_after_root
    ).unwrap();
    println!("After path valid: {}", after_valid);
    assert!(after_valid, "After Merkle path is invalid!");

    println!("\n✓ All Merkle paths are valid!");
}
