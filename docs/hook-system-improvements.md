# Hook System Improvements

This document outlines potential improvements and known limitations of the current post-execution hook system in the runtime.

## Current Implementation Status

The hook system is **functional and production-ready** for basic game logic:

✅ **Hook Chaining**: Hooks can trigger other hooks via `next_hook_names()`
✅ **Multiple Actions**: Hooks can generate multiple system actions via `create_actions()`
✅ **Priority-based Execution**: Root hooks execute in priority order
✅ **Cycle Prevention**: MAX_DEPTH limit prevents infinite loops
✅ **Entry Hook Pattern**: Hooks can chain without generating actions (e.g., DamageHook)

## Known Limitations and Improvement Opportunities

### 1. Root Hook vs Lookup Hook Distinction

**Problem**: All registered hooks currently execute as root hooks on every action.

```rust
// Current structure
pub struct HookRegistry {
    hooks: Arc<[Arc<dyn PostExecutionHook>]>,  // All hooks execute every time
}

// Usage
let registry = HookRegistry::new(vec![
    Arc::new(ActionCostHook),   // Root: should run every action
    Arc::new(ActivationHook),   // Root: should run every action
    Arc::new(DamageHook),       // Root: should run every action
    Arc::new(DeathCheckHook),   // Should only run after damage!
    Arc::new(BleedingHook),     // Should only run after damage!
]);
```

**Issues**:
- Performance overhead: `should_trigger()` called for every hook on every action
- Semantic confusion: Some hooks are meant to be chained only, not root
- As hook count grows (20+), this becomes significant overhead

**Proposed Solution**:

```rust
pub struct HookRegistry {
    root_hooks: Vec<Arc<dyn PostExecutionHook>>,      // Execute every action
    lookup_table: HashMap<&'static str, Arc<dyn PostExecutionHook>>,  // For chaining
}

impl HookRegistry {
    pub fn new(
        root_hooks: Vec<Arc<dyn PostExecutionHook>>,
        all_hooks: Vec<Arc<dyn PostExecutionHook>>,
    ) -> Self {
        let lookup_table = all_hooks
            .iter()
            .map(|h| (h.name(), Arc::clone(h)))
            .collect();

        Self {
            root_hooks,
            lookup_table,
        }
    }

    pub fn execute_hooks(&self, delta, state, oracles) {
        let env = oracles.as_game_env();

        // Only execute root hooks
        for hook in self.root_hooks.iter() {
            hook.execute(delta, state, oracles, &env, self, 0)?;
        }
    }

    pub fn find(&self, name: &str) -> Option<&Arc<dyn PostExecutionHook>> {
        self.lookup_table.get(name)
    }
}

// Usage
let registry = HookRegistry::new(
    vec![
        Arc::new(ActionCostHook),
        Arc::new(ActivationHook),
        Arc::new(DamageHook),
    ],  // Root: execute every action
    vec![
        Arc::new(ActionCostHook),
        Arc::new(ActivationHook),
        Arc::new(DamageHook),
        Arc::new(DeathCheckHook),   // Lookup only: called from chain
        Arc::new(BleedingHook),     // Lookup only: called from chain
        Arc::new(OnDeathHook),      // Lookup only: called from chain
    ],  // All hooks for lookup
);
```

**Benefits**:
- Clear semantic distinction: root hooks vs chain-only hooks
- Performance: Only root hooks checked every action
- Scalability: Can add many chain-only hooks without overhead

**Complexity**: Medium - requires API change in RuntimeBuilder

---

### 2. Context Information Limitations

**Problem**: `HookContext` doesn't provide enough information for complex hooks.

```rust
// Current
pub struct HookContext<'a> {
    pub delta: &'a StateDelta,
    pub state: &'a GameState,
    pub oracles: &'a OracleManager,
}
```

**Issues**:

#### 2.1 Multi-Target Ambiguity

When multiple entities are affected, hooks can't distinguish which entity to process:

```rust
// Scenario: AoE attack hits Goblin and Orc
impl DeathCheckHook {
    fn should_trigger(&self, ctx: &HookContext) -> bool {
        // Which entity should I check?
        // Both took damage, but I need to check each individually
        ctx.state.entities.iter().any(|e| e.hp <= 0)  // Checks all entities!
    }
}
```

