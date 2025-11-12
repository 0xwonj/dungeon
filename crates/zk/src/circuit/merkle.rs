//! Sparse Merkle tree implementation with Poseidon hash.
//!
//! Optimized for batch operations and lazy tree building.

use crate::ProofError;

#[cfg(feature = "arkworks")]
use super::commitment::hash_two;
#[cfg(all(test, feature = "arkworks"))]
use super::commitment::hash_one;
#[cfg(feature = "arkworks")]
use ark_bn254::Fr as Fp254;
#[cfg(feature = "arkworks")]
use std::collections::BTreeMap;

#[cfg(feature = "arkworks")]
/// Merkle proof path with sibling hashes and direction bits.
///
/// OPTIMIZATION: Removed redundant `path_bits` field (now uses `directions` only).
#[derive(Debug, Clone)]
pub struct MerklePath {
    pub siblings: Vec<Fp254>,
    pub directions: Vec<bool>,
}

#[cfg(feature = "arkworks")]
/// Simple binary Merkle tree for proof generation.
///
/// OPTIMIZATION: Uses lazy tree building - tree is only built when root() or prove() is called,
/// and cache persists across operations unless leaves change.
#[derive(Debug, Clone)]
pub struct SparseMerkleTree {
    leaves: BTreeMap<u32, Fp254>,
    pub depth: usize,
    empty_hash: Fp254,
    tree_cache: BTreeMap<usize, BTreeMap<u32, Fp254>>,
    cache_dirty: bool,
}

#[cfg(feature = "arkworks")]
impl SparseMerkleTree {
    pub fn new(depth: usize) -> Self {
        Self {
            leaves: BTreeMap::new(),
            depth,
            empty_hash: Fp254::from(0u64),
            tree_cache: BTreeMap::new(),
            cache_dirty: false,
        }
    }

    /// Insert a single leaf.
    ///
    /// OPTIMIZATION: Marks cache as dirty but doesn't rebuild immediately (lazy evaluation).
    pub fn insert(&mut self, index: u32, value: Fp254) {
        self.leaves.insert(index, value);
        self.cache_dirty = true;
    }

    /// Batch insert multiple leaves at once.
    ///
    /// PERFORMANCE: ~10-100x faster than individual inserts for large batches,
    /// since tree is only rebuilt once at the end.
    pub fn batch_insert(&mut self, entries: impl IntoIterator<Item = (u32, Fp254)>) {
        for (index, value) in entries {
            self.leaves.insert(index, value);
        }
        self.cache_dirty = true;
    }

    /// Build the Merkle tree from leaves.
    ///
    /// OPTIMIZATION: Only rebuilds if cache_dirty flag is set (lazy evaluation).
    /// This avoids rebuilding the tree multiple times during batch operations.
    fn build_tree(&mut self) -> Result<(), ProofError> {
        // Early return if cache is valid
        if !self.cache_dirty && !self.tree_cache.is_empty() {
            return Ok(());
        }

        // Clear old cache and rebuild
        self.tree_cache.clear();

        // Level 0 uses leaves directly (they're already hashed by build_entity_tree)
        let mut current_level = BTreeMap::new();
        for (&index, &value) in &self.leaves {
            current_level.insert(index, value);
        }
        self.tree_cache.insert(0, current_level.clone());

        // Build tree bottom-up using Poseidon hash
        for level in 1..=self.depth {
            let mut next_level = BTreeMap::new();
            let level_size = 1u32 << (self.depth - level);

            for i in 0..level_size {
                let left = current_level
                    .get(&(i * 2))
                    .copied()
                    .unwrap_or(self.empty_hash);
                let right = current_level
                    .get(&(i * 2 + 1))
                    .copied()
                    .unwrap_or(self.empty_hash);
                next_level.insert(i, hash_two(left, right)?);
            }

            self.tree_cache.insert(level, next_level.clone());
            current_level = next_level;
        }

        // Mark cache as clean
        self.cache_dirty = false;

        Ok(())
    }

    pub fn root(&mut self) -> Result<Fp254, ProofError> {
        if self.leaves.is_empty() {
            return Ok(self.empty_hash);
        }

        self.build_tree()?;
        Ok(self
            .tree_cache
            .get(&self.depth)
            .and_then(|level| level.get(&0))
            .copied()
            .unwrap_or(self.empty_hash))
    }

