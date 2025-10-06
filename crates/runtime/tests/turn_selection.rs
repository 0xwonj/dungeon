use game_core::{EntityId, GameConfig, GameEngine, GameState, Position, Tick};

#[test]
fn select_next_turn_picks_earliest_ready_at() {
    let mut state = GameState::default();
    let config = GameConfig::default();

    // Setup entities with different ready_at values
    state.entities.player.id = EntityId::PLAYER;
    state.entities.player.position = Position::ORIGIN;
    state.entities.player.ready_at = Some(Tick(100));

    state
        .entities
        .npcs
        .push(game_core::ActorState::new(
            EntityId(1),
            Position::new(1, 1),
            game_core::ActorStats::default(),
            game_core::InventoryState::default(),
        ).with_ready_at(Tick(50)))
        .unwrap();

    state
        .entities
        .npcs
        .push(game_core::ActorState::new(
            EntityId(2),
            Position::new(2, 2),
            game_core::ActorStats::default(),
            game_core::InventoryState::default(),
        ).with_ready_at(Tick(75)))
        .unwrap();

    // Activate all entities
    let mut engine = GameEngine::new(&mut state, &config);
    engine.activate(EntityId::PLAYER, Position::ORIGIN, Tick(100));
    engine.activate(EntityId(1), Position::new(1, 1), Tick(50));
    engine.activate(EntityId(2), Position::new(2, 2), Tick(75));

    // Manually select next turn (mimicking SimWorker::select_next_turn)
    let scheduled = state
        .turn
        .active_actors
        .iter()
        .filter_map(|&id| {
            let actor = state.entities.actor(id)?;
            actor.ready_at.map(|tick| (tick, id))
        })
        .min_by_key(|(tick, _)| *tick)
        .map(|(ready_at, entity)| (entity, ready_at));

    assert!(scheduled.is_some());
    let (entity, ready_at) = scheduled.unwrap();
    assert_eq!(entity, EntityId(1));
    assert_eq!(ready_at, Tick(50));
}

#[test]
fn activation_region_filters_by_distance() {
    let player_pos = Position::new(5, 5);
    let radius = 2;

    // Within range
    assert!(is_within_activation_region(
        player_pos,
        Position::new(5, 5),
        radius
    ));
    assert!(is_within_activation_region(
        player_pos,
        Position::new(6, 6),
        radius
    ));
    assert!(is_within_activation_region(
        player_pos,
        Position::new(7, 7),
        radius
    ));

    // Out of range
    assert!(!is_within_activation_region(
        player_pos,
        Position::new(8, 8),
        radius
    ));
    assert!(!is_within_activation_region(
        player_pos,
        Position::new(10, 5),
        radius
    ));
}

fn is_within_activation_region(
    player_position: Position,
    entity_position: Position,
    activation_radius: u32,
) -> bool {
    let dx = (entity_position.x - player_position.x).abs() as u32;
    let dy = (entity_position.y - player_position.y).abs() as u32;
    dx <= activation_radius && dy <= activation_radius
}
