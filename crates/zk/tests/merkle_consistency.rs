//! Test Merkle tree consistency between native and circuit implementations.

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_relations::r1cs::ConstraintSystem;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::R1CSVar;

use zk::circuit::merkle::{SparseMerkleTree, hash_many};
use zk::circuit::gadgets::{poseidon_hash_many_gadget, verify_merkle_path_gadget};

#[test]
fn test_merkle_tree_simple() {
    // Create a simple tree with one leaf
    let mut tree = SparseMerkleTree::new(4); // depth 4 = 16 max leaves

    // Create leaf data (simulating serialized actor)
    let leaf_data = vec![
        Fp254::from(100u64),  // x
        Fp254::from(200u64),  // y
        Fp254::from(50u64),   // health
    ];

    // Hash the leaf data
    let leaf_hash = hash_many(&leaf_data).expect("Failed to hash leaf data");
    println!("Leaf hash: {:?}", leaf_hash);

    // Insert into tree
    tree.insert(0, leaf_hash);

    // Get root
    let root = tree.root().expect("Failed to get root");
    println!("Tree root: {:?}", root);

    // Generate proof
    let proof = tree.prove(0).expect("Failed to generate proof");
    println!("Proof siblings: {} elements", proof.siblings.len());
    println!("Proof depth: {}", proof.path_bits.len());
    for (i, (&sibling, &direction)) in proof.siblings.iter().zip(proof.directions.iter()).enumerate() {
        println!("  Level {}: sibling={:?}, direction={}", i, sibling, direction);
    }

    // Now verify in circuit
    let cs = ConstraintSystem::<Fp254>::new_ref();

    // Allocate leaf data as witnesses
    let leaf_data_vars: Vec<FpVar<Fp254>> = leaf_data
        .iter()
        .map(|&val| FpVar::new_witness(cs.clone(), || Ok(val)).unwrap())
        .collect();

    // Compute leaf hash in circuit
    let leaf_hash_var = poseidon_hash_many_gadget(&leaf_data_vars).expect("Failed to hash in circuit");
    let circuit_leaf_hash = leaf_hash_var.value().expect("Failed to get leaf hash value");

    println!("Circuit leaf hash: {:?}", circuit_leaf_hash);
    println!("Leaf hashes match: {}", leaf_hash == circuit_leaf_hash);

    // Allocate proof path
    let path_vars: Vec<(FpVar<Fp254>, Boolean<Fp254>)> = proof.siblings
        .iter()
        .zip(proof.directions.iter())
        .map(|(&sibling, &direction)| {
            let sibling_var = FpVar::new_witness(cs.clone(), || Ok(sibling)).unwrap();
            let direction_var = Boolean::new_witness(cs.clone(), || Ok(direction)).unwrap();
            (sibling_var, direction_var)
        })
        .collect();

    // Allocate expected root
    let root_var = FpVar::new_input(cs.clone(), || Ok(root)).unwrap();

    // Manually compute root to debug
    println!("\nManual root computation:");
    let mut current = leaf_hash_var.clone();
    println!("  Start (leaf): {:?}", current.value().unwrap());
    for (i, (sibling_var, direction_var)) in path_vars.iter().enumerate() {
        let sibling_val = sibling_var.value().unwrap();
        let dir_val = direction_var.value().unwrap();
        println!("  Level {}: sibling={:?}, direction={}", i, sibling_val, dir_val);

        // Compute next level
        use zk::circuit::gadgets::poseidon_hash_two_gadget;
        let next = if dir_val {
            poseidon_hash_two_gadget(&current, sibling_var).unwrap()
        } else {
            poseidon_hash_two_gadget(sibling_var, &current).unwrap()
        };
        let next_val = next.value().unwrap();
        println!("    -> hash result: {:?}", next_val);
        current = next;
    }
    println!("  Final (computed root): {:?}", current.value().unwrap());
    println!("  Expected root: {:?}", root);

    // Verify path in circuit
    let verify_result = verify_merkle_path_gadget(&leaf_hash_var, &path_vars, &root_var);

    println!("\nVerification result: {:?}", verify_result);
    println!("Constraints satisfied: {}", cs.is_satisfied().unwrap());
    println!("Number of constraints: {}", cs.num_constraints());

    assert!(verify_result.is_ok(), "Merkle path verification failed!");
    assert!(cs.is_satisfied().unwrap(), "Constraints not satisfied!");
}

#[test]
fn test_merkle_tree_multiple_leaves() {
    // Create a tree with multiple leaves
    let mut tree = SparseMerkleTree::new(4);

    // Add 3 leaves
    let leaves = vec![
        vec![Fp254::from(1u64), Fp254::from(2u64)],
        vec![Fp254::from(3u64), Fp254::from(4u64)],
        vec![Fp254::from(5u64), Fp254::from(6u64)],
    ];

    for (i, leaf_data) in leaves.iter().enumerate() {
        let leaf_hash = hash_many(leaf_data).expect("Failed to hash");
        tree.insert(i as u32, leaf_hash);
    }

    let root = tree.root().expect("Failed to get root");
    println!("Root with 3 leaves: {:?}", root);

    // Verify leaf 1
    let proof = tree.prove(1).expect("Failed to generate proof for leaf 1");

    let cs = ConstraintSystem::<Fp254>::new_ref();

    // Compute leaf 1 hash in circuit
    let leaf1_data_vars: Vec<FpVar<Fp254>> = leaves[1]
        .iter()
        .map(|&val| FpVar::new_witness(cs.clone(), || Ok(val)).unwrap())
        .collect();

    let leaf1_hash_var = poseidon_hash_many_gadget(&leaf1_data_vars).expect("Failed to hash");

    // Allocate proof
    let path_vars: Vec<(FpVar<Fp254>, Boolean<Fp254>)> = proof.siblings
        .iter()
        .zip(proof.directions.iter())
        .map(|(&sibling, &direction)| {
            let sibling_var = FpVar::new_witness(cs.clone(), || Ok(sibling)).unwrap();
            let direction_var = Boolean::new_witness(cs.clone(), || Ok(direction)).unwrap();
            (sibling_var, direction_var)
        })
        .collect();

    let root_var = FpVar::new_input(cs.clone(), || Ok(root)).unwrap();

    // Verify
    let verify_result = verify_merkle_path_gadget(&leaf1_hash_var, &path_vars, &root_var);

    println!("Verification for leaf 1: {:?}", verify_result);
    println!("Constraints satisfied: {}", cs.is_satisfied().unwrap());

    assert!(verify_result.is_ok(), "Merkle path verification failed for leaf 1!");
    assert!(cs.is_satisfied().unwrap(), "Constraints not satisfied!");
}
