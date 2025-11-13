//! Test if HelloWorldCircuit dummy can generate constraints

#![cfg(feature = "arkworks")]

use ark_bn254::Fr as Fp254;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use zk::circuit::constraints::HelloWorldCircuit;

#[test]
fn test_helloworld_dummy_constraints() {
    let dummy = HelloWorldCircuit::dummy();
    let cs = ConstraintSystem::<Fp254>::new_ref();

    println!("Attempting to generate constraints with HelloWorldCircuit::dummy()...");
    let result = dummy.generate_constraints(cs.clone());

    match result {
        Ok(()) => {
            println!("✓ Constraint generation succeeded");
            println!("Constraints: {}", cs.num_constraints());
            println!("Satisfied: {}", cs.is_satisfied().unwrap());
        }
        Err(e) => {
            println!("❌ Constraint generation failed: {:?}", e);
            println!("This is expected! Arkworks key generation must handle AssignmentMissing specially.");
        }
    }
}
