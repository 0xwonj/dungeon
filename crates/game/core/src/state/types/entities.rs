//! Entity collection and prop state types.
//!
//! This module contains:
//! - EntitiesState: Aggregate container for all entities
//! - PropState: Non-actor entities (doors, switches, hazards)

use bounded_vector::BoundedVec;

use super::actor::ActorState;
use super::item::ItemState;
use super::{EntityId, Position};
use crate::config::GameConfig;
use crate::provider::{InteractiveKind, ProviderKind};
use crate::traits::{Faction, Species, TraitProfile};

/// Aggregate state for every entity in the map.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntitiesState {
    /// All actors (including player). Player is typically at index 0 with EntityId::PLAYER.
    /// Minimum size is 1 to guarantee player exists.
    pub actors: BoundedVec<ActorState, 1, { GameConfig::MAX_ACTORS }>,
    pub props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
    pub items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
}

impl EntitiesState {
    pub fn new(
        actors: BoundedVec<ActorState, 1, { GameConfig::MAX_ACTORS }>,
        props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
        items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
    ) -> Self {
        Self {
            actors,
            props,
            items,
        }
    }

    /// Creates an empty EntitiesState with no actors (minimum constraint temporarily violated).
    ///
    /// # Safety
    ///
    /// This violates the MIN=1 constraint for actors. The caller MUST add at least one actor
    /// (typically the player) before using this state in gameplay logic.
    ///
    /// Use this only for scenario initialization where entities will be added immediately.
    pub fn empty() -> Self {
        Self {
            actors: unsafe { BoundedVec::from_vec_unchecked(vec![]) },
            props: BoundedVec::new(),
            items: BoundedVec::new(),
        }
    }

    /// Returns a reference to an actor by ID.
    pub fn actor(&self, id: EntityId) -> Option<&ActorState> {
        self.actors.iter().find(|a| a.id == id)
    }

    /// Returns a mutable reference to an actor by ID.
    pub fn actor_mut(&mut self, id: EntityId) -> Option<&mut ActorState> {
        self.actors.iter_mut().find(|a| a.id == id)
    }

    /// Returns a reference to the player actor.
    ///
    /// # Panics
    ///
    /// Panics if no actor with EntityId::PLAYER exists (should never happen if invariants are maintained).
    pub fn player(&self) -> &ActorState {
        self.actor(EntityId::PLAYER)
            .expect("Player must exist in EntitiesState")
    }

    /// Returns a mutable reference to the player actor.
    ///
    /// # Panics
    ///
    /// Panics if no actor with EntityId::PLAYER exists (should never happen if invariants are maintained).
    pub fn player_mut(&mut self) -> &mut ActorState {
        self.actor_mut(EntityId::PLAYER)
            .expect("Player must exist in EntitiesState")
    }

    /// Returns an iterator over all actors.
    pub fn all_actors(&self) -> impl Iterator<Item = &ActorState> {
        self.actors.iter()
    }

    /// Returns a mutable iterator over all actors.
    pub fn all_actors_mut(&mut self) -> impl Iterator<Item = &mut ActorState> {
        self.actors.iter_mut()
    }

    /// Returns the position of an actor by ID.
    ///
    /// Returns `None` if the actor is not found or has no position.
    pub fn position(&self, id: EntityId) -> Option<Position> {
        self.actor(id)?.position
    }

    /// Returns a reference to an item by ID.
    pub fn item(&self, id: EntityId) -> Option<&ItemState> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Returns a mutable reference to an item by ID.
    pub fn item_mut(&mut self, id: EntityId) -> Option<&mut ItemState> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Returns an iterator over all items.
    pub fn all_items(&self) -> impl Iterator<Item = &ItemState> {
        self.items.iter()
    }

    /// Returns a mutable iterator over all items.
    pub fn all_items_mut(&mut self) -> impl Iterator<Item = &mut ItemState> {
        self.items.iter_mut()
    }

    /// Returns a reference to a prop by ID.
    pub fn prop(&self, id: EntityId) -> Option<&PropState> {
        self.props.iter().find(|p| p.id == id)
    }

    /// Returns a mutable reference to a prop by ID.
    pub fn prop_mut(&mut self, id: EntityId) -> Option<&mut PropState> {
        self.props.iter_mut().find(|p| p.id == id)
    }

    /// Returns an iterator over all props.
    pub fn all_props(&self) -> impl Iterator<Item = &PropState> {
        self.props.iter()
    }

    /// Returns a mutable iterator over all props.
    pub fn all_props_mut(&mut self) -> impl Iterator<Item = &mut PropState> {
        self.props.iter_mut()
    }
}

impl EntitiesState {
    /// Create a new entities state with a default player actor.
    pub fn with_player() -> Self {
        use crate::env::ActorTemplate;

        // Create default player template
        let template = ActorTemplate::builder()
            .provider_kind(ProviderKind::Interactive(InteractiveKind::CliInput))
            .species(Species::Human)
            .faction(Faction::Player)
            .archetype("none")
            .temperament("neutral")
            .trait_profile(TraitProfile::default())
            .build();

        // Convert to ActorState with PLAYER id and default position
        let player = template.to_actor(EntityId::PLAYER, Position::default());

        // SAFETY: We're creating a Vec with exactly 1 element, which satisfies MIN=1 constraint
        let actors = unsafe { BoundedVec::from_vec_unchecked(vec![player]) };

        Self {
            actors,
            props: BoundedVec::new(),
            items: BoundedVec::new(),
        }
    }
}

/// Non-actor entities such as doors, switches, or hazards.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PropState {
    pub id: EntityId,
    pub position: Position,
    pub kind: PropKind,
    pub is_active: bool,
}

impl PropState {
    pub fn new(id: EntityId, position: Position, kind: PropKind, is_active: bool) -> Self {
        Self {
            id,
            position,
            kind,
            is_active,
        }
    }
}

/// Enumerates the basic prop categories. Extend as needed by gameplay.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PropKind {
    Door,
    Switch,
    Hazard,
    Other,
}
