//! Test hash_many consistency with 5 elements (actor serialization size).

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_r1cs_std::R1CSVar;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_relations::r1cs::ConstraintSystem;

use zk::circuit::gadgets::poseidon_hash_many_gadget;
use zk::circuit::merkle::hash_many;

#[test]
fn test_hash_five_elements_consistency() {
    // Test with 5 elements (same as actor serialization)
    let inputs = vec![
        Fp254::from(0u64),   // entity ID
        Fp254::from(5u64),   // x
        Fp254::from(5u64),   // y
        Fp254::from(105u64), // HP
        Fp254::from(105u64), // max HP
    ];

    // Native hash
    let native = hash_many(&inputs).expect("Native hash failed");
    println!("Native hash_many(5 elements): {:?}", native);

    // Circuit hash
    let cs = ConstraintSystem::<Fp254>::new_ref();
    let input_vars: Vec<FpVar<Fp254>> = inputs
        .iter()
        .map(|&val| FpVar::new_witness(cs.clone(), || Ok(val)).unwrap())
        .collect();
    let circuit_var = poseidon_hash_many_gadget(&input_vars).unwrap();
    let circuit = circuit_var.value().unwrap();

    println!("Circuit hash_many(5 elements): {:?}", circuit);
    println!("Match: {}", native == circuit);
    println!("Constraints satisfied: {}", cs.is_satisfied().unwrap());

    assert_eq!(
        native, circuit,
        "hash_many results for 5 elements must match!"
    );
}
