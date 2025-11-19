/// Proof Verification Module - RISC0 Groth16 Compatible
///
/// Implements two-stage verification for ZK proofs of game state transitions.
///
/// # RISC0 Groth16 Architecture
///
/// RISC0's Groth16 wrapper only exposes 3 public inputs:
/// 1. CONTROL_ROOT - RISC0 control root
/// 2. CLAIM_DIGEST - Hash of execution claim (contains IMAGE_ID and JOURNAL_DIGEST)
/// 3. CONTROL_ID - Control identifier
///
/// The journal_digest (SHA-256 of journal bytes) is embedded in CLAIM_DIGEST,
/// NOT directly accessible as a Groth16 public input.
///
/// # Two-Stage Verification
///
/// **Stage 1 - Groth16 Proof Verification:**
/// - Verify the cryptographic proof (seal) is valid
/// - Verify journal_digest matches the proof's committed value
/// - This proves: "Some valid execution produced this journal digest"
///
/// **Stage 2 - Journal Content Verification:**
/// - Receive 168-byte journal data from caller
/// - Verify: SHA-256(journal_data) == journal_digest
/// - Extract and validate 6 fields from journal
/// - This proves: "The journal contains these specific committed values"
///
/// # Journal Structure (168 bytes)
///
/// The guest program commits 6 fields to the journal in exact order:
/// ```
/// 1. oracle_root       (32 bytes, offset 0..32)   - Static game content commitment
/// 2. seed_commitment   (32 bytes, offset 32..64)  - RNG seed commitment
/// 3. prev_state_root   (32 bytes, offset 64..96)  - State before execution
/// 4. actions_root      (32 bytes, offset 96..128) - Action sequence (Walrus blob_id)
/// 5. new_state_root    (32 bytes, offset 128..160) - State after execution
/// 6. new_nonce         (8 bytes, offset 160..168) - Action counter (u64 little-endian)
/// ```
///
/// Total: 168 bytes (5 × 32 + 8)
///
/// # Design Rationale
///
/// This two-stage approach is required because RISC0 Groth16 wrapper does not
/// expose individual journal fields as public inputs. Instead, it only exposes
/// the journal digest. This design:
/// - ✅ Maintains full RISC0 compatibility
/// - ✅ Preserves all 6 fields as verifiable data
/// - ✅ Minimal gas overhead (one SHA-256 + comparison)
/// - ✅ Clean separation: cryptographic proof vs data validation
///
/// Tradeoff: Journal data (168 bytes) must be included in transaction calldata
module dungeon::proof_verifier {
    use sui::groth16;
    use std::hash;

    // ===== Error Codes =====

    /// Invalid proof - Groth16 verification failed
    const EInvalidProof: u64 = 1;
    /// Journal digest mismatch - provided journal doesn't hash to expected digest
    const EJournalMismatch: u64 = 2;
    /// Invalid journal format - wrong size or structure
    const EInvalidJournal: u64 = 3;

    // ===== Constants =====

    /// Expected journal size in bytes (5 × 32 + 8)
    const JOURNAL_SIZE: u64 = 168;

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

    /// Journal data structure - matches guest program output (168 bytes)
    ///
    /// This struct represents the parsed journal committed by the RISC0 guest program.
    /// All fields are cryptographically committed via the journal_digest.
    public struct JournalData has copy, drop, store {
        /// Oracle data commitment (32 bytes) - SHA-256 of OracleSnapshot
        oracle_root: vector<u8>,
        /// Seed commitment for RNG (32 bytes) - SHA-256 of game_seed
        seed_commitment: vector<u8>,
        /// Previous state root (32 bytes) - SHA-256 of GameState before execution
        prev_state_root: vector<u8>,
        /// Actions root (32 bytes) - Walrus blob_id or SHA-256 of action sequence
        /// For single action: all zeros (no batch)
        /// For batch: actual Walrus blob_id or hash commitment
        actions_root: vector<u8>,
        /// New state root (32 bytes) - SHA-256 of GameState after execution
        new_state_root: vector<u8>,
        /// New nonce (8 bytes, u64) - Action counter after execution
        new_nonce: u64,
    }

    // ===== Admin Functions =====

    /// Initialize a new verifying key as a shared object
    ///
    /// This should be called once to prepare the verifying key for the game circuit.
    /// The verifying key comes from the RISC0 Groth16 trusted setup.
    /// The created VerifyingKey is shared globally so all users can verify proofs.
    ///
    /// # Arguments
    /// * `vk_bytes` - Raw verifying key bytes from RISC0
    /// * `version` - Circuit version identifier
    /// * `ctx` - Transaction context
    entry fun create_verifying_key(
        vk_bytes: vector<u8>,
        version: u64,
        ctx: &mut TxContext,
    ) {
        let curve = groth16::bn254();
        let prepared_vk = groth16::prepare_verifying_key(&curve, &vk_bytes);

        let vk = VerifyingKey {
            id: object::new(ctx),
            prepared_vk,
            version,
        };

        // Share the VK so everyone can use it for proof verification
        transfer::share_object(vk);
    }