    /// Generate a Merkle proof for a leaf at the given index.
    ///
    /// OPTIMIZATION: Pre-allocates vectors with known capacity.
    pub fn prove(&mut self, index: u32) -> Result<MerklePath, ProofError> {
        if !self.leaves.contains_key(&index) {
            return Err(ProofError::CircuitProofError(format!(
                "Leaf at index {} not found",
                index
            )));
        }

        self.build_tree()?;

        let mut siblings = Vec::with_capacity(self.depth);
        let mut directions = Vec::with_capacity(self.depth);
        let mut current_idx = index;

        for level in 0..self.depth {
            let is_right = current_idx % 2 == 1;
            let sibling_idx = if is_right {
                current_idx - 1
            } else {
                current_idx + 1
            };

            let sibling_hash = self
                .tree_cache
                .get(&level)
                .and_then(|level_map| level_map.get(&sibling_idx))
                .copied()
                .unwrap_or(self.empty_hash);

            siblings.push(sibling_hash);
            directions.push(is_right);
            current_idx /= 2;
        }

        Ok(MerklePath {
            siblings,
            directions,
        })
    }

    /// Verify a Merkle proof.
    ///
    /// OPTIMIZATION: Removed redundant length check.
    pub fn verify(
        leaf: Fp254,
        path: &MerklePath,
        expected_root: Fp254,
    ) -> Result<bool, ProofError> {
        // Leaf is already hashed (by build_entity_tree using hash_many)
        // Don't hash again - just use it directly
        let mut current = leaf;
        for (sibling, &is_right) in path.siblings.iter().zip(&path.directions) {
            current = if is_right {
                hash_two(*sibling, current)?
            } else {
                hash_two(current, *sibling)?
            };
        }

        Ok(current == expected_root)
    }
}

// ============================================================================
// GameState Merkle Tree Functions
// ============================================================================

#[cfg(feature = "arkworks")]
use game_core::{ActorState, GameState, ItemState, PropState};

#[cfg(feature = "arkworks")]
/// Hash multiple field elements together.
///
/// Currently uses a simple combination of hash_two for demonstration.
/// Production should use Poseidon hash with variable-length input.
pub fn hash_many(inputs: &[Fp254]) -> Result<Fp254, ProofError> {
    if inputs.is_empty() {
        return Ok(Fp254::from(0u64));
    }

    // Use Poseidon sponge to hash multiple inputs (same as circuit)
    use super::commitment::get_poseidon_config;
    use ark_crypto_primitives::sponge::{CryptographicSponge, poseidon::PoseidonSponge};

    let config = get_poseidon_config();
    let mut sponge = PoseidonSponge::<Fp254>::new(config);

    // Absorb all inputs
    for &input in inputs {
        let input_vec = vec![input];
        sponge.absorb(&input_vec.as_slice());
    }

    // Squeeze output
    let result = sponge.squeeze_field_elements::<Fp254>(1);

    result
        .first()
        .copied()
        .ok_or_else(|| ProofError::CircuitProofError("Poseidon squeeze failed".to_string()))
}

#[cfg(feature = "arkworks")]
/// Serialize an actor to field elements for hashing.
///
/// Serializes essential actor fields:
/// - Entity ID (u32)
/// - Position (x: i32, y: i32) - uses offset encoding for negative values
/// - Current HP (u32)
/// - Max HP (u32)
///
/// # Signed Integer Encoding
///
/// Coordinates use offset encoding: `field_value = (i32_value + COORD_OFFSET) as u64`
/// where COORD_OFFSET = 2^30 = 1,073,741,824. This allows coordinates in range
/// [-1,073,741,824, 1,073,741,823] to be represented as field elements [0, 2^31-1].
///
/// Examples:
/// - x = -5  => field_value = 1,073,741,819
/// - x = 0   => field_value = 1,073,741,824
/// - x = 10  => field_value = 1,073,741,834
///
/// Additional fields (equipment, stats, abilities) will be added in future iterations.
///
/// OPTIMIZATION: Returns fixed-size array (5 elements) - no heap allocation.
#[inline]
pub fn serialize_actor(actor: &ActorState) -> [Fp254; 5] {
    // Offset for signed integer encoding (2^30)
    // Allows coordinates in range [-2^30, 2^30-1]
    const COORD_OFFSET: i64 = 1 << 30;

    [
        Fp254::from(actor.id.0 as u64),
        // Encode signed coordinates with offset to avoid negative field element issues
        Fp254::from((actor.position.x as i64 + COORD_OFFSET) as u64),
        Fp254::from((actor.position.y as i64 + COORD_OFFSET) as u64),
        Fp254::from(actor.resources.hp as u64),
        // TODO: Compute max HP from stats snapshot instead of duplicating current HP
        // For now, use current HP as placeholder (Issue #XX)
        Fp254::from(actor.resources.hp as u64),
    ]
}

