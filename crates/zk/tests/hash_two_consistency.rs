//! Test hash_two consistency.

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_r1cs_std::R1CSVar;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_relations::r1cs::ConstraintSystem;

use zk::circuit::commitment::hash_two;
use zk::circuit::gadgets::poseidon_hash_two_gadget;

#[test]
fn test_hash_two_same_order() {
    let left = Fp254::from(100u64);
    let right = Fp254::from(0u64); // Empty sibling

    // Native hash
    let native = hash_two(left, right).expect("Native hash failed");
    println!("Native hash_two(100, 0): {:?}", native);

    // Circuit hash
    let cs = ConstraintSystem::<Fp254>::new_ref();
    let left_var = FpVar::new_witness(cs.clone(), || Ok(left)).unwrap();
    let right_var = FpVar::new_witness(cs.clone(), || Ok(right)).unwrap();
    let circuit_var = poseidon_hash_two_gadget(&left_var, &right_var).unwrap();
    let circuit = circuit_var.value().unwrap();

    println!("Circuit hash_two(100, 0): {:?}", circuit);
    println!("Match: {}", native == circuit);

    assert_eq!(native, circuit, "hash_two results must match!");
}

#[test]
fn test_hash_two_reversed_order() {
    let left = Fp254::from(0u64); // Empty sibling
    let right = Fp254::from(100u64);

    // Native hash
    let native = hash_two(left, right).expect("Native hash failed");
    println!("Native hash_two(0, 100): {:?}", native);

    // Circuit hash
    let cs = ConstraintSystem::<Fp254>::new_ref();
    let left_var = FpVar::new_witness(cs.clone(), || Ok(left)).unwrap();
    let right_var = FpVar::new_witness(cs.clone(), || Ok(right)).unwrap();
    let circuit_var = poseidon_hash_two_gadget(&left_var, &right_var).unwrap();
    let circuit = circuit_var.value().unwrap();

    println!("Circuit hash_two(0, 100): {:?}", circuit);
    println!("Match: {}", native == circuit);

    assert_eq!(native, circuit, "hash_two results must match!");
}
