//! R1CS gadgets for game state verification.
//!
//! This module provides reusable constraint gadgets for:
//! - Poseidon hashing (circuit-friendly hash function)
//! - Merkle path verification
//! - Range checks and bounds validation
//! - Arithmetic operations with overflow protection

#![allow(dead_code)]

use ark_bn254::Fr as Fp254;
use ark_relations::r1cs::SynthesisError;
use ark_r1cs_std::fields::{fp::FpVar, FieldVar};
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::cmp::CmpGadget;
use ark_r1cs_std::select::CondSelectGadget;
use ark_r1cs_std::R1CSVar;
use ark_crypto_primitives::sponge::poseidon::PoseidonSponge;
use ark_crypto_primitives::sponge::CryptographicSponge;
use ark_r1cs_std::alloc::AllocVar;

use super::commitment::get_poseidon_config;

// ============================================================================
// Poseidon Hash Gadgets
// ============================================================================

/// Compute Poseidon hash of a single field element (circuit version).
///
/// This is the R1CS constraint version of `commitment::hash_one()`.
/// Uses the same parameters to ensure consistency between native and circuit hashing.
pub fn poseidon_hash_one_gadget(
    input: &FpVar<Fp254>,
) -> Result<FpVar<Fp254>, SynthesisError> {
    let params = get_poseidon_config();
    let mut sponge = PoseidonSponge::new(&params);

    // Absorb input
    let input_value = input.value()?;
    sponge.absorb(&vec![input_value]);

    // Squeeze output
    let output_value = sponge.squeeze_field_elements::<Fp254>(1)[0];

    // Create output variable (witness)
    let output_var = FpVar::new_witness(input.cs(), || Ok(output_value))?;

    // TODO: Add R1CS constraints for Poseidon permutation
    // This requires implementing the full Poseidon gadget with S-boxes and MDS matrix
    // For now, we trust the witness value (insecure for production)

    Ok(output_var)
}

/// Compute Poseidon hash of two field elements (circuit version).
///
/// This is the R1CS constraint version of `commitment::hash_two()`.
pub fn poseidon_hash_two_gadget(
    left: &FpVar<Fp254>,
    right: &FpVar<Fp254>,
) -> Result<FpVar<Fp254>, SynthesisError> {
    let params = get_poseidon_config();
    let mut sponge = PoseidonSponge::new(&params);

    // Absorb inputs
    let left_value = left.value()?;
    let right_value = right.value()?;
    sponge.absorb(&vec![left_value, right_value]);

    // Squeeze output
    let output_value = sponge.squeeze_field_elements::<Fp254>(1)[0];

    // Create output variable (witness)
    let output_var = FpVar::new_witness(left.cs(), || Ok(output_value))?;

    // TODO: Add R1CS constraints for Poseidon permutation
    // This is critical for security but complex to implement

    Ok(output_var)
}

/// Compute Poseidon hash of a variable-length input (circuit version).
pub fn poseidon_hash_many_gadget(
    inputs: &[FpVar<Fp254>],
) -> Result<FpVar<Fp254>, SynthesisError> {
    if inputs.is_empty() {
        return Err(SynthesisError::Unsatisfiable);
    }

    let params = get_poseidon_config();
    let mut sponge = PoseidonSponge::new(&params);

    // Collect all input values
    let input_values: Result<Vec<Fp254>, SynthesisError> =
        inputs.iter().map(|input| input.value()).collect();
    let input_values = input_values?;

    // Absorb all inputs
    sponge.absorb(&input_values);

    // Squeeze output
    let output_value = sponge.squeeze_field_elements::<Fp254>(1)[0];

    // Create output variable (witness)
    let output_var = FpVar::new_witness(inputs[0].cs(), || Ok(output_value))?;

    // TODO: Add R1CS constraints for Poseidon permutation

    Ok(output_var)
}

// ============================================================================
// Merkle Path Verification Gadgets
// ============================================================================

