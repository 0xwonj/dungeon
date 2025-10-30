# Client Architecture

## Overview

The client architecture implements an **event-driven reactive UI pattern** that enables efficient, incremental updates to the presentation layer. The design prioritizes code reusability across multiple frontends (TUI, GUI, Web) while maintaining clean separation of concerns between game logic, state management, and rendering.

## Core Principles

### 1. Event-Driven Updates
Runtime events carry state deltas that enable incremental ViewModel updates instead of full state regeneration on every change.

### 2. Stateful ViewModel
The client maintains a local ViewModel that serves as a presentation-optimized cache, updated incrementally as events arrive.

### 3. Zero Duplication with game-core
UI types directly reuse `game-core` structures (e.g., `StatsSnapshot`, `PropKind`) rather than creating duplicate presentation types. This ensures the ZK proof system and UI consume identical data representations.

### 4. Framework Independence
A `PresentationMapper` abstraction allows the same ViewModel to drive different UI frameworks (Ratatui, egui, web renderers) by translating domain types to framework-specific styles.

---

## Architecture Layers

```
┌─────────────────────────────────────────┐
│         Runtime (Event Publisher)       │
│  Emits: Event { delta, after_state }   │
└──────────────┬──────────────────────────┘
               │
               ↓
┌─────────────────────────────────────────┐
│      Service Layer (Event Interpreter)  │
│  - Interprets StateDelta                │
│  - Updates ViewModel incrementally      │
│  - Returns UpdateScope (what changed)   │
└──────────────┬──────────────────────────┘
               │
               ↓
┌─────────────────────────────────────────┐
│        ViewModel (Presentation State)   │
│  - Owned by EventLoop                   │
│  - Incrementally updated                │
│  - Framework-agnostic types             │
└──────────────┬──────────────────────────┘
               │
               ↓
┌─────────────────────────────────────────┐
│       Renderer (Framework-Specific)     │
│  - Pure functions                       │
│  - Reads ViewModel (immutable)          │
│  - Uses PresentationMapper for styling  │
└─────────────────────────────────────────┘
```

---

## Layer Responsibilities

### Runtime Layer
**Location**: `crates/runtime`
**Responsibility**: Orchestrates game simulation and publishes events

**Key Outputs**:
- `Event::GameState(GameStateEvent::ActionExecuted)` containing:
  - `delta: StateDelta` - What changed
  - `after_state: GameState` - Complete state after change
  - `clock`, `action`, metadata

**Why it matters**: Events include both deltas (for incremental updates) and complete state (for fallback/initialization), enabling flexible update strategies.

---

### Service Layer
**Location**: `crates/client/core/src/services/`
**Responsibility**: Translates runtime events into ViewModel updates

**Key Components**:
- **ViewModelUpdater**: Interprets `StateDelta` and applies incremental changes to ViewModel
- **targeting**: Auto-target selection and tactical query helpers
- **UpdateScope**: Bitflags tracking which ViewModel fields changed

**UpdateScope Flags**:
```rust
bitflags! {
    pub struct UpdateScope: u32 {
        const TURN        = 0b00000001;  // Turn metadata changed
        const MAP         = 0b00000010;  // Map terrain changed
        const WORLD       = 0b00000100;  // World statistics changed
        const ACTORS      = 0b00001000;  // Actor entities changed
        const PROPS       = 0b00010000;  // Prop entities changed
        const ITEMS       = 0b00100000;  // Item entities changed
        const PLAYER_ONLY = 0b01000000;  // Only player stats changed
        const OCCUPANCY   = 0b10000000;  // Map occupancy changed
    }
}
```

**Example Flow**:
```
GameStateEvent::ActionExecuted { delta, after_state }
    ↓
ViewModelUpdater::update()
    ↓ [reads delta.entities.actors]
    ↓ [updates only changed ActorView entries]
    ↓
Returns UpdateScope::ACTORS
    ↓
EventLoop checks scope.is_empty() → render only if changed
```

**Design Goal**: Keep update logic separate from rendering concerns, making it reusable across all frontend implementations.

---

### ViewModel Layer
**Location**: `crates/client/core/src/view_model/`
**Responsibility**: Maintains presentation-optimized game state

**Structure**:
```rust
pub struct ViewModel {
    pub turn: TurnView,              // Turn metadata
    pub map: MapView,                 // 2D grid for rendering
    pub player: ActorView,            // Player cached for O(1) access
    pub actors: Vec<ActorView>,       // ALL actors (includes player at [0])
    pub props: Vec<PropView>,         // Props (for examination)
    pub items: Vec<ItemView>,         // Items (for examination)
    pub world: WorldSummary,          // Aggregate statistics
    pub last_sync_nonce: u64,        // Sync verification with GameState
}
```

**Design Notes**:
- `player`: Cached reference for O(1) access (UI frequently needs player data without searching)
- `actors`: ALL actors including player (invariant: `actors[0]` is always player)
- This allows both fast player access AND convenient iteration over all actors
- Use `view_model.npcs()` method to iterate NPCs only (excludes player via `skip(1)`)
- `last_sync_nonce`: Enables detection of stale data if events are lost

