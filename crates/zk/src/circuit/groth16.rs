//! Groth16 proving and verification on BN254 curve.

#![allow(dead_code)]

use crate::ProofError;

#[cfg(feature = "arkworks")]
use ark_bn254::{Bn254, Fr as Fp254};
#[cfg(feature = "arkworks")]
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
#[cfg(feature = "arkworks")]
use ark_relations::r1cs::ConstraintSynthesizer;
#[cfg(feature = "arkworks")]
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
#[cfg(feature = "arkworks")]
use ark_std::rand::RngCore;

#[cfg(feature = "arkworks")]
/// Groth16 proving and verifying keys
///
/// Contains the cryptographic keys needed for proof generation and verification.
/// Generated during a trusted setup ceremony.
#[derive(Clone)]
pub struct Groth16Keys {
    /// Proving key (secret, used by prover)
    pub proving_key: ProvingKey<Bn254>,
    /// Verifying key (public, used by verifier)
    pub verifying_key: VerifyingKey<Bn254>,
}

#[cfg(feature = "arkworks")]
impl Groth16Keys {
    /// Generate keys from a circuit (trusted setup)
    ///
    /// # Security Warning
    /// This performs a circuit-specific setup. The randomness used must be destroyed
    /// after key generation to ensure soundness. In production, use a multi-party
    /// computation ceremony or universal setup.
    ///
    /// # Arguments
    /// * `circuit` - The circuit to generate keys for (should be a dummy/template instance)
    /// * `rng` - Random number generator for the setup
    pub fn generate<C, R>(circuit: C, rng: &mut R) -> Result<Self, ProofError>
    where
        C: ConstraintSynthesizer<Fp254>,
        R: RngCore,
    {
        // Use Groth16::generate_random_parameters_with_reduction for key generation
        let params = Groth16::<Bn254>::generate_random_parameters_with_reduction(circuit, rng)
            .map_err(|e| {
                ProofError::CircuitProofError(format!("Groth16 key generation failed: {:?}", e))
            })?;

        Ok(Self {
            proving_key: params.clone(),
            verifying_key: params.vk,
        })
    }

    /// Serialize proving key to bytes
    ///
    /// Uses compressed serialization for smaller size.
    pub fn serialize_proving_key(&self) -> Result<Vec<u8>, ProofError> {
        let mut bytes = Vec::new();
        self.proving_key
            .serialize_compressed(&mut bytes)
            .map_err(|e| ProofError::SerializationError(e.to_string()))?;
        Ok(bytes)
    }

    /// Deserialize proving key from bytes
    pub fn deserialize_proving_key(bytes: &[u8]) -> Result<ProvingKey<Bn254>, ProofError> {
        ProvingKey::<Bn254>::deserialize_compressed(bytes)
            .map_err(|e| ProofError::SerializationError(e.to_string()))
    }

    /// Serialize verifying key to bytes
    ///
    /// Uses compressed serialization for smaller size.
    pub fn serialize_verifying_key(&self) -> Result<Vec<u8>, ProofError> {
        let mut bytes = Vec::new();
        self.verifying_key
            .serialize_compressed(&mut bytes)
            .map_err(|e| ProofError::SerializationError(e.to_string()))?;
        Ok(bytes)
    }

    /// Deserialize verifying key from bytes
    pub fn deserialize_verifying_key(bytes: &[u8]) -> Result<VerifyingKey<Bn254>, ProofError> {
        VerifyingKey::<Bn254>::deserialize_compressed(bytes)
            .map_err(|e| ProofError::SerializationError(e.to_string()))
    }

    /// Serialize both keys to bytes
    ///
    /// Format: [pk_len (8 bytes)][pk_bytes][vk_bytes]
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProofError> {
        let pk_bytes = self.serialize_proving_key()?;
        let vk_bytes = self.serialize_verifying_key()?;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(pk_bytes.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&pk_bytes);
        bytes.extend_from_slice(&vk_bytes);

        Ok(bytes)
    }

