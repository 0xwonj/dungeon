# Turn System Design

## Goals
- Provide deterministic, zk-friendly sequencing of player and NPC actions.
- Prioritize responsiveness near the player while keeping distant entities cheap to simulate.
- Model speed modifiers, action costs, and status effects in a flexible way.

## High-Level Model
The game advances through discrete ticks. Each actionable entity owns:
- `next_ready_tick`: the earliest tick when it may act.
- `speed` and other modifiers that influence the delay between actions.

Turn order is driven by a min-oriented priority queue (timeline). The loop repeatedly:
1. Pops the entity with the smallest `next_ready_tick`.
2. Advances the global clock to that tick.
3. Executes the entity's chosen action.
4. Recomputes the entity's `next_ready_tick` using:
   - base delay for the action kind (move, attack, cast, wait, etc.),
   - adjustments from `speed` stats, buffs, debuffs, or status ailments,
   - optional randomness that is fully committed in advance for zk reproducibility.
5. Pushes the entity back into the queue if it remains active.

This structure mirrors a timeline simulation and keeps ordering deterministic given identical inputs.

## Active Entity Set
To avoid maintaining every entity in the queue:
- Maintain an "active set" centered on the player (e.g. square of radius `n`).
- Entities entering the region are (re)activated: compute their `next_ready_tick` from the current clock and insert them into the queue.
- Entities leaving the region are deactivated: mark them inactive and remove or lazily skip them when popped.
- Non-active entities may be updated through coarse background ticks if world logic requires it, but they do not interact with the main timeline until reactivated.

This keeps the queue compact and focuses computation on nearby actors.

## ZK Proof Considerations
- **Determinism**: All delay calculations must be pure functions of public/committed inputs (stats, action kind, RNG commitments). No hidden randomness.
- **Integer Arithmetic**: Express action timing in integer ticks to simplify circuits; avoid divisions where possible by pre-scaling constants.
- **Priority Selection**: For small active sets, a fixed-size array with an `argmin` check is circuit-friendly. For larger sets, consider Merkle-ized heaps or successive pairwise comparisons with proofs that the chosen entity indeed has the minimal tick.
- **Activation Rules**: Encode the activation predicate (position within `n×n`, visibility, etc.) explicitly so the circuit can enforce correct membership changes each step.
- **State Commitments**: Hash the queue contents and global clock into the state commitment so verifiers can confirm the transition was applied to the exact ordering.

## Edge Cases and Extensions
- Handle ties in `next_ready_tick` deterministically (e.g. break by entity id or committed initiative roll).
- Support "skip turn" actions by setting the new tick to `current_tick + base_delay` without a state change.
- Allow immediate requeues (extra turns) by assigning zero cost actions, as long as they remain within circuit bounds.
- Integrate environmental hazards or traps as entities that live in the active set and schedule themselves with appropriate delays.

## Next Steps
1. Define the concrete tick units, base delays, and speed modifiers.
2. Specify the activation radius and how often it reevaluates as the player moves.
3. Design data structures for the queue that are efficient both in Rust and in the zk circuit representation.
4. Prototype reducer integration: advance the global clock, process actions, and update `TurnState`.
