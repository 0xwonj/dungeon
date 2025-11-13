//! Debug test for dummy circuit constraint satisfaction

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use zk::circuit::game_transition::GameTransitionCircuit;

#[test]
fn test_dummy_circuit_constraint_satisfaction() {
    let dummy = GameTransitionCircuit::dummy();
    let cs = ConstraintSystem::<Fp254>::new_ref();

    println!("Generating constraints for dummy circuit...");
    dummy.generate_constraints(cs.clone()).expect("Constraint generation failed");

    println!("Number of constraints: {}", cs.num_constraints());
    println!("Checking if constraints are satisfied...");

    let is_satisfied = cs.is_satisfied().unwrap();
    println!("Constraints satisfied: {}", is_satisfied);

    if !is_satisfied {
        println!("\nDEBUG: Constraints NOT satisfied!");
        println!("This means the dummy circuit has invalid witness values.");
    }

    assert!(is_satisfied, "Dummy circuit constraints must be satisfied!");
}