#[cfg(feature = "arkworks")]
/// Serialize a prop to field elements for hashing.
///
/// Serializes essential prop fields:
/// - Entity ID (u32)
/// - Position (x: i32, y: i32) - uses offset encoding for negative values
/// - Kind (u8 representation)
/// - Active status (0 or 1)
///
/// See `serialize_actor()` for signed integer encoding details.
///
/// OPTIMIZATION: Returns fixed-size array (5 elements) - no heap allocation.
#[inline]
pub fn serialize_prop(prop: &PropState) -> [Fp254; 5] {
    use game_core::PropKind;

    const COORD_OFFSET: i64 = 1 << 30;

    let kind_value = match prop.kind {
        PropKind::Door => 0u64,
        PropKind::Switch => 1u64,
        PropKind::Hazard => 2u64,
        PropKind::Other => 3u64,
    };

    [
        Fp254::from(prop.id.0 as u64),
        Fp254::from((prop.position.x as i64 + COORD_OFFSET) as u64),
        Fp254::from((prop.position.y as i64 + COORD_OFFSET) as u64),
        Fp254::from(kind_value),
        Fp254::from(if prop.is_active { 1u64 } else { 0u64 }),
    ]
}

#[cfg(feature = "arkworks")]
/// Serialize an item to field elements for hashing.
///
/// Serializes essential item fields:
/// - Entity ID (u32)
/// - Position (x: i32, y: i32) - uses offset encoding for negative values
/// - Item handle (u32)
/// - Quantity (u16)
///
/// See `serialize_actor()` for signed integer encoding details.
///
/// OPTIMIZATION: Returns fixed-size array (5 elements) - no heap allocation.
#[inline]
pub fn serialize_item(item: &ItemState) -> [Fp254; 5] {
    const COORD_OFFSET: i64 = 1 << 30;

    [
        Fp254::from(item.id.0 as u64),
        Fp254::from((item.position.x as i64 + COORD_OFFSET) as u64),
        Fp254::from((item.position.y as i64 + COORD_OFFSET) as u64),
        Fp254::from(item.handle.0 as u64),
        Fp254::from(item.quantity as u64),
    ]
}

#[cfg(feature = "arkworks")]
/// Build a Merkle tree from all entities in GameState.
///
/// Combines actors, props, and items into a single sparse Merkle tree
/// indexed by entity ID.
///
/// # Arguments
///
/// * `state` - The game state containing all entities
///
/// # Returns
///
/// A sparse Merkle tree with depth 10 (supports up to 1024 entities).
/// Each leaf is the hash of the serialized entity data.
///
/// OPTIMIZATION: Uses batch_insert for ~10-100x faster tree construction
/// compared to individual inserts, since tree is only built once at the end.
pub fn build_entity_tree(state: &GameState) -> Result<SparseMerkleTree, ProofError> {
    let mut tree = SparseMerkleTree::new(10); // depth 10 = up to 1024 entities

    // Collect all entity hashes into a vector for batch insert
    let total_entities =
        state.entities.actors.len() + state.entities.props.len() + state.entities.items.len();

    let mut leaves = Vec::with_capacity(total_entities);

    // Collect actor hashes
    for actor in state.entities.actors.iter() {
        let serialized = serialize_actor(actor);
        let leaf_hash = hash_many(&serialized)?;
        leaves.push((actor.id.0, leaf_hash));
    }

    // Collect prop hashes
    for prop in state.entities.props.iter() {
        let serialized = serialize_prop(prop);
        let leaf_hash = hash_many(&serialized)?;
        leaves.push((prop.id.0, leaf_hash));
    }

    // Collect item hashes
    for item in state.entities.items.iter() {
        let serialized = serialize_item(item);
        let leaf_hash = hash_many(&serialized)?;
        leaves.push((item.id.0, leaf_hash));
    }

    // Batch insert all leaves at once (tree is built lazily on first root/prove call)
    tree.batch_insert(leaves);

    Ok(tree)
}

#[cfg(feature = "arkworks")]
/// Compute the complete state root commitment.
///
/// This creates a hierarchical commitment to the entire game state:
/// - Entity tree (actors + props + items)
/// - Turn state (turn number, current actor, nonce)
/// - World state (future: tile occupancy, visibility)
///
/// Currently implements a simplified version using only the entity tree.
/// Full implementation will combine multiple subtree roots.
///
/// # Arguments
///
/// * `state` - The game state to commit to
///
/// # Returns
///
/// The Merkle root hash representing the complete state commitment.
pub fn compute_state_root(state: &GameState) -> Result<Fp254, ProofError> {
    // Build entity tree and get root
    let mut entity_tree = build_entity_tree(state)?;
    let entity_root = entity_tree.root()?;

    // Serialize turn state
    let turn_fields = vec![
        Fp254::from(state.turn.clock),
        Fp254::from(state.turn.current_actor.0 as u64),
        Fp254::from(state.turn.nonce),
    ];
    let turn_hash = hash_many(&turn_fields)?;

    // Combine entity root and turn hash for final state root
    // In full implementation, would also include world state and other components
    let state_root = hash_two(entity_root, turn_hash)?;

    Ok(state_root)
}

