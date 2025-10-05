# “Dungeon” — Game Systems Design (System-First, Minimal Start)

*A compact, deterministic, 2D, turn-based, tilemap RPG. This document defines the **game systems** themselves—not code architecture—so the team can build a coherent first playable and expand cleanly later. No numeric tuning is prescribed.*

---

## 1) Vision & Scope

* **Tactics over twitch:** Clear, legible rules with predictable outcomes.
* **Deterministic resolution:** Same inputs → same results, always.
* **Small surface, deep choices:** Few actions, compact spaces, meaningful trade-offs.
* **Composable systems:** Movement, terrain, statuses, skills, items and AI interact in explicit ways.

---

## 2) Temporal Model (Turns & Phases)

* **Discrete turns** shared by all entities.
* **Per-turn phases (fixed order):**

  1. **Player Action** (one action).
  2. **NPC Actions** (each NPC acts once, in a documented stable order).
  3. **End-of-Turn (EoT) Ticks** (status durations, hazard ticks, cleanup).
* **Atomic actions:** An action either fully applies or is rejected; no partial effects.
* **Tie-breaking:** When order matters within a phase, use a single, documented rule (e.g., lowest entity ID then reading order).

---

## 3) Spatial Model (Grid & Tiles)

* **Orthogonal grid** with integer coordinates.
* **Tile tags** (examples): floor, wall, door, exit, hazard, switch, treasure, spawn.
* **Base semantics:**

  * **Walkability:** Only walkable tiles can be entered.
  * **Occupancy:** Fixed number of entities per tile.
  * **Triggers:** Tiles may define hooks—**on-enter**, **on-stand**, **on EoT**—with deterministic effects.
  * **Topology changes:** Doors/switches can toggle walkability or hazards in a visible, immediate way.

---

## 4) Entities & State

* **Kinds:** Player, NPC (multiple archetypes), and interactive objects.
* **Per-entity state:** identity, position, resources, statuses, intent (optional, for telegraphs), cooldowns.
* **Lifecycle:** spawn → act per turn → die/despawn on conditions → cleanup effects.

---

## 5) Action System (Validate → Execute → Effects)

* **Action categories (MVP):**

  * **Move** (cardinal step).
  * **Attack** (adjacent melee).
  * **Use Item** (consume, apply effect).
  * **Interact** (adjacent object: door/switch/chest/exit).
  * **Wait** (advance time deliberately).
* **Pipeline per action:**

  1. **Validate** preconditions (legality, range, resources, cooldowns, occupancy).
  2. **Execute** state changes in a fixed internal order.
  3. **Apply effects** and register any triggers/marks for EoT.
  4. **Log** the outcome in a human-readable turn log.
* **Targeting rules:**

  * **Melee:** Manhattan adjacency.
  * **Ranged (post-MVP):** Define max distance and an LoS rule (grid ray or simplified “ignores walls” variant—choose one and document).

---

## 6) Movement System

* **Cardinal movement** only in MVP (no diagonals, no pushing/pulling by default).
* **Legality checks:** destination exists, is walkable, and is unoccupied.
* **Triggers:** resolve **on-enter** effects immediately after a successful move (e.g., hazards).
* **Blocked paths:** rejected with a clear, typed reason (e.g., wall, occupied, out of bounds).

---

## 7) Combat System

* **Deterministic resolution:** No randomness, no hidden modifiers.
* **Melee flow:** legality check → compute damage from explicit formula → apply mitigation → apply damage → check defeat → cleanup.
* **Damage sources:** attacks, hazards, statuses, items/skills.
* **Ordering:** Defense/mitigation first, then damage application, then death checks, then on-death triggers.

---

## 8) Status & Effect System

* **Statuses are discrete** (binary or small bounded stacks): examples—**Guard**, **Poison**, **Stun**, **Slow**, **Burn**.
* **Each status defines:**

  * **Apply conditions** (when it can be added).
  * **Timing** (immediate, on action, EoT tick).
  * **Duration model** (turn counts; decreases at EoT).
  * **Stacking policy** (no stack / capped / replace).
  * **Conflict resolution** (priority or exclusive tags).
* **Effect layering (fixed precedence):** Prevents → Replacements → Additive modifiers → Derived totals → Outcomes.
* **Cleanup:** Remove expired statuses during EoT; resolve death/despawn last.

---

## 9) Resource & Cooldown System

* **Resources:** At minimum **Health** and one **Ability resource** (e.g., energy/mana).
* **Costs:** Checked before execution; consumed once in a fixed order.
* **Bounds:** Clamp results to valid ranges; define explicit behavior at zero.
* **Cooldowns:** Integer turns; decrement at EoT; action illegal if cooldown > 0.

---

## 10) Items & Inventory

