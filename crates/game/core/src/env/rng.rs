//! RNG oracle for deterministic random number generation.
//!
//! This module provides a trait-based RNG system that ensures deterministic
//! random number generation for game mechanics like hit rolls, damage variance,
//! and procedural generation.
//!
//! # Determinism
//!
//! All RNG implementations must be deterministic: given the same seed,
//! they must produce the same sequence of random numbers. This is critical
//! for ZK proofs and game replay.

/// RNG oracle for deterministic random number generation.
///
/// Implementations must be deterministic and produce the same values
/// given the same seed.
pub trait RngOracle: Send + Sync {
    /// Generate a random u32 value from a seed.
    fn next_u32(&self, seed: u64) -> u32;

    /// Roll a d100 (1-100 inclusive).
    ///
    /// Common for percentage-based mechanics like hit chance.
    fn roll_d100(&self, seed: u64) -> u32 {
        (self.next_u32(seed) % 100) + 1
    }

    /// Roll a die with N sides (1-N inclusive).
    fn roll_die(&self, seed: u64, sides: u32) -> u32 {
        (self.next_u32(seed) % sides) + 1
    }

    /// Generate a random value in range [min, max] inclusive.
    fn range(&self, seed: u64, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        let range = max - min + 1;
        min + (self.next_u32(seed) % range)
    }
}

/// PCG random number generator (Permuted Congruential Generator).
///
/// PCG is a family of simple, fast, space-efficient RNGs with excellent
/// statistical quality. This implementation uses PCG-XSH-RR, which produces
/// 32-bit output from 64-bit state.
///
/// # Properties
///
/// - **Deterministic**: Same seed always produces same output
/// - **Fast**: Single multiply + xorshift + rotate
/// - **Small state**: Only 64 bits
/// - **Good quality**: Passes statistical tests (PractRand, TestU01)
/// - **ZK-friendly**: Simple operations, no branches
///
/// # References
///
/// - PCG paper: <https://www.pcg-random.org/>
/// - Implementation based on PCG-XSH-RR variant
#[derive(Clone, Copy, Debug, Default)]
pub struct PcgRng;

impl PcgRng {
    /// PCG multiplier constant.
    const MULTIPLIER: u64 = 6364136223846793005;

    /// PCG increment constant.
    const INCREMENT: u64 = 1442695040888963407;

    /// Advance the PCG state by one step.
    ///
    /// Uses LCG (Linear Congruential Generator) formula:
    /// `state' = (state × multiplier + increment) mod 2^64`
    #[inline]
    fn pcg_step(state: u64) -> u64 {
        state
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
    }

    /// PCG output function using XSH-RR (xorshift high, random rotate).
    ///
    /// This is where the "permutation" happens - transforms the LCG state
    /// into high-quality random output.
    #[inline]
    fn pcg_output(state: u64) -> u32 {
        // XOR upper bits with lower bits, shift right
        let xorshifted = (((state >> 18) ^ state) >> 27) as u32;

        // Use upper bits to determine rotation amount
        let rot = (state >> 59) as u32;

        // Random rotation provides the final permutation
        xorshifted.rotate_right(rot)
    }
}

impl RngOracle for PcgRng {
    fn next_u32(&self, seed: u64) -> u32 {
        let state = Self::pcg_step(seed);
        Self::pcg_output(state)
    }
}

/// Compute deterministic seed from game state components.
///
/// Combines multiple entropy sources to ensure unique seeds for each
/// random event in the game.
///
/// # Arguments
///
/// * `game_seed` - Base seed set at game start (for replay/determinism)
/// * `nonce` - Action sequence number (increments each action)
/// * `actor_id` - Entity performing the action
/// * `context` - Additional context for multiple rolls in same action
///
/// # Context Values
///
/// Use different context values when the same action needs multiple
/// independent random rolls:
///
/// - `0`: Primary roll (e.g., hit check)
/// - `1`: Secondary roll (e.g., damage variance)
/// - `2`: Tertiary roll (e.g., critical check)
/// - etc.
pub fn compute_seed(game_seed: u64, nonce: u64, actor_id: u32, context: u32) -> u64 {
    // Mix all inputs using simple hash combiners
    // These constants are based on SplitMix64 and FxHash multipliers
    let mut hash = game_seed;

    // Mix in nonce (action sequence)
    hash ^= nonce.wrapping_mul(0x9e3779b97f4a7c15);

    // Mix in actor_id
    hash ^= (actor_id as u64).wrapping_mul(0x517cc1b727220a95);

    // Mix in context
    hash ^= (context as u64).wrapping_mul(0x85ebca6b);

    // Final avalanche step
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51afd7ed558ccd);
    hash ^= hash >> 33;

    hash
}
