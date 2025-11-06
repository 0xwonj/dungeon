# Action System Design v2.0

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Design Philosophy](#2-design-philosophy)
3. [Core Architecture](#3-core-architecture)
4. [Type System](#4-type-system)
5. [Effect System](#5-effect-system)
6. [Targeting System](#6-targeting-system)
7. [Execution Pipeline](#7-execution-pipeline)
8. [Data Format](#8-data-format)
9. [Extension Guide](#9-extension-guide)
10. [Implementation Plan](#10-implementation-plan)

---

## 1. Executive Summary

### Vision

A **data-driven, effect-based action system** inspired by industry-leading games (WoW, Diablo, PoE) that achieves:

- **Composability**: Actions as combinations of effects
- **Extensibility**: New actions without code changes
- **Type Safety**: Compile-time guarantees where possible
- **Performance**: ZK-circuit friendly (static dispatch)

### Key Innovation

**Effect-First Architecture**: Actions don't execute logic directly—they apply a sequence of **Effects** (damage, heal, status, movement, etc.). This allows:

```ron
// Vampiric Strike = Damage + Heal
effects: [
    Damage(formula: WeaponDamage(1.0), type: Physical),
    Heal(formula: PercentOfDamageDealt(0.5)),
]

// Charge = Move + Damage + Stun
effects: [
    Move(displacement: TowardDirection(5)),
    Damage(formula: StatScaling(Strength, 0.8)),
    ApplyStatus(Stunned, duration: 2),
]
```

### Industry Alignment

| Game | Pattern | Our Adoption |
|------|---------|--------------|
| **WoW** | Spell Effects (max 3) | Effect List (unlimited) |
| **Diablo 3** | Formula System | DamageFormula/HealFormula |
| **PoE** | Tag + Support Gems | Tag System |
| **Roguelikes** | Status Effects | StatusEffect with duration |

---

## 2. Design Philosophy

### 2.1 Core Principles

**1. Composition over Hierarchy**
- Actions composed of effects, not subclasses
- Effects are atomic, composable units
- No deep inheritance trees

**2. Data over Code**
- Behavior defined in RON files
- Formulas express relationships
- Code provides mechanisms, data provides policy

**3. Explicitness over Magic**
- All effects visible in profile
- No hidden behaviors
- Clear execution order

**4. Simplicity over Generality**
- Status effects handle most temporary effects
- Complex systems only when needed
- Pragmatic over theoretical perfection

### 2.2 Design Constraints

**ZK-Friendly Requirements**:
- ✅ No trait objects (no `dyn Trait`)
- ✅ All dispatch via `enum match` (static)
- ✅ Deterministic execution (no randomness leaks)
- ✅ Serializable state (no pointers)

**Pure Functional Core**:
- ✅ No I/O in `game-core`
- ✅ All external data via Oracles
- ✅ Stateless execution (state passed in/out)

---

## 3. Core Architecture

### 3.1 Layer Model

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: ActionKind (What can be done)                      │
│                                                              │
│ - Enum of all action types: Move, MeleeAttack, Fireball... │
│ - Stored in Actor.abilities: Vec<ActionAbility>            │
│ - Explicit numbering for ZK circuit optimization           │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: CharacterAction (Execution instance)               │
│                                                              │
│ - CharacterAction { actor, kind, targets }                  │
│ - Created by get_available_actions()                        │
│ - Contains minimal execution context                        │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: ActionProfile (Action specification)               │
│                                                              │
│ - Loaded from RON files via TablesOracle                   │
│ - Contains: targeting, costs, effects, tags                │
│ - Data-driven behavior definition                          │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Effect System (What happens)                       │
│                                                              │
│ - ActionEffect enum: Damage, Heal, Status, Movement...     │
│ - Composable, reusable effect definitions                  │
│ - Effects applied in sequence to resolved targets          │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Data Flow

```
User Input → CharacterAction
              ↓
         Load ActionProfile (RON)
              ↓
         Resolve Targeting
              ↓
         For each Target:
           For each Effect:
             Apply Effect → Update State
              ↓
         Return ActionResult
```

---

## 4. Type System

### 4.1 ActionKind (Ability Definition)

```rust
/// All action types in the game.
/// Explicit numbering enables bitflag optimizations in ZK circuits.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind {
    // Basic Actions (0-9)
    Move = 0,
    Wait = 1,
    Interact = 2,
    UseItem = 3,

    // Melee Combat (10-29)
    MeleeAttack = 10,
    PowerAttack = 11,
    Backstab = 12,
    Cleave = 13,
    Whirlwind = 14,

    // Ranged Combat (30-49)
    RangedAttack = 30,
    AimedShot = 31,
    MultiShot = 32,
    PiercingShot = 33,

    // Offensive Magic (50-79)
    Fireball = 50,
    Lightning = 51,
    ChainLightning = 52,
    IceSpike = 53,
    Meteor = 54,
    Earthquake = 55,

    // Support Magic (80-99)
    Heal = 80,
    MassHeal = 81,
    Shield = 82,
    Haste = 83,
    Teleport = 84,

    // Control (100-119)
    Stun = 100,
    Root = 101,
    Silence = 102,
    Slow = 103,
    Fear = 104,

    // Mobility (120-139)
    Dash = 120,
    Charge = 121,
    Blink = 122,
    Leap = 123,

    // Social (140-159)
    Intimidate = 140,
    Rally = 141,
    Taunt = 142,

    // Summon (160-179)
    SummonSkeleton = 160,
    CallAllies = 161,
    SummonWolf = 162,

    // Special (180-199)
    VampiricStrike = 180,
    LifeDrain = 181,
    Execute = 182,  // Damage scales with missing HP
    Resurrect = 183,
    Transform = 184,
    Polymorph = 185,
    PoisonStrike = 186,
}

impl ActionKind {
    pub const COUNT: usize = 50;

    pub fn category(&self) -> ActionCategory {
        match *self as u16 {
            0..=9 => ActionCategory::Basic,
            10..=29 => ActionCategory::MeleeCombat,
            30..=49 => ActionCategory::RangedCombat,
            50..=79 => ActionCategory::MagicOffensive,
            80..=99 => ActionCategory::MagicSupport,
            100..=119 => ActionCategory::Control,
            120..=139 => ActionCategory::Mobility,
            140..=159 => ActionCategory::Social,
            160..=179 => ActionCategory::Summon,
            180..=199 => ActionCategory::Special,
            _ => ActionCategory::Special,
        }
    }
}
```

### 4.2 CharacterAction (Execution Instance)

```rust
/// A concrete action ready for execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterAction {
    pub actor: EntityId,
    pub kind: ActionKind,
    pub targets: ActionTargets,
}

/// Action target specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionTargets {
    None,
    Self_,
    Single(EntityId),
    Position(Position),
    Direction(CardinalDirection),
    Multi(Vec<EntityId>),
}
```

### 4.3 ActionTag (Multi-dimensional Classification)

```rust
/// Tags for cross-cutting concerns (rules, AI, synergies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionTag {
    // Damage Types
    Physical, Fire, Cold, Lightning, Poison, Arcane,

    // Action Types
    Attack, Spell, Movement,

    // Delivery Methods
    Melee, Ranged, Projectile, Aoe,

    // Schools
    Offensive, Defensive, Utility,

    // Special Flags
    Channeled, Instant, Interruptible,
}
```

---

## 5. Effect System

### 5.1 ActionEffect (What Happens)

```rust
/// Atomic effects that actions can apply.
#[derive(Clone, Debug, PartialEq)]
pub enum ActionEffect {
    // Damage & Healing
    Damage {
        formula: DamageFormula,
        damage_type: DamageType,
        can_crit: bool,
    },

    Heal {
        formula: HealFormula,
        overheal_allowed: bool,
    },

    DrainLife {
        damage_formula: DamageFormula,
        lifesteal_percent: u32,
    },

    // Resources
    RestoreResource {
        resource: ResourceType,
        amount: u32
    },

    DrainResource {
        resource: ResourceType,
        amount: u32,
        transfer_to_caster: bool
    },

    // Status Effects (managed in ActorState.status_effects)
    ApplyStatus {
        status: StatusEffectKind,  // See: state/types/status.rs
        duration: Tick,
    },

    RemoveStatus {
        status: StatusEffectKind,
    },

    // Movement
    Move {
        displacement: Displacement
    },

    Teleport {
        destination: TeleportDestination
    },

    Swap,

    Knockback {
        direction: KnockbackDirection,
        distance: u32
    },

    Pull {
        distance: u32
    },

    // Summon & Transform
    Summon {
        template_id: String,
        count: u32,
        duration: Option<Tick>
    },

    Transform {
        into_template: String,
        duration: Option<Tick>
    },

    Resurrect {
        hp_percent: u32,
        mana_percent: u32
    },

    // Utility
    Interact {
        interaction_type: InteractionType
    },

    Conditional {
        condition: Condition,
        then_effects: Vec<ActionEffect>,
        else_effects: Vec<ActionEffect>
    },
}
```

### 5.2 Formulas

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum DamageFormula {
    Flat(u32),
    WeaponDamage { multiplier: f32 },
    StatScaling { stat: Stat, multiplier: f32, flat_bonus: u32 },
    Complex { base: u32, stat_scaling: Vec<(Stat, f32)>, weapon_damage_mult: f32 },
    PercentMissingHp(u32),
    PercentCurrentHp(u32),
    PercentMaxHp(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HealFormula {
    Flat(u32),
    StatScaling { stat: Stat, multiplier: f32, flat_bonus: u32 },
    PercentMaxHp(u32),
    PercentMissingHp(u32),
    PercentOfDamageDealt(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageType {
    Physical,
    Fire,
    Cold,
    Lightning,
    Poison,
    Arcane,
    True,  // Ignores all resistances
}
```

---

## 6. Targeting System

### 7.1 TargetingMode

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum TargetingMode {
    None,
    Self_,
    SingleTarget { range: u32, filter: TargetFilter, requires_los: bool },
    Directional { range: u32, width: Option<u32> },
    Aoe { center: AoeCenter, shape: AoeShape, radius: u32, filter: TargetFilter },
    Multi { max_targets: u32, range: u32, filter: TargetFilter, selection: MultiTargetSelection },
    Chain { initial_target: Box<TargetingMode>, max_bounces: u32, bounce_range: u32, filter: TargetFilter },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AoeShape {
    Circle,
    Cone,
    Line,
    Cross,
    Ring,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetFilter {
    pub teams: TeamFilter,
    pub types: Vec<EntityType>,
    pub exclude_self: bool,
    pub exclude_dead: bool,
}
```

---

## 7. Execution Pipeline

### 8.1 ActionProfile

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct ActionProfile {
    // Identity
    pub kind: ActionKind,
    pub category: ActionCategory,
    pub tags: Vec<ActionTag>,
    pub name: String,
    pub description: String,

    // Targeting
    pub targeting: TargetingMode,

    // Costs
    pub base_cost: Tick,
    pub resource_costs: Vec<ResourceCost>,

    // Effects
    pub effects: Vec<ActionEffect>,

    // Constraints
    pub requirements: Vec<Requirement>,

    // Metadata
    pub cooldown: Option<Tick>,
    pub icon: Option<String>,
}
```

### 8.2 Execution Flow

```rust
impl CharacterAction {
    fn execute(&self, state: &mut GameState, env: &GameEnv) -> Result<ActionResult, ActionError> {
        // 1. Load profile
        let profile = env.tables().action_profile(self.kind)?;

        // 2. Pre-validation
        self.pre_validate(state, env, &profile)?;

        // 3. Deduct costs
        self.deduct_costs(state, &profile)?;

        // 4. Resolve targets
        let targets = self.resolve_targets(state, env, &profile)?;

        // 5. Apply effects to each target
        let mut result = ActionResult::default();
        for target in targets {
            for effect in &profile.effects {
                self.apply_effect(effect, target, state, env, &mut result)?;
            }
        }

        // 6. Process status effects
        state.tick_status_effects()?;

        Ok(result)
    }
}
```

---

## 8. Data Format

### 9.1 Simple Example: Melee Attack

```ron
ActionProfile(
    kind: MeleeAttack,
    category: MeleeCombat,
    tags: [Attack, Melee, Physical],
    name: "Melee Attack",

    targeting: SingleTarget(
        range: 1,
        filter: TargetFilter(teams: Enemies, types: [Actor]),
        requires_los: false,
    ),

    base_cost: 100,
    resource_costs: [(resource: Stamina, amount: 10)],

    effects: [
        Damage(
            formula: WeaponDamage(multiplier: 1.0),
            damage_type: Physical,
            can_crit: true,
        ),
    ],

    requirements: [],
)
```

### 9.2 Complex Example: Vampiric Strike

```ron
ActionProfile(
    kind: VampiricStrike,
    tags: [Attack, Melee, Lifesteal],

    targeting: SingleTarget(range: 1, ...),

    effects: [
        DrainLife(
            damage_formula: WeaponDamage(0.8),
            lifesteal_percent: 50,
        ),
    ],

    cooldown: Some(300),
)
```

### 9.3 Complex Example: Poison Strike

```ron
ActionProfile(
    kind: PoisonStrike,
    tags: [Attack, Melee, Poison],

    targeting: SingleTarget(range: 1, ...),

    effects: [
        Damage(
            formula: WeaponDamage(0.8),
            damage_type: Physical,
            can_crit: true,
        ),
        ApplyStatus(status: Poisoned, duration: 500),
    ],

    cooldown: Some(200),
)
```

### 9.4 Complex Example: Haste Buff

```ron
ActionProfile(
    kind: Haste,
    tags: [Spell, Buff, Support],

    targeting: SingleTarget(range: 5, filter: TargetFilter(teams: Allies, ...)),

    effects: [
        ApplyStatus(status: Hasted, duration: 500),
    ],

    base_cost: 100,
    resource_costs: [(resource: Mana, amount: 30)],
)
```

### 9.5 Complex Example: Charge

```ron
ActionProfile(
    kind: Charge,
    tags: [Movement, Attack, Melee],

    targeting: Directional(range: 5, width: Some(1)),

    effects: [
        Move(displacement: TowardDirection(5)),
        Damage(formula: StatScaling(Strength, 0.6)),
        ApplyStatus(status: Stunned, duration: 200),
    ],
)
```

---

## 9. Extension Guide

### 10.1 Adding Simple Action

**Goal**: Add "Frost Nova" (AoE freeze)

```rust
// Step 1: Add to ActionKind
FrostNova = 187,

// Step 2: Create RON profile
ActionProfile(
    kind: FrostNova,
    category: MagicOffensive,
    tags: [Spell, Cold, Aoe],

    targeting: Aoe(
        center: Caster,
        shape: Circle,
        radius: 3,
        filter: TargetFilter(teams: Enemies),
    ),

    effects: [
        Damage(formula: Flat(20), damage_type: Cold),
        ApplyStatus(status: Rooted, duration: 300),
    ],

    base_cost: 150,
    resource_costs: [(resource: Mana, amount: 40)],
)
```

Done! No code changes needed.

### 9.2 Adding New Effect

```rust
// Step 1: Add to ActionEffect enum
Taunt { duration: Tick, threat_multiplier: u32 },

// Step 2: Implement in apply_effect
ActionEffect::Taunt { duration, threat_multiplier } => {
    apply_taunt(target, self.actor, *duration, *threat_multiplier, state)?;
}

// Step 3: Use in profile
effects: [Taunt(duration: 300, threat_multiplier: 200)],
```

---

## 10. Implementation Plan

### Phase 1: Foundation
- Define type system (ActionKind, CharacterAction, ActionTargets)
- Define ActionProfile structure
- Define ActionEffect enum (basic effects)
- Extend TablesOracle

### Phase 2: Basic Effects
- Implement Damage, Heal, Move
- Implement ApplyStatus (using existing StatusEffect system)
- Create basic RON profiles

### Phase 3: Targeting System
- Implement TargetingMode resolution
- Update get_available_actions()

### Phase 4: Advanced Effects
- DrainLife, Teleport, Knockback
- Advanced RON profiles

### Phase 5: Status Integration
- Integrate with existing StatusEffect system
- Implement ApplyStatus/RemoveStatus effects

### Phase 6: Formula System
- DamageFormula/HealFormula evaluation
- Crit, resistance, armor calculations

### Phase 7: Migration
- Remove old CharacterActionKind
- Remove Action::character_legacy()
- Update AI/client

### Phase 8: Content Creation
- Create all action profiles
- Balance pass

### Phase 9: Testing & Polish (Week 9)
- Integration tests
- Performance optimization

### Phase 10: Advanced Features (Week 10+)
- Chain targeting
- Conditional effects
- Summon/Transform mechanics

---

## Conclusion

This effect-based architecture provides:
- **Composability** through effect combinations
- **Extensibility** through data files
- **Type safety** through enums
- **ZK compatibility** through static dispatch

New actions require only RON files, enabling rapid content creation and balance iteration.

## Status Effects

Status effects (Stunned, Poisoned, Hasted, etc.) are managed separately in `ActorState.status_effects`. Actions apply them via the `ApplyStatus` effect. See implementation: `crates/game/core/src/state/types/status.rs`