* **Inventory model:** Finite slots or keyed collection; ownership is part of entity state.
* **Item types:**

  * **Consumables:** one-shot effects (heal, cleanse, blink, bomb).
  * **Keys:** unlock doors/switches.
  * **Artifacts (post-MVP):** passives that alter rules in simple, documented ways.
* **Use Item flow:** legality → consume (if applicable) → apply effect → update inventory state → log change.
* **Determinism:** Item effects must be previewable before commit.

---

## 11) Interactables & Environment

* **Doors/Switches:** Toggle walkability or open paths; changes are immediate and visible.
* **Hazards:** Defined by timing hook (on-enter/on-stand/EoT), area (single tile or zone), and effect (deterministic).
* **Treasure/Chests:** Grant items or flags; one-time or gated by keys.
* **Exits:** Standing on exit at the appropriate phase ends the room with a success flag.

---

## 12) Visibility & Information (Optional for MVP)

* **Line of Sight:** A simple, consistent rule (grid ray or Manhattan gating).
* **Fog of War:** If used, reveal within a radius or LoS-based field; unrevealed tiles are hidden but consistent.
* **Telegraphs:** Enemies may expose intent for the next turn (arrows, highlights).

---

## 13) NPC Behavior (Deterministic Archetypes)

* **Chaser:** Move to reduce Manhattan distance; attack if adjacent.
* **Zoner:** Maintain preferred distance; use ranged attack if allowed.
* **Sentinel:** Stationary until triggered; then follows a fixed pattern (charge, line shot, cone, etc.).
* **Controller:** Create zones/hazards on a defined schedule.
* **Decision priority per archetype:** e.g., attack if legal → reposition → wait.
* **Action order:** NPCs act in a stable, documented sequence each turn.

---

## 14) Encounter & Objective System

* **Room as unit:** A compact map with a set of entities and interactables.
* **Objectives (choose per room):** reach exit, defeat enemies, activate switches, survive X turns.
* **Failure conditions:** health depleted, soft-fail objectives (e.g., escape blocked), or time pressure (if defined).
* **Progression between rooms (post-MVP):** carry items/status flags; present simple route choices.

---

## 15) Turn Log & Feedback

* **Turn log:** A human-readable sequence describing validations, actions, and outcomes per phase.
* **Previews:** Before committing an action, show expected results (new positions, affected tiles, predicted damage/status changes).
* **Readable icons/overlays:** Movement ranges, target highlights, hazard zones, status markers with tooltips.

---

## 16) Save/Load & Reproducibility

* **Snapshot model:** Room state (tiles, entity states, statuses, inventory, cooldowns, objectives), turn index, and logs.
* **Deterministic replay:** Given initial snapshot and the same action script, replay produces the same outcomes.
* **Versioning:** Include a ruleset/version tag in the save to prevent incompatible loads.

---

## 17) Content Authoring (Structure, not numbers)

* **Tilemap format:** Simple, versioned (CSV/TMX/JSON); tags map directly to system behaviors.
* **Entity prefabs:** Declared by attributes (archetype, stats, AI parameters, inventory).
* **Interactions:** Doors/switches/hazards defined declaratively with hooks and scopes (tiles/zones), not scripts in MVP.

---

## 18) Difficulty & Pacing Knobs (Non-numeric)

* **Composition:** Enemy archetype mix, interactable density, and hazard presence.
* **Layout:** Choke points vs. open fields; alternate safe and pressure spaces.
* **Objective pressure:** Optional soft timers (e.g., expanding hazards), multi-switch gates, or patrol routes.

---

## 19) MVP Feature Checklist (Build Order)

1. **Turn/Phase system** with stable ordering and tie-breaks.
2. **Tilemap** with walkability, occupancy, basic tags (floor, wall, exit, hazard).
3. **Movement** (cardinal) + **on-enter triggers**.
4. **Melee combat** (adjacent) with deterministic resolution.
5. **Statuses (minimal)** with duration and timing at EoT.
6. **Items (one consumable)** and **Interact** (door/switch/exit).
7. **NPC archetype (Chaser)** with deterministic priority.
8. **Turn log** and **action preview**.
9. **Save/Load snapshot** and deterministic replay.

---

## 20) Expansion Hooks (Post-MVP)

* **Ranged/area skills**, push/pull, multi-tile moves (dash).
* **Advanced hazards:** conveyors, collapsing floors, timed traps.
* **Additional AI archetypes** with telegraphed multi-turn patterns.
* **Fog of War & LoS** if the tactical clarity benefits.
* **Artifacts/passives** that alter simple rules (e.g., ignore first hazard per turn).

---

### Guiding Principles (Keep These True)

* **Legibility:** Players should infer legality and results from what they see.
* **Determinism:** No hidden randomness; fixed resolution orders.
* **Atomicity:** Validate before execute; all-or-nothing actions.
* **Composability:** Systems combine without surprises; precedence is documented.
* **Minimal first:** Start with the smallest coherent set; grow by adding new actions/statuses/tags rather than changing existing semantics.