    // ===== Public Functions =====

    /// Verify a game state transition proof with two-stage verification
    ///
    /// **Stage 1:** Verify Groth16 proof with journal_digest as public input
    /// **Stage 2:** Verify journal_data hashes to journal_digest and extract fields
    ///
    /// This function combines both stages for convenience. For gas optimization,
    /// you can use `verify_groth16` and `verify_journal` separately.
    ///
    /// # Arguments
    /// * `vk` - Prepared verifying key
    /// * `journal_digest` - SHA-256 hash of journal (32 bytes) - the Groth16 public input
    /// * `journal_data` - Actual journal content (168 bytes)
    /// * `proof_bytes` - Groth16 proof bytes (seal)
    ///
    /// # Returns
    /// Parsed JournalData if verification succeeds
    ///
    /// # Aborts
    /// * `EInvalidProof` - If Groth16 proof verification fails
    /// * `EJournalMismatch` - If journal doesn't hash to expected digest
    /// * `EInvalidJournal` - If journal format is invalid
    public fun verify_game_proof(
        vk: &VerifyingKey,
        journal_digest: vector<u8>,
        journal_data: &JournalData,
        proof_bytes: vector<u8>,
    ): JournalData {
        // Stage 1: Verify Groth16 proof with journal_digest
        verify_groth16(vk, journal_digest, proof_bytes);

        // Stage 2: Verify journal content matches digest
        verify_journal(journal_digest, journal_data);

        // Return parsed journal data
        *journal_data
    }

    /// Stage 1: Verify Groth16 proof with journal_digest as public input
    ///
    /// Verifies that the cryptographic proof is valid for the given journal digest.
    ///
    /// # Arguments
    /// * `vk` - Prepared verifying key
    /// * `journal_digest` - SHA-256 hash of journal (32 bytes)
    /// * `proof_bytes` - Groth16 proof bytes (seal)
    ///
    /// # Aborts
    /// * `EInvalidProof` - If proof verification fails
    public fun verify_groth16(
        vk: &VerifyingKey,
        journal_digest: vector<u8>,
        proof_bytes: vector<u8>,
    ) {
        // Create proof points from bytes
        let curve = groth16::bn254();
        let proof_points = groth16::proof_points_from_bytes(proof_bytes);

        // Public input is just the journal digest (32 bytes)
        let public_inputs = groth16::public_proof_inputs_from_bytes(journal_digest);

        // Verify Groth16 proof
        let valid = groth16::verify_groth16_proof(
            &curve,
            &vk.prepared_vk,
            &public_inputs,
            &proof_points,
        );

        assert!(valid, EInvalidProof);
    }

    /// Stage 2: Verify journal content matches digest and validate structure
    ///
    /// Verifies that:
    /// 1. Provided journal data hashes to the expected journal_digest
    /// 2. Journal structure is valid (168 bytes with correct field layout)
    ///
    /// # Arguments
    /// * `expected_digest` - Expected SHA-256 hash of journal
    /// * `journal_data` - Journal data to verify
    ///
    /// # Aborts
    /// * `EJournalMismatch` - If computed digest doesn't match expected
    public fun verify_journal(
        expected_digest: vector<u8>,
        journal_data: &JournalData,
    ) {
        // Compute digest of provided journal data
        let computed_digest = compute_journal_digest(journal_data);

        // Verify digest matches
        assert!(computed_digest == expected_digest, EJournalMismatch);
    }

    // ===== Helper Functions =====

    /// Compute SHA-256 digest of journal data
    ///
    /// Serializes journal fields in exact order matching guest program:
    /// 1. oracle_root (32 bytes)
    /// 2. seed_commitment (32 bytes)
    /// 3. prev_state_root (32 bytes)
    /// 4. actions_root (32 bytes)
    /// 5. new_state_root (32 bytes)
    /// 6. new_nonce (8 bytes, u64 little-endian)
    ///
    /// Total: 168 bytes → SHA-256 → 32 bytes digest
    fun compute_journal_digest(journal: &JournalData): vector<u8> {
        let mut bytes = vector::empty<u8>();

        // 1. Oracle root (32 bytes)
        vector::append(&mut bytes, journal.oracle_root);

        // 2. Seed commitment (32 bytes)
        vector::append(&mut bytes, journal.seed_commitment);

        // 3. Previous state root (32 bytes)
        vector::append(&mut bytes, journal.prev_state_root);

        // 4. Actions root (32 bytes)
        vector::append(&mut bytes, journal.actions_root);

        // 5. New state root (32 bytes)
        vector::append(&mut bytes, journal.new_state_root);

        // 6. New nonce (8 bytes, u64 little-endian)
        vector::append(&mut bytes, u64_to_bytes_le(journal.new_nonce));

        // Verify total size is 168 bytes
        assert!(vector::length(&bytes) == JOURNAL_SIZE, EInvalidJournal);

        // Compute SHA-256 (sha2_256 from std::hash)
        hash::sha2_256(bytes)
    }