/// Verify a Merkle authentication path in-circuit.
///
/// Given a leaf, path siblings, and path directions, compute the root
/// and verify it matches the expected root.
///
/// # Arguments
///
/// * `leaf` - The leaf value to verify
/// * `path` - Vector of (sibling, direction) pairs where direction is true for right
/// * `expected_root` - The expected Merkle root
///
/// # Returns
///
/// Ok(()) if the path is valid, Err otherwise
pub fn verify_merkle_path_gadget(
    leaf: &FpVar<Fp254>,
    path: &[(FpVar<Fp254>, Boolean<Fp254>)],
    expected_root: &FpVar<Fp254>,
) -> Result<(), SynthesisError> {
    let mut current = leaf.clone();

    // Walk up the tree, hashing with siblings
    for (sibling, direction) in path {
        // If direction is true (1), current is left child: hash(current, sibling)
        // If direction is false (0), current is right child: hash(sibling, current)
        let hash_result = poseidon_hash_two_conditional_gadget(
            &current,
            sibling,
            direction,
        )?;
        current = hash_result;
    }

    // Verify computed root matches expected root
    current.enforce_equal(expected_root)?;

    Ok(())
}

/// Conditionally hash two values based on a boolean selector.
///
/// If selector is true: hash(left, right)
/// If selector is false: hash(right, left)
fn poseidon_hash_two_conditional_gadget(
    left: &FpVar<Fp254>,
    right: &FpVar<Fp254>,
    selector: &Boolean<Fp254>,
) -> Result<FpVar<Fp254>, SynthesisError> {
    // Use conditional select to swap arguments based on selector
    // first = selector ? left : right
    // second = selector ? right : left
    let first = FpVar::conditionally_select(selector, left, right)?;
    let second = FpVar::conditionally_select(selector, right, left)?;

    poseidon_hash_two_gadget(&first, &second)
}

// ============================================================================
// Range Check Gadgets
// ============================================================================

/// Verify that a field element represents a value within a given range.
///
/// This is used for validating positions, health values, etc.
pub fn range_check_gadget(
    value: &FpVar<Fp254>,
    min: Fp254,
    max: Fp254,
) -> Result<(), SynthesisError> {
    let min_var = FpVar::constant(min);
    let max_var = FpVar::constant(max);

    // Check value >= min
    let gt_min = value.is_cmp(&min_var, std::cmp::Ordering::Greater, false)?;
    let eq_min = value.is_eq(&min_var)?;
    let ge_min = &gt_min | &eq_min;
    ge_min.enforce_equal(&Boolean::TRUE)?;

    // Check value <= max
    let lt_max = value.is_cmp(&max_var, std::cmp::Ordering::Less, false)?;
    let eq_max = value.is_eq(&max_var)?;
    let le_max = &lt_max | &eq_max;
    le_max.enforce_equal(&Boolean::TRUE)?;

    Ok(())
}

/// Verify that a value is one of a set of allowed values.
///
/// Used for validating action types, directions, etc.
pub fn one_of_gadget(
    value: &FpVar<Fp254>,
    allowed: &[Fp254],
) -> Result<(), SynthesisError> {
    if allowed.is_empty() {
        return Err(SynthesisError::Unsatisfiable);
    }

    // Create disjunction: value == allowed[0] || value == allowed[1] || ...
    let mut is_valid = value.is_eq(&FpVar::constant(allowed[0]))?;

    for &allowed_value in &allowed[1..] {
        let is_equal = value.is_eq(&FpVar::constant(allowed_value))?;
        is_valid = &is_valid | &is_equal;
    }

    is_valid.enforce_equal(&Boolean::TRUE)?;

    Ok(())
}

// ============================================================================
// Position Validation Gadgets
// ============================================================================

/// Verify that a position is within map bounds.
pub fn bounds_check_gadget(
    x: &FpVar<Fp254>,
    y: &FpVar<Fp254>,
    max_x: Fp254,
    max_y: Fp254,
) -> Result<(), SynthesisError> {
    // x >= 0 (implicit since we're using unsigned representation)
    // x < max_x
    let max_x_var = FpVar::constant(max_x);
    let x_in_bounds = x.is_cmp(&max_x_var, std::cmp::Ordering::Less, false)?;
    x_in_bounds.enforce_equal(&Boolean::TRUE)?;

    // y >= 0 (implicit)
    // y < max_y
    let max_y_var = FpVar::constant(max_y);
    let y_in_bounds = y.is_cmp(&max_y_var, std::cmp::Ordering::Less, false)?;
    y_in_bounds.enforce_equal(&Boolean::TRUE)?;

    Ok(())
}

