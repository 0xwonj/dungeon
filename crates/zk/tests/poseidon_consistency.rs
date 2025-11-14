//! Test Poseidon hash consistency between native and circuit implementations.

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_r1cs_std::R1CSVar;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_relations::r1cs::ConstraintSystem;

use zk::circuit::commitment::{get_poseidon_config, hash_one, hash_two};
use zk::circuit::gadgets::{poseidon_hash_one_gadget, poseidon_hash_two_gadget};

#[test]
fn test_poseidon_hash_one_consistency() {
    let input = Fp254::from(42u64);

    // Compute hash natively
    let native_hash = hash_one(input).expect("Native hash failed");

    println!("Native hash of 42: {:?}", native_hash);

    // Compute hash in circuit
    let cs = ConstraintSystem::<Fp254>::new_ref();
    let input_var = FpVar::new_witness(cs.clone(), || Ok(input)).unwrap();
    let circuit_hash_var = poseidon_hash_one_gadget(&input_var).expect("Circuit hash failed");
    let circuit_hash = circuit_hash_var
        .value()
        .expect("Failed to get circuit hash value");

    println!("Circuit hash of 42: {:?}", circuit_hash);
    println!("Hashes match: {}", native_hash == circuit_hash);
    println!("Constraints satisfied: {}", cs.is_satisfied().unwrap());
    println!("Number of constraints: {}", cs.num_constraints());

    assert_eq!(
        native_hash, circuit_hash,
        "Native and circuit hashes must match!"
    );
}

#[test]
fn test_poseidon_hash_two_consistency() {
    let left = Fp254::from(10u64);
    let right = Fp254::from(20u64);

    // Compute hash natively
    let native_hash = hash_two(left, right).expect("Native hash failed");

    println!("Native hash of (10, 20): {:?}", native_hash);

    // Compute hash in circuit
    let cs = ConstraintSystem::<Fp254>::new_ref();
    let left_var = FpVar::new_witness(cs.clone(), || Ok(left)).unwrap();
    let right_var = FpVar::new_witness(cs.clone(), || Ok(right)).unwrap();
    let circuit_hash_var =
        poseidon_hash_two_gadget(&left_var, &right_var).expect("Circuit hash failed");
    let circuit_hash = circuit_hash_var
        .value()
        .expect("Failed to get circuit hash value");

    println!("Circuit hash of (10, 20): {:?}", circuit_hash);
    println!("Hashes match: {}", native_hash == circuit_hash);
    println!("Constraints satisfied: {}", cs.is_satisfied().unwrap());
    println!("Number of constraints: {}", cs.num_constraints());

    assert_eq!(
        native_hash, circuit_hash,
        "Native and circuit hashes must match!"
    );
}

#[test]
fn test_poseidon_config_parameters() {
    let config = get_poseidon_config();

    println!("Poseidon Config:");
    println!("  Full rounds: {}", config.full_rounds);
    println!("  Partial rounds: {}", config.partial_rounds);
    println!("  Alpha: {}", config.alpha);
    println!("  Rate: {}", config.rate);
    println!("  Capacity: {}", config.capacity);
    println!("  ARK (round constants): {} rounds", config.ark.len());
    println!("  MDS matrix: {}x{}", config.mds.len(), config.mds[0].len());
}