**Proposed Solution A: Target Entity in Context**

```rust
pub struct HookContext<'a> {
    pub delta: &'a StateDelta,
    pub state: &'a GameState,
    pub oracles: &'a OracleManager,
    pub target: Option<EntityId>,  // NEW: Which entity is this hook evaluating?
}

// Registry executes hooks per affected entity
impl HookRegistry {
    pub fn execute_hooks(&self, delta, state, oracles) {
        // Extract affected entities from delta
        let affected_entities = delta.extract_affected_entities();

        for entity_id in affected_entities {
            let ctx = HookContext {
                delta,
                state,
                oracles,
                target: Some(entity_id),  // Set target
            };

            for hook in self.root_hooks.iter() {
                hook.execute_with_context(&ctx, ...)?;
            }
        }
    }
}
```

**Proposed Solution B: Rich Delta Information**

```rust
pub struct StateDelta {
    pub action: Action,
    pub turn: TurnDelta,
    pub entities: EntitiesDelta,
    pub world: WorldDelta,

    // NEW: Semantic event information
    pub events: Vec<GameEvent>,
}

pub enum GameEvent {
    DamageDealt { source: EntityId, target: EntityId, amount: u32 },
    Healed { target: EntityId, amount: u32 },
    Moved { entity: EntityId, from: Position, to: Position },
    Died { entity: EntityId, killer: Option<EntityId> },
    // ...
}

// Usage in hooks
impl DeathCheckHook {
    fn should_trigger(&self, ctx: &HookContext) -> bool {
        ctx.delta.events.iter().any(|e| matches!(e, GameEvent::DamageDealt { .. }))
    }

    fn create_actions(&self, ctx: &HookContext) -> Vec<Action> {
        ctx.delta.events
            .iter()
            .filter_map(|e| {
                if let GameEvent::DamageDealt { target, .. } = e {
                    if ctx.state.entities.actor(*target)?.hp <= 0 {
                        Some(Action::new(EntityId::SYSTEM, SetDeathAction::new(*target)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}
```

**Benefits**:
- Solution A: Simple, explicit target tracking
- Solution B: Rich event information, more flexible

**Complexity**: High - requires significant refactoring of StateDelta or execution flow

---

### 3. Inter-Hook Communication

**Problem**: Hooks cannot pass information to subsequent hooks in the chain.

```rust
// Scenario: Damage hook wants to tell death_check about critical hits
impl DamageHook {
    fn create_actions(&self, ctx: &HookContext) -> Vec<Action> {
        let was_critical = calculate_critical(...);
        // No way to pass this to death_check!
        vec![]
    }
}

impl DeathCheckHook {
    fn should_trigger(&self, ctx: &HookContext) -> bool {
        // Can't know if it was a critical hit
        // Would need to recalculate or check state
    }
}
```

**Proposed Solution: Hook Metadata**

```rust
pub struct HookContext<'a> {
    pub delta: &'a StateDelta,
    pub state: &'a GameState,
    pub oracles: &'a OracleManager,
    pub metadata: &'a mut HashMap<String, Box<dyn Any>>,  // NEW: Shared data
}

// Usage
impl DamageHook {
    fn execute_next_hooks(&self, delta, state, oracles, env, registry, depth) {
        let mut metadata = HashMap::new();
        metadata.insert("was_critical".to_string(), Box::new(true));
        metadata.insert("damage_type".to_string(), Box::new(DamageType::Fire));

        for name in self.next_hook_names() {
            if let Some(hook) = registry.find(name) {
                let ctx = HookContext { delta, state, oracles, metadata: &mut metadata };
                hook.execute_with_context(&ctx, ...)?;
            }
        }
    }
}

impl OnDeathHook {
    fn create_actions(&self, ctx: &HookContext) -> Vec<Action> {
        let was_critical = ctx.metadata
            .get("was_critical")
            .and_then(|v| v.downcast_ref::<bool>())
            .copied()
            .unwrap_or(false);

        if was_critical {
            // Special on-death effect for critical kills
        }
        vec![]
    }
}
```