    /// Deserialize both keys from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProofError> {
        if bytes.len() < 8 {
            return Err(ProofError::SerializationError(
                "Invalid key bytes: too short".to_string(),
            ));
        }

        let pk_len = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + pk_len {
            return Err(ProofError::SerializationError(
                "Invalid key bytes: pk too short".to_string(),
            ));
        }

        let pk_bytes = &bytes[8..8 + pk_len];
        let vk_bytes = &bytes[8 + pk_len..];

        let proving_key = Self::deserialize_proving_key(pk_bytes)?;
        let verifying_key = Self::deserialize_verifying_key(vk_bytes)?;

        Ok(Self {
            proving_key,
            verifying_key,
        })
    }
}

#[cfg(feature = "arkworks")]
/// Generate a Groth16 proof
///
/// # Arguments
/// * `circuit` - The circuit to prove (with witness values)
/// * `keys` - The proving and verifying keys
/// * `rng` - Random number generator for proof randomness
///
/// # Returns
/// A Groth16 proof that can be verified
pub fn prove<C, R>(
    circuit: C,
    keys: &Groth16Keys,
    rng: &mut R,
) -> Result<Proof<Bn254>, ProofError>
where
    C: ConstraintSynthesizer<Fp254>,
    R: RngCore,
{
    Groth16::<Bn254>::create_random_proof_with_reduction(circuit, &keys.proving_key, rng)
        .map_err(|e| ProofError::CircuitProofError(format!("Groth16 proving failed: {:?}", e)))
}

#[cfg(feature = "arkworks")]
/// Verify a Groth16 proof
///
/// # Arguments
/// * `proof` - The proof to verify
/// * `public_inputs` - Public inputs to the circuit (e.g., Merkle root)
/// * `vk` - The verifying key
///
/// # Returns
/// `true` if the proof is valid, `false` otherwise
pub fn verify(
    proof: &Proof<Bn254>,
    public_inputs: &[Fp254],
    vk: &VerifyingKey<Bn254>,
) -> Result<bool, ProofError> {
    let pvk = prepare_verifying_key(vk);
    Groth16::<Bn254>::verify_proof(&pvk, proof, public_inputs)
        .map_err(|e| ProofError::CircuitProofError(format!("Groth16 verification failed: {:?}", e)))
}

#[cfg(feature = "arkworks")]
/// Serialize a proof to bytes
pub fn serialize_proof(proof: &Proof<Bn254>) -> Result<Vec<u8>, ProofError> {
    let mut bytes = Vec::new();
    proof
        .serialize_compressed(&mut bytes)
        .map_err(|e| ProofError::SerializationError(e.to_string()))?;
    Ok(bytes)
}

#[cfg(feature = "arkworks")]
/// Deserialize a proof from bytes
pub fn deserialize_proof(bytes: &[u8]) -> Result<Proof<Bn254>, ProofError> {
    Proof::<Bn254>::deserialize_compressed(bytes)
        .map_err(|e| ProofError::SerializationError(e.to_string()))
}

#[cfg(feature = "arkworks")]
/// Prepare verifying key for batch verification
///
/// This is an optimization for verifying multiple proofs with the same verifying key.
pub fn prepare_verifying_key(vk: &VerifyingKey<Bn254>) -> ark_groth16::PreparedVerifyingKey<Bn254> {
    ark_groth16::prepare_verifying_key(vk)
}

#[cfg(feature = "arkworks")]
/// Verify a proof using a prepared verifying key
///
/// More efficient than `verify()` when verifying multiple proofs.
pub fn verify_with_prepared_vk(
    proof: &Proof<Bn254>,
    public_inputs: &[Fp254],
    pvk: &ark_groth16::PreparedVerifyingKey<Bn254>,
) -> Result<bool, ProofError> {
    Groth16::<Bn254>::verify_proof(pvk, proof, public_inputs)
        .map_err(|e| ProofError::CircuitProofError(format!("Groth16 verification failed: {:?}", e)))
}
