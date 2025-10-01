# Dungeon TODOs

## Game Core

- Env / oracle traits: formalise the read-only interfaces that `ActionTransition` hooks and `CommandContext` expose (e.g., `MapOracle`, `TablesOracle`, status lookups) and provide a single `Env` aggregate that the reducer and commands can depend on.
- Witness delta pipeline: introduce the structs/enums that record which state fields and oracle reads each transition performed, and make `step` return (or expose) those deltas for the runtime/prover to consume.
- Reducer instrumentation: add hooks for structured logging/telemetry so every step can emit the actor, action kind, transition phase timings, and attach the witness record for replay/audit.
- State commitment helpers: codify the canonical field ordering (likely via iterators or derive helpers) so downstream crates can hash or serialise state deterministically without duplicating layout knowledge.
- Action implementations: flesh out the `ActionTransition` logic per action (movement bounds, combat math, inventory mutations, interaction rules) and wire their validation errors into `StepError` so invariants are enforced in both software and proofs.
