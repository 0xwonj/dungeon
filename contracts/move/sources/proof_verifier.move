/// Proof Verification Module
///
/// Provides ZK proof verification for game state transitions using Groth16 SNARKs.
/// This module wraps Sui's native Groth16 verifier and provides a game-specific
/// verification interface.
///
/// # Design
/// - Uses BN254 curve (standard for RISC0 Groth16 wrapper)
/// - Public inputs schema matches RISC0 guest program output
/// - Verifying key is prepared once and reused for all verifications
module dungeon::proof_verifier {
    use sui::groth16;

    // ===== Error Codes =====

    /// Invalid proof - verification failed
    const EInvalidProof: u64 = 1;
    /// Invalid public inputs format
    const EInvalidPublicInputs: u64 = 2;
    /// Verifying key not initialized
    const EVerifyingKeyNotInitialized: u64 = 3;

    // ===== Constants =====

    /// Maximum public inputs allowed by Sui (8 field elements)
    const MAX_PUBLIC_INPUTS: u64 = 8;

    // ===== Structs =====

    /// Prepared verifying key for Groth16 proof verification
    ///
    /// This key is prepared once and stored on-chain for reuse.
    /// It corresponds to the RISC0 game guest program's circuit.
    public struct VerifyingKey has key, store {
        id: UID,
        /// Prepared verifying key for efficient verification
        prepared_vk: groth16::PreparedVerifyingKey,
        /// Version/identifier for the game circuit
        version: u64,
    }

    /// Public inputs for game state transition proof
    ///
    /// These values are committed by the ZK proof and verified on-chain.
    /// The schema must match the RISC0 guest program's public outputs.
    ///
    /// CRITICAL: actions_root fields are included to cryptographically bind
    /// the action sequence to the state transition. Without this, a player could
    /// submit fake actions to ActionLog while using different (cheating) actions
    /// in actual gameplay.
    public struct PublicInputs has copy, drop, store {
        /// Oracle data commitment (32 bytes)
        oracle_root: vector<u8>,
        /// Seed commitment for RNG (32 bytes)
        seed_commitment: vector<u8>,
        /// Previous state root (32 bytes)
        prev_state_root: vector<u8>,
        /// Previous actions root (32 bytes)
        prev_actions_root: vector<u8>,
        /// Previous nonce (8 bytes, u64)
        prev_nonce: u64,
        /// New state root (32 bytes)
        new_state_root: vector<u8>,
        /// New actions root (32 bytes)
        new_actions_root: vector<u8>,
        /// New nonce (8 bytes, u64)
        new_nonce: u64,
    }

    // ===== Admin Functions =====

    /// Initialize a new verifying key
    ///
    /// This should be called once to prepare the verifying key for the game circuit.
    /// The verifying key comes from the RISC0 Groth16 trusted setup.
    ///
    /// # Arguments
    /// * `vk_bytes` - Raw verifying key bytes from RISC0
    /// * `version` - Circuit version identifier
    /// * `ctx` - Transaction context
    ///
    /// # Returns
    /// A VerifyingKey object that should be shared or transferred to a registry
    public fun create_verifying_key(
        vk_bytes: vector<u8>,
        version: u64,
        ctx: &mut TxContext,
    ): VerifyingKey {
        let curve = groth16::bn254();
        let prepared_vk = groth16::prepare_verifying_key(&curve, &vk_bytes);

        VerifyingKey {
            id: object::new(ctx),
            prepared_vk,
            version,
        }
    }

    // ===== Public Functions =====

    /// Verify a game state transition proof
    ///
    /// Verifies that a ZK proof correctly proves the transition from one game state
    /// to another following all game rules.
    ///
    /// # Arguments
    /// * `vk` - Prepared verifying key
    /// * `public_inputs` - Public inputs for the proof
    /// * `proof_bytes` - Groth16 proof bytes
    ///
    /// # Aborts
    /// * `EInvalidProof` - If proof verification fails
    /// * `EInvalidPublicInputs` - If public inputs format is invalid
    public fun verify_game_proof(
        vk: &VerifyingKey,
        public_inputs: &PublicInputs,
        proof_bytes: vector<u8>,
    ) {
        // Serialize public inputs to field elements
        let public_inputs_bytes = serialize_public_inputs(public_inputs);

        // Create proof points from bytes
        let curve = groth16::bn254();
        let proof_points = groth16::proof_points_from_bytes(proof_bytes);
        let public_proof_inputs = groth16::public_proof_inputs_from_bytes(public_inputs_bytes);

        // Verify proof
        let valid = groth16::verify_groth16_proof(
            &curve,
            &vk.prepared_vk,
            &public_proof_inputs,
            &proof_points,
        );

        assert!(valid, EInvalidProof);
    }