/// Verify that two positions are adjacent (within 1 tile, including diagonals).
pub fn adjacency_check_gadget(
    x1: &FpVar<Fp254>,
    y1: &FpVar<Fp254>,
    x2: &FpVar<Fp254>,
    y2: &FpVar<Fp254>,
) -> Result<(), SynthesisError> {
    // Calculate differences (absolute values)
    // |x2 - x1| <= 1 && |y2 - y1| <= 1 && (dx != 0 || dy != 0)

    // Compute dx = x2 - x1 (may be negative, but we'll check range)
    let dx = x2 - x1;
    let dy = y2 - y1;

    // Check |dx| <= 1: dx in {-1, 0, 1}
    let valid_dx_values = vec![
        Fp254::from(-1i64),
        Fp254::from(0u64),
        Fp254::from(1u64),
    ];
    one_of_gadget(&dx, &valid_dx_values)?;

    // Check |dy| <= 1: dy in {-1, 0, 1}
    let valid_dy_values = vec![
        Fp254::from(-1i64),
        Fp254::from(0u64),
        Fp254::from(1u64),
    ];
    one_of_gadget(&dy, &valid_dy_values)?;

    // Check not both zero (must actually move)
    let dx_is_zero = dx.is_eq(&FpVar::constant(Fp254::from(0u64)))?;
    let dy_is_zero = dy.is_eq(&FpVar::constant(Fp254::from(0u64)))?;
    let both_zero = &dx_is_zero & &dy_is_zero;
    both_zero.enforce_equal(&Boolean::FALSE)?;

    Ok(())
}

// ============================================================================
// Arithmetic Gadgets
// ============================================================================

/// Subtract with underflow check: ensure result is non-negative.
pub fn safe_subtract_gadget(
    minuend: &FpVar<Fp254>,
    subtrahend: &FpVar<Fp254>,
) -> Result<FpVar<Fp254>, SynthesisError> {
    // Ensure minuend >= subtrahend
    let gt = minuend.is_cmp(subtrahend, std::cmp::Ordering::Greater, false)?;
    let eq = minuend.is_eq(subtrahend)?;
    let ge = &gt | &eq;
    ge.enforce_equal(&Boolean::TRUE)?;

    // Compute difference
    let result = minuend - subtrahend;

    Ok(result)
}

/// Add with overflow check: ensure result doesn't exceed maximum.
pub fn safe_add_gadget(
    a: &FpVar<Fp254>,
    b: &FpVar<Fp254>,
    max: Fp254,
) -> Result<FpVar<Fp254>, SynthesisError> {
    let result = a + b;
    let max_var = FpVar::constant(max);

    // Ensure result <= max
    let lt_max = result.is_cmp(&max_var, std::cmp::Ordering::Less, false)?;
    let eq_max = result.is_eq(&max_var)?;
    let le_max = &lt_max | &eq_max;
    le_max.enforce_equal(&Boolean::TRUE)?;

    Ok(result)
}

/// Clamp a value to [min, max] range.
pub fn clamp_gadget(
    value: &FpVar<Fp254>,
    min: Fp254,
    max: Fp254,
) -> Result<FpVar<Fp254>, SynthesisError> {
    let min_var = FpVar::constant(min);
    let max_var = FpVar::constant(max);

    // result = max(min, min(value, max))
    let value_clamped_max = FpVar::conditionally_select(
        &value.is_cmp(&max_var, std::cmp::Ordering::Less, false)?,
        value,
        &max_var,
    )?;

    let result = FpVar::conditionally_select(
        &value_clamped_max.is_cmp(&min_var, std::cmp::Ordering::Greater, false)?,
        &value_clamped_max,
        &min_var,
    )?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn test_range_check_gadget() {
        let cs = ConstraintSystem::<Fp254>::new_ref();

        let value = FpVar::new_witness(cs.clone(), || Ok(Fp254::from(5u64))).unwrap();
        let result = range_check_gadget(&value, Fp254::from(0u64), Fp254::from(10u64));

        assert!(result.is_ok());
        assert!(cs.is_satisfied().unwrap());
    }

    #[test]
    fn test_one_of_gadget() {
        let cs = ConstraintSystem::<Fp254>::new_ref();

        let value = FpVar::new_witness(cs.clone(), || Ok(Fp254::from(2u64))).unwrap();
        let allowed = vec![Fp254::from(0u64), Fp254::from(1u64), Fp254::from(2u64)];
        let result = one_of_gadget(&value, &allowed);

        assert!(result.is_ok());
        assert!(cs.is_satisfied().unwrap());
    }
}