**Alternative: Typed Metadata**

```rust
#[derive(Default)]
pub struct HookMetadata {
    pub damage_info: Option<DamageInfo>,
    pub critical_hit: bool,
    pub status_effects: Vec<StatusEffect>,
    // Strongly typed fields
}

pub struct HookContext<'a> {
    pub delta: &'a StateDelta,
    pub state: &'a GameState,
    pub oracles: &'a OracleManager,
    pub metadata: &'a mut HookMetadata,  // Type-safe!
}
```

**Benefits**:
- Hooks can share computed information
- Avoid redundant calculations
- Enable complex effect interactions

**Complexity**: Medium - requires threading metadata through execution

**Trade-off**: Type safety vs flexibility (HashMap vs struct)

---

### 4. Error Handling and Criticality

**Problem**: All hook failures are treated equally and logged but ignored.

```rust
// Current
pub fn execute_hooks(&self, delta, state, oracles) {
    for hook in self.hooks.iter() {
        if let Err(e) = hook.execute(...) {
            error!("Hook execution failed");  // Just log and continue
        }
    }
}
```

**Issues**:
- Critical hooks (ActionCost) failing is a serious problem
- Optional hooks (cosmetic effects) failing is fine
- No way to distinguish importance levels
- Game state can become inconsistent if critical hooks fail

**Proposed Solution: Hook Criticality Levels**

```rust
pub enum HookCriticality {
    /// Hook failure should fail the entire action
    Critical,

    /// Hook failure should be logged as error but allow continuation
    Important,

    /// Hook failure is expected and can be silently ignored
    Optional,
}

pub trait PostExecutionHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn priority(&self) -> i32 { 0 }

    /// Defines how important this hook is
    fn criticality(&self) -> HookCriticality {
        HookCriticality::Important  // Default: log but continue
    }

    // ... rest of trait
}

// Usage
impl ActionCostHook {
    fn criticality(&self) -> HookCriticality {
        HookCriticality::Critical  // Must succeed!
    }
}

impl CosmeticEffectHook {
    fn criticality(&self) -> HookCriticality {
        HookCriticality::Optional  // Who cares if particles fail
    }
}

// Registry execution
pub fn execute_hooks(&self, delta, state, oracles) -> Result<()> {
    for hook in self.hooks.iter() {
        match hook.execute(...) {
            Ok(()) => continue,
            Err(e) => match hook.criticality() {
                HookCriticality::Critical => {
                    error!("Critical hook {} failed: {}", hook.name(), e);
                    return Err(e);  // Propagate error
                }
                HookCriticality::Important => {
                    error!("Hook {} failed: {}", hook.name(), e);
                    // Continue to next hook
                }
                HookCriticality::Optional => {
                    debug!("Optional hook {} failed: {}", hook.name(), e);
                    // Continue silently
                }
            }
        }
    }
    Ok(())
}
```

**Benefits**:
- Clear error semantics
- Can abort action if critical hook fails
- Better production reliability
- Easier debugging (know which failures matter)

**Complexity**: Low - simple enum addition

---

### 5. Hook Execution Order and Dependencies

**Problem**: Only `priority` for root hooks, no dependency declaration.

```rust
// Current: Only priority controls order
impl ActionCostHook {
    fn priority(&self) -> i32 { -100 }
}

impl ActivationHook {
    fn priority(&self) -> i32 { -10 }
}
```

**Issues**:
- Magic numbers: Why -100 vs -10?
- Implicit dependencies: ActionCost must run before Activation
- No validation of dependency correctness
- Hard to add new hooks in between

**Proposed Solution: Explicit Dependencies**