    // ===== Helper Functions =====

    /// Serialize public inputs to bytes for Groth16 verification
    ///
    /// Converts PublicInputs struct to a byte array of field elements.
    /// Each field element is 32 bytes in BN254.
    ///
    /// Order matches RISC0 guest program output:
    /// 1. oracle_root (32 bytes)
    /// 2. seed_commitment (32 bytes)
    /// 3. prev_state_root (32 bytes)
    /// 4. prev_actions_root (32 bytes)
    /// 5. prev_nonce (32 bytes, u64 padded)
    /// 6. new_state_root (32 bytes)
    /// 7. new_actions_root (32 bytes)
    /// 8. new_nonce (32 bytes, u64 padded)
    fun serialize_public_inputs(inputs: &PublicInputs): vector<u8> {
        let mut result = vector::empty<u8>();

        // Oracle root (32 bytes)
        vector::append(&mut result, inputs.oracle_root);

        // Seed commitment (32 bytes)
        vector::append(&mut result, inputs.seed_commitment);

        // Previous state root (32 bytes)
        vector::append(&mut result, inputs.prev_state_root);

        // Previous actions root (32 bytes)
        vector::append(&mut result, inputs.prev_actions_root);

        // Previous nonce (u64 → 32 bytes, little-endian)
        vector::append(&mut result, u64_to_32_bytes(inputs.prev_nonce));

        // New state root (32 bytes)
        vector::append(&mut result, inputs.new_state_root);

        // New actions root (32 bytes)
        vector::append(&mut result, inputs.new_actions_root);

        // New nonce (u64 → 32 bytes, little-endian)
        vector::append(&mut result, u64_to_32_bytes(inputs.new_nonce));

        result
    }

    /// Convert u64 to 32-byte array (little-endian with zero padding)
    fun u64_to_32_bytes(value: u64): vector<u8> {
        let mut bytes = vector::empty<u8>();

        // Extract 8 bytes (little-endian)
        let mut v = value;
        let mut i = 0;
        while (i < 8) {
            vector::push_back(&mut bytes, ((v & 0xFF) as u8));
            v = v >> 8;
            i = i + 1;
        };

        // Pad with zeros to 32 bytes
        while (i < 32) {
            vector::push_back(&mut bytes, 0u8);
            i = i + 1;
        };

        bytes
    }

    // ===== View Functions =====

    /// Get the verifying key version
    public fun verifying_key_version(vk: &VerifyingKey): u64 {
        vk.version
    }

    /// Create public inputs from components
    public fun new_public_inputs(
        oracle_root: vector<u8>,
        seed_commitment: vector<u8>,
        prev_state_root: vector<u8>,
        prev_actions_root: vector<u8>,
        prev_nonce: u64,
        new_state_root: vector<u8>,
        new_actions_root: vector<u8>,
        new_nonce: u64,
    ): PublicInputs {
        PublicInputs {
            oracle_root,
            seed_commitment,
            prev_state_root,
            prev_actions_root,
            prev_nonce,
            new_state_root,
            new_actions_root,
            new_nonce,
        }
    }

    /// Borrow oracle root from public inputs
    public fun oracle_root(inputs: &PublicInputs): &vector<u8> {
        &inputs.oracle_root
    }

    /// Borrow seed commitment from public inputs
    public fun seed_commitment(inputs: &PublicInputs): &vector<u8> {
        &inputs.seed_commitment
    }

    /// Borrow previous state root from public inputs
    public fun prev_state_root(inputs: &PublicInputs): &vector<u8> {
        &inputs.prev_state_root
    }

    /// Borrow previous actions root from public inputs
    public fun prev_actions_root(inputs: &PublicInputs): &vector<u8> {
        &inputs.prev_actions_root
    }

    /// Get previous nonce from public inputs
    public fun prev_nonce(inputs: &PublicInputs): u64 {
        inputs.prev_nonce
    }

    /// Borrow new state root from public inputs
    public fun new_state_root(inputs: &PublicInputs): &vector<u8> {
        &inputs.new_state_root
    }

    /// Borrow new actions root from public inputs
    public fun new_actions_root(inputs: &PublicInputs): &vector<u8> {
        &inputs.new_actions_root
    }

    /// Get new nonce from public inputs
    public fun new_nonce(inputs: &PublicInputs): u64 {
        inputs.new_nonce
    }
}