**Key Types**:
- `ActorView`: Contains `game_core::StatsSnapshot` directly (no duplication)
- `PropView`: Uses `game_core::PropKind` directly
- `ItemView`: Uses `game_core::ItemHandle` directly

**Lifecycle**:
1. **Initialization**: Created once from `GameState` at startup
2. **Incremental Updates**: Modified in-place by `ViewModelUpdater` as events arrive
3. **Read-Only Access**: Renderer reads but never mutates

**Why Stateful**: Maintaining state between frames enables:
- Faster updates (modify existing allocations vs recreate)
- Efficient delta application (O(changed) vs O(total))
- Natural reactive patterns (state → render)

---

### Presentation Layer
**Location**: `crates/client/core/src/view_model/presentation.rs`
**Responsibility**: Define framework-agnostic presentation interface

**Core Abstraction**:
```rust
pub trait PresentationMapper {
    type Style;

    fn render_actor(&self, stats: &StatsSnapshot, ...) -> (String, Style);
    fn render_prop(&self, kind: &PropKind, ...) -> (String, Style);
    fn render_terrain(&self, terrain: TerrainKind, ...) -> (String, Style);
    fn style_health(&self, current: u32, max: u32) -> Style;
    // ... more rendering methods
}
```

**Purpose**: Enable the same ViewModel to drive different UI frameworks by abstracting "how to render" from "what to render."

**Implementations**:
- `RatatuiTheme`: Maps to `ratatui::style::Style`
- Future: `EguiTheme`, `WebTheme`, etc.

---

### Renderer Layer
**Location**: `crates/client/{frontend}/src/presentation/widgets/`
**Responsibility**: Convert ViewModel to framework-specific UI primitives

**Pattern**: Pure Functions
```rust
pub fn render_map(
    frame: &mut Frame,
    area: Rect,
    view_model: &ViewModel,
    theme: &impl PresentationMapper,
) {
    // Read ViewModel (immutable)
    // Translate to Ratatui widgets using theme
    // No side effects, no state mutation
}
```

**Widget Organization**:
- `header.rs`: Turn info, current actor, game mode
- `map.rs`: 2D grid with entities
- `player_stats.rs`: HP, MP, speed, inventory
- `messages.rs`: Message log
- `examine.rs`: Detailed entity inspection
- `footer.rs`: Key bindings help

**Why Pure Functions**: Simplifies testing, enables hot-reload, prevents accidental state mutations.

---

## Data Flow Examples

### Example 1: Player Movement

```
1. User presses 'h' key
   ↓
2. InputHandler converts to Action::Move(West)
   ↓
3. Action sent to Runtime via mpsc channel
   ↓
4. Runtime executes action, generates Event:
   Event::GameState(ActionExecuted {
       delta: StateDelta {
           entities: Some(EntitiesChanges {
               actors: [(PLAYER, ActorFields {
                   position: Some(Position { x: 4, y: 5 })
               })]
           })
       },
       after_state: GameState { ... }
   })
   ↓
5. EventLoop receives event
   ↓
6. ViewModelUpdater::update()
   - Reads delta.entities.actors
   - Rebuilds view_model.actors with updated player position
   - Updates view_model.player cache (actors[0])
   - Returns UpdateScope::ACTORS
   ↓
7. EventLoop checks scope.is_empty() → false, render needed
   ↓
8. Widgets read view_model (immutable)
   - render_map() sees updated player position
   - render_player_stats() uses same ActorView
   ↓
9. Ratatui draws to terminal
```

**Key Insight**: Only the player's position field was updated. No full state regeneration, no redundant allocations.

---

### Example 2: NPC Takes Damage

```
1. Runtime processes NPC action (auto-generated)
   ↓
2. Combat system applies damage
   ↓
3. Event emitted:
   Event::GameState(ActionExecuted {
       delta: StateDelta {
           entities: Some(EntitiesChanges {
               actors: [(npc_id, ActorFields {
                   resources: Some(ResourceCurrent { hp: 45, ... })
               })]
           })
       },
       after_state: GameState { ... }
   })
   ↓
4. ViewModelUpdater detects resource change
   - Finds ActorView with id=npc_id
   - Updates actor_view.stats = after_state.entities.actor(npc_id).snapshot()
   - (StatsSnapshot recalculated because resources changed)
   ↓
5. Render uses updated StatsSnapshot
   - Health bar shows 45/100
   - Color changes based on percentage (red if < 30%)
```

**Key Insight**: `game_core::StatsSnapshot` is used directly by both the game engine and UI renderer. No translation layer needed.

---

## Performance Characteristics

### Memory
- **ViewModel Size**: ~500KB for 100 entities (typical)
- **Per-Frame Allocation**: Near-zero (ViewModel reused)
- **Delta Processing**: O(changed entities) not O(all entities)

### Update Cost
| Operation | Without Deltas | With Incremental Updates |
|-----------|----------------|--------------------------|
| Single move | O(n) entities | O(1) |
| HP change | O(n) entities | O(1) |
| Turn advance | O(n) entities | O(k) active actors |