```rust
pub trait PostExecutionHook: Send + Sync {
    fn name(&self) -> &'static str;

    /// Hooks that must execute before this one (at root level)
    fn depends_on(&self) -> &[&'static str] {
        &[]
    }

    fn next_hook_names(&self) -> &[&'static str] {
        &[]
    }

    // ... rest
}

impl ActivationHook {
    fn depends_on(&self) -> &[&'static str] {
        &["action_cost"]  // Must run after ActionCost
    }
}

// Registry builds dependency graph and validates
impl HookRegistry {
    pub fn new(hooks: Vec<Arc<dyn PostExecutionHook>>) -> Result<Self, DependencyError> {
        let graph = build_dependency_graph(&hooks)?;
        let sorted = topological_sort(graph)?;  // Ensure no cycles

        Ok(Self { hooks: sorted.into() })
    }
}
```

**Benefits**:
- Self-documenting dependencies
- Compile-time cycle detection
- Easier to insert new hooks
- Less error-prone than priority numbers

**Complexity**: Medium - requires dependency graph implementation

---

### 6. Hook Testing and Isolation

**Problem**: Hooks are tightly coupled to the full execution environment.

**Issues**:
- Hard to unit test hooks in isolation
- Need full GameState, OracleManager setup for simple tests
- Can't easily mock next hooks in chain
- Integration tests are slow

**Proposed Solution: Test Utilities**

```rust
// crates/runtime/src/hooks/testing.rs

pub struct HookTestContext {
    state: GameState,
    oracles: OracleManager,
    next_hooks: HashMap<&'static str, MockHook>,
}

impl HookTestContext {
    pub fn new() -> Self {
        Self {
            state: GameState::default(),
            oracles: OracleManager::test_manager(),
            next_hooks: HashMap::new(),
        }
    }

    pub fn with_state(mut self, state: GameState) -> Self {
        self.state = state;
        self
    }

    pub fn mock_next_hook(mut self, name: &'static str, mock: MockHook) -> Self {
        self.next_hooks.insert(name, mock);
        self
    }

    pub fn execute(&mut self, hook: &dyn PostExecutionHook, action: Action) -> HookTestResult {
        let delta = StateDelta::from_states(action, &self.state, &self.state);
        let ctx = HookContext {
            delta: &delta,
            state: &self.state,
            oracles: &self.oracles,
        };

        let triggered = hook.should_trigger(&ctx);
        let actions = if triggered { hook.create_actions(&ctx) } else { vec![] };

        HookTestResult {
            triggered,
            actions,
            next_hooks_called: vec![],  // Track which were called
        }
    }
}

// Usage in tests
#[test]
fn damage_hook_chains_to_death_check() {
    let mut test_ctx = HookTestContext::new()
        .mock_next_hook("death_check", MockHook::new());

    let hook = DamageHook;
    let action = Action::new(
        EntityId::PLAYER,
        ActionKind::Attack(AttackAction::new(...)),
    );

    let result = test_ctx.execute(&hook, action);

    assert!(result.triggered);
    assert_eq!(result.next_hooks_called, vec!["death_check"]);
}
```

**Benefits**:
- Easier unit testing
- Faster tests (no full engine needed)
- Better isolation and mocking
- Test-driven hook development

**Complexity**: Medium - new testing infrastructure

---

### 7. Hook Observability and Debugging

**Problem**: Limited visibility into hook execution at runtime.

**Issues**:
- Hard to debug why a hook didn't trigger
- Can't trace full execution chain
- No metrics on hook performance
- Difficult to diagnose production issues

**Proposed Solution: Hook Tracing**

```rust
pub struct HookExecutionTrace {
    pub hook_name: &'static str,
    pub triggered: bool,
    pub actions_created: usize,
    pub next_hooks_triggered: Vec<&'static str>,
    pub duration: Duration,
    pub error: Option<String>,
}

pub struct HookContext<'a> {
    pub delta: &'a StateDelta,
    pub state: &'a GameState,
    pub oracles: &'a OracleManager,
    pub tracer: Option<&'a mut HookTracer>,  // Optional tracing
}

impl HookTracer {
    pub fn record_execution(&mut self, trace: HookExecutionTrace) {
        self.traces.push(trace);
    }

    pub fn print_execution_tree(&self) {
        // Print nice tree view of hook chain
    }
}

// Usage
let mut tracer = HookTracer::new();
registry.execute_hooks(delta, state, oracles, Some(&mut tracer))?;

println!("{}", tracer.print_execution_tree());
// Output:
// ActionCostHook [2ms] ✓
// ActivationHook [1ms] ✗ (not triggered)
// DamageHook [0ms] ✓
//   └─ death_check [3ms] ✓
//      └─ on_death [5ms] ✓
//         └─ damage [1ms] ✗ (no targets)
```

