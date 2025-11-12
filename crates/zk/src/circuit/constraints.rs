//! R1CS constraint generation for hello world circuit.

#![allow(dead_code)]

#[cfg(feature = "arkworks")]
use ark_bn254::Fr as Fp254;
#[cfg(feature = "arkworks")]
use ark_r1cs_std::alloc::AllocVar;
#[cfg(feature = "arkworks")]
use ark_r1cs_std::eq::EqGadget;
#[cfg(feature = "arkworks")]
use ark_r1cs_std::fields::fp::FpVar;
#[cfg(feature = "arkworks")]
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

#[cfg(feature = "arkworks")]
use super::merkle::MerklePath;

#[cfg(feature = "arkworks")]
/// Minimal hello world circuit demonstrating Groth16 infrastructure.
/// Trivial constraint: leaf equals root.
#[derive(Clone)]
pub struct HelloWorldCircuit {
    pub root: Option<Fp254>,
    pub leaf: Option<Fp254>,
    pub path: Option<MerklePath>,
}

#[cfg(feature = "arkworks")]
impl HelloWorldCircuit {
    pub fn new(root: Fp254, leaf: Fp254, path: MerklePath) -> Self {
        Self {
            root: Some(root),
            leaf: Some(leaf),
            path: Some(path),
        }
    }

    pub fn dummy() -> Self {
        Self {
            root: None,
            leaf: None,
            path: None,
        }
    }
}

#[cfg(feature = "arkworks")]
impl ConstraintSynthesizer<Fp254> for HelloWorldCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fp254>) -> Result<(), SynthesisError> {
        let root_var = FpVar::new_input(cs.clone(), || {
            self.root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let leaf_var =
            FpVar::new_witness(cs, || self.leaf.ok_or(SynthesisError::AssignmentMissing))?;

        // Trivial constraint: leaf == root
        leaf_var.enforce_equal(&root_var)?;

        Ok(())
    }
}