    /// Convert u64 to 8-byte array (little-endian)
    ///
    /// This matches the Rust `u64::to_le_bytes()` serialization used in
    /// the guest program and host prover.
    fun u64_to_bytes_le(value: u64): vector<u8> {
        let mut bytes = vector::empty<u8>();

        // Extract 8 bytes in little-endian order
        let mut v = value;
        let mut i = 0;
        while (i < 8) {
            vector::push_back(&mut bytes, ((v & 0xFF) as u8));
            v = v >> 8;
            i = i + 1;
        };

        bytes
    }

    // ===== Constructor Functions =====

    /// Create journal data from raw 168-byte vector
    ///
    /// Parses a raw journal byte vector into structured JournalData.
    /// Useful when receiving journal from off-chain or storage.
    ///
    /// # Arguments
    /// * `journal_bytes` - Raw journal bytes (must be exactly 168 bytes)
    ///
    /// # Returns
    /// Parsed JournalData
    ///
    /// # Aborts
    /// * `EInvalidJournal` - If size is not 168 bytes
    public fun parse_journal_bytes(journal_bytes: vector<u8>): JournalData {
        // Verify size
        assert!(vector::length(&journal_bytes) == JOURNAL_SIZE, EInvalidJournal);

        // Extract fields in order (matching offsets from documentation)
        let oracle_root = vector_slice(&journal_bytes, 0, 32);
        let seed_commitment = vector_slice(&journal_bytes, 32, 64);
        let prev_state_root = vector_slice(&journal_bytes, 64, 96);
        let actions_root = vector_slice(&journal_bytes, 96, 128);
        let new_state_root = vector_slice(&journal_bytes, 128, 160);
        let new_nonce = bytes_to_u64_le(&journal_bytes, 160);

        JournalData {
            oracle_root,
            seed_commitment,
            prev_state_root,
            actions_root,
            new_state_root,
            new_nonce,
        }
    }

    /// Create journal data from individual field components
    ///
    /// # Arguments
    /// * `oracle_root` - Oracle data commitment (32 bytes)
    /// * `seed_commitment` - RNG seed commitment (32 bytes)
    /// * `prev_state_root` - Previous state root (32 bytes)
    /// * `actions_root` - Actions commitment (32 bytes)
    /// * `new_state_root` - New state root (32 bytes)
    /// * `new_nonce` - New nonce (u64)
    ///
    /// # Returns
    /// JournalData struct
    public fun new_journal_data(
        oracle_root: vector<u8>,
        seed_commitment: vector<u8>,
        prev_state_root: vector<u8>,
        actions_root: vector<u8>,
        new_state_root: vector<u8>,
        new_nonce: u64,
    ): JournalData {
        JournalData {
            oracle_root,
            seed_commitment,
            prev_state_root,
            actions_root,
            new_state_root,
            new_nonce,
        }
    }

    // ===== View Functions =====

    /// Get the verifying key version
    public fun verifying_key_version(vk: &VerifyingKey): u64 {
        vk.version
    }

    /// Borrow oracle root from journal data
    public fun oracle_root(journal: &JournalData): &vector<u8> {
        &journal.oracle_root
    }

    /// Borrow seed commitment from journal data
    public fun seed_commitment(journal: &JournalData): &vector<u8> {
        &journal.seed_commitment
    }

    /// Borrow previous state root from journal data
    public fun prev_state_root(journal: &JournalData): &vector<u8> {
        &journal.prev_state_root
    }

    /// Borrow actions root from journal data (Walrus blob_id)
    public fun actions_root(journal: &JournalData): &vector<u8> {
        &journal.actions_root
    }

    /// Borrow new state root from journal data
    public fun new_state_root(journal: &JournalData): &vector<u8> {
        &journal.new_state_root
    }

    /// Get new nonce from journal data
    public fun new_nonce(journal: &JournalData): u64 {
        journal.new_nonce
    }

    // ===== Internal Helper Functions =====

    /// Extract a slice from a vector (inclusive start, exclusive end)
    fun vector_slice(vec: &vector<u8>, start: u64, end: u64): vector<u8> {
        let mut result = vector::empty<u8>();
        let mut i = start;
        while (i < end) {
            vector::push_back(&mut result, *vector::borrow(vec, i));
            i = i + 1;
        };
        result
    }

    /// Convert 8 bytes to u64 (little-endian) starting at offset
    fun bytes_to_u64_le(bytes: &vector<u8>, offset: u64): u64 {
        let mut result: u64 = 0;
        let mut i: u64 = 0;
        while (i < 8) {
            let byte = (*vector::borrow(bytes, offset + i) as u64);
            let shift = ((i * 8) as u8);
            result = result | (byte << shift);
            i = i + 1;
        };
        result
    }
}