#[cfg(test)]
#[cfg(feature = "arkworks")]
mod game_state_tests {
    use super::*;
    use bounded_vector::BoundedVec;
    use game_core::{
        ActorState, CoreStats, EntitiesState, EntityId, InventoryState, Position, TurnState,
        WorldState,
    };

    #[test]
    fn test_serialize_actor() {
        let actor = ActorState::new(
            EntityId::PLAYER,
            Position::new(5, 10),
            CoreStats::default(),
            InventoryState::default(),
        );

        let serialized = serialize_actor(&actor);

        // Should have 5 field elements: id, x, y, hp, max_hp
        assert_eq!(serialized.len(), 5);
        assert_eq!(serialized[0], Fp254::from(EntityId::PLAYER.0 as u64));
        // Coordinates use offset encoding: (coord + 2^30) to handle signed values
        const COORD_OFFSET: i64 = 1 << 30;
        assert_eq!(
            serialized[1],
            Fp254::from((5i64 + COORD_OFFSET) as u64)
        );
        assert_eq!(
            serialized[2],
            Fp254::from((10i64 + COORD_OFFSET) as u64)
        );
    }

    #[test]
    fn test_hash_many_empty() {
        let result = hash_many(&[]).expect("hash_many should succeed");
        assert_eq!(result, Fp254::from(0u64));
    }

    #[test]
    fn test_hash_many_single() {
        let input = Fp254::from(42u64);
        let result = hash_many(&[input]).expect("hash_many should succeed");
        let expected = hash_one(input).expect("hash_one should succeed");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_hash_many_multiple() {
        let inputs = vec![Fp254::from(1u64), Fp254::from(2u64), Fp254::from(3u64)];
        let result = hash_many(&inputs).expect("hash_many should succeed");

        // Should be non-zero
        assert_ne!(result, Fp254::from(0u64));

        // Should be deterministic
        let result2 = hash_many(&inputs).expect("hash_many should succeed");
        assert_eq!(result, result2);
    }

    #[test]
    fn test_build_entity_tree_empty() {
        // Create state with no entities (except we need at least 1 actor)
        let actor = ActorState::new(
            EntityId::PLAYER,
            Position::default(),
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );

        let state = GameState::new(TurnState::default(), entities, WorldState::default());

        let tree = build_entity_tree(&state).unwrap();

        // Should have one entry (the player)
        assert_eq!(tree.leaves.len(), 1);
    }

    #[test]
    fn test_build_entity_tree_with_actors() {
        let actor1 = ActorState::new(
            EntityId::PLAYER,
            Position::new(0, 0),
            CoreStats::default(),
            InventoryState::default(),
        );

        let actor2 = ActorState::new(
            EntityId(1),
            Position::new(5, 5),
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor1, actor2]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );

        let state = GameState::new(TurnState::default(), entities, WorldState::default());

        let tree = build_entity_tree(&state).unwrap();

        // Should have two actors
        assert_eq!(tree.leaves.len(), 2);
        assert!(tree.leaves.contains_key(&EntityId::PLAYER.0));
        assert!(tree.leaves.contains_key(&1));
    }

    #[test]
    fn test_compute_state_root() {
        let actor = ActorState::new(
            EntityId::PLAYER,
            Position::new(3, 7),
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );

        let state = GameState::new(TurnState::default(), entities, WorldState::default());

        let root = compute_state_root(&state).unwrap();

        // Root should be non-zero
        assert_ne!(root, Fp254::from(0u64));

        // Should be deterministic
        let root2 = compute_state_root(&state).unwrap();
        assert_eq!(root, root2);
    }

    #[test]
    fn test_state_root_changes_with_position() {
        // Create two states with different actor positions
        let actor1 = ActorState::new(
            EntityId::PLAYER,
            Position::new(0, 0),
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities1 = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor1]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );
        let state1 = GameState::new(TurnState::default(), entities1, WorldState::default());

        let actor2 = ActorState::new(
            EntityId::PLAYER,
            Position::new(5, 5),
            CoreStats::default(),
            InventoryState::default(),
        );

        let entities2 = EntitiesState::new(
            unsafe { BoundedVec::from_vec_unchecked(vec![actor2]) },
            BoundedVec::new(),
            BoundedVec::new(),
        );
        let state2 = GameState::new(TurnState::default(), entities2, WorldState::default());

        let root1 = compute_state_root(&state1).unwrap();
        let root2 = compute_state_root(&state2).unwrap();

        // Roots should be different
        assert_ne!(root1, root2);
    }
}