### Rendering Cost
Rendering complexity remains O(visible tiles), but ViewModel preparation is now O(changed entities) rather than O(all entities).

---

## Extension Points

### Adding a New Frontend
1. Implement `PresentationMapper` for your framework
2. Create widget functions that read `ViewModel`
3. Wire up `EventLoop` with your event handling
4. Reuse `ViewModelUpdater` and all Service Layer code

**No changes needed** to `client-core` or `game-core`.

### Adding New UI Features
1. Extend `ViewModel` with new fields (e.g., `minimap: MinimapView`)
2. Add update logic to `ViewModelUpdater`
3. Create widget functions to render new data
4. Extend `PresentationMapper` if new visual mappings needed

### Optimizing Specific Updates
If profiling shows a bottleneck:
1. Add more granular `UpdateScope` flags
2. Implement specialized delta handlers in `ViewModelUpdater`
3. Widget functions can check `UpdateScope` to skip unchanged areas

---

## Design Trade-offs

### Chosen: Stateful ViewModel
**Pros**:
- O(changed) updates instead of O(total)
- Natural reactive patterns
- Memory efficiency (reuse allocations)

**Cons**:
- Must maintain synchronization with GameState
- More complex than stateless rebuild
- Potential for stale data if events lost

**Mitigation**: The `last_sync_nonce` field enables detection of stale data

---

### Chosen: Direct game-core Type Reuse
**Pros**:
- Zero duplication between ZK and UI
- Single source of truth
- Automatic consistency

**Cons**:
- UI tightly coupled to game-core types
- Cannot customize without affecting ZK

**Mitigation**: `PresentationMapper` provides rendering flexibility without changing types

---

### Chosen: Delta-Based Updates
**Pros**:
- Optimal performance for incremental changes
- Scales to large entity counts

**Cons**:
- More complex update logic
- Requires careful delta interpretation

**Mitigation**: Service Layer isolates complexity, can fall back to full rebuild if needed

---

## Future Considerations

### Change Detection Optimization
If the ViewModel grows large, consider:
- Fine-grained reactivity (per-field subscriptions)
- Dirty flags at the component level
- Virtual scrolling for large lists

### Multi-Frontend State Management
When supporting GUI + TUI simultaneously:
- Shared ViewModel in a separate process
- Each frontend subscribes to relevant slices
- Service Layer becomes a centralized state manager

### Persistence & Replay
ViewModel can be serialized for:
- Instant save/load (no GameState query needed)
- Replay UI state for debugging
- Network synchronization (multiplayer)

---

## Comparison to Industry Patterns

### Similar to Redux (React Ecosystem)
- Action → Reducer → Store → View
- Event → Updater → ViewModel → Renderer
- Immutable updates, selective re-rendering

### Similar to ECS Change Detection (Bevy/Unity DOTS)
- `Query<Changed<Health>>` → only process changed components
- `StateDelta` → only update changed entities

### Similar to MVVM (WPF/.NET)
- Model (GameState) → ViewModel (presentation state) → View (widgets)
- Two-way binding replaced with one-way event flow (simpler)

---

## Current Implementation

The architecture is fully implemented with all components operational.

### Core Components

**State Management:**
- `ViewModel` maintains presentation-optimized game state with incremental updates
- `ViewModelUpdater` interprets `StateDelta` events and returns `UpdateScope` to track changes
- `UpdateScope` bitflags enable granular change tracking across different state aspects
- `last_sync_nonce` provides synchronization verification to prevent stale data issues

**Presentation Layer:**
- `PresentationMapper` trait provides framework-agnostic styling interface
- `RatatuiTheme` implements terminal UI styling using the Ratatui framework
- All widgets implemented as pure functions with immutable ViewModel access

**Widget System:**

All widgets consume the ViewModel directly through pure functions:
- **header**: Turn clock, current actor, active count, mode indicator
- **footer**: Context-sensitive key bindings
- **messages**: Message log with bottom-to-top rendering
- **player_stats**: Player statistics with themed styling
- **map**: Full map grid with all entity types
- **examine**: Detailed entity inspection (Manual/Auto modes)
- **game_area**: Orchestrator widget composing map, stats, and examine panels

### Potential Optimizations

Areas identified for future performance improvements:
- Selective rendering using UpdateScope flags (currently renders all widgets on any change)
- Fine-grained reactivity for scenarios with large entity counts (100+ active entities)
- Virtual scrolling for very large maps (beyond typical 50x50 size)

---

## Summary

This architecture achieves:
- **Performance**: Incremental updates scale to large entity counts
- **Reusability**: Service Layer and ViewModel shared across all frontends
- **Simplicity**: Pure widget functions, clear data flow
- **Consistency**: Direct reuse of game-core types ensures ZK/UI alignment
- **Extensibility**: New frontends require minimal code, no core changes

The design is inspired by proven patterns (Redux, ECS, MVVM) adapted for a turn-based game with ZK proof requirements.