**Benefits**:
- Better debugging tools
- Performance profiling
- Production monitoring
- Easier troubleshooting

**Complexity**: Medium - tracing infrastructure

---

### 8. Dynamic Hook Registration

**Problem**: All hooks must be registered at runtime creation.

**Issues**:
- Can't add/remove hooks at runtime
- Modding support is limited
- Hot-reloading hooks is impossible
- Testing specific hook combinations is awkward

**Proposed Solution: Runtime Hook Management**

```rust
pub struct RuntimeHandle {
    command_tx: Sender<Command>,
    event_rx: Receiver<GameEvent>,
    hook_manager: Arc<RwLock<HookManager>>,  // NEW
}

impl RuntimeHandle {
    pub async fn add_hook(&self, hook: Arc<dyn PostExecutionHook>) -> Result<()> {
        let mut manager = self.hook_manager.write().await;
        manager.add_hook(hook)?;
        Ok(())
    }

    pub async fn remove_hook(&self, name: &str) -> Result<()> {
        let mut manager = self.hook_manager.write().await;
        manager.remove_hook(name)?;
        Ok(())
    }

    pub async fn enable_hook(&self, name: &str) {
        let mut manager = self.hook_manager.write().await;
        manager.enable(name);
    }

    pub async fn disable_hook(&self, name: &str) {
        let mut manager = self.hook_manager.write().await;
        manager.disable(name);
    }
}
```

**Benefits**:
- Modding support
- A/B testing of game mechanics
- Dynamic gameplay changes
- Better development workflow

**Complexity**: High - thread-safety, synchronization challenges

---

## Priority Recommendations

### 🔴 High Priority (Do Soon)
1. **Root vs Lookup Separation** (Issue #1)
   - Most impactful for performance and clarity
   - Relatively straightforward to implement
   - Prevents technical debt as hooks grow

2. **Hook Criticality** (Issue #4)
   - Important for production reliability
   - Low complexity, high value
   - Prevents silent game state corruption

### 🟡 Medium Priority (Do Later)
3. **Rich Delta Events** (Issue #2, Solution B)
   - Enables complex hook logic
   - Significant refactoring but high value
   - Needed for advanced gameplay features

4. **Hook Testing Utilities** (Issue #6)
   - Improves development velocity
   - Reduces bug introduction rate
   - Pay-off increases with more hooks

### 🟢 Low Priority (Nice to Have)
5. **Hook Metadata** (Issue #3)
   - Only needed for very specific interactions
   - Can be worked around with creative delta usage
   - Adds complexity to API

6. **Explicit Dependencies** (Issue #5)
   - Current priority system works fine for small hook counts
   - Mainly helps documentation and validation
   - Can defer until hook count > 20

7. **Hook Tracing** (Issue #7)
   - Very useful for debugging but can use logs for now
   - Consider when production issues arise
   - Integration with tracing crate is straightforward

8. **Dynamic Hook Registration** (Issue #8)
   - Not needed for core game
   - Only valuable for modding/plugins
   - Significant complexity for niche use case

## Migration Path

When implementing improvements:

1. **Start with #1 (Root/Lookup)**: Clean foundation for all other improvements
2. **Add #4 (Criticality)**: Small change, immediate safety benefit
3. **Implement #2 (Rich Delta)**: Enables powerful hook patterns
4. **Build #6 (Testing)**: Improves quality of new hooks
5. **Consider others**: Based on actual needs discovered during development

## Conclusion

The current hook system is **production-ready** for the game's core needs. The improvements listed here are **optimizations and enhancements** that can be added incrementally as the game grows in complexity.

Priority should be:
1. Ship game content with current system
2. Identify pain points through real usage
3. Implement improvements that solve actual problems
4. Avoid premature optimization

The system's foundation (chaining, multiple actions, priority) is solid. Everything else is refinement.
