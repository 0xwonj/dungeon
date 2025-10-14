# Stat System Architecture v2.0

> **Status:** Stable draft  
>
> **Scope:** Outlines Dungeon’s core principles — how verifiability, fairness, and emergence shape the game’s design vision.

---

## Overview

This document specifies the layered stat system architecture for the dungeon RPG. The system is designed as a unidirectional dependency graph where all calculations flow from core statistics through derived layers to final game mechanics. This architecture ensures determinism, prevents circular dependencies, and maintains a clear separation between permanent state and computed values.

## Design Principles

1. **Single Source of Truth (SSOT)**: Core stats, level, and current resource values are the only permanently stored state
2. **Unidirectional Flow**: Dependencies flow strictly downward through layers (upper layers never depend on lower layers)
3. **Snapshot Consistency**: All derived values are computed and locked at action initiation for deterministic resolution
4. **Deterministic Computation**: All calculations are pure functions of the input state
5. **Clear Storage Boundaries**: Persistent state vs. computed cache is explicitly defined

## Layer Hierarchy

```
[ Core Stats ]
     ↓
[ Derived Stats ]
     ↓
[ Speed / Cost ]
     ↓
[ Modifiers ]
     ↓
[ Resources (HP / MP / Lucidity) ]
```

### Layer Dependency Rules

- **Core Stats** serve as the foundational source for all calculations
- **Derived Stats**, **Speed/Cost**, **Modifiers**, and **Resources** are all derived from Core Stats
- Bonuses and buffs can apply at each layer
- Inter-layer influence flows exclusively top-down (unidirectional)
- Reverse dependencies (bottom-up) are implemented via **conditions** only, never direct formula references

## Layer Specifications

### Layer 1: Core Stats

**Components**: STR, DEX, CON, INT, WIL, EGO, Level

**Calculation Timing**: Only when state changes occur (equipment change, level up, permanent stat modification)

**Storage**: ✅ Permanently stored

**Purpose**: Foundation for all derived calculations

**Formula**:
```
CoreEffective = (Base + Flat) × (1 + %Inc) × More × Less × Clamp
```

**Details**:
- Base: Character's intrinsic stat value
- Flat: Additive bonuses from equipment, buffs, and environmental effects
- %Inc: Percentage increases (summed before multiplication)
- More/Less: Sequential multipliers applied in order
- Clamp: Final bounds enforcement

### Layer 2: Derived Stats

**Components**: Attack, Accuracy, Evasion, Armor Class (AC), Psionic Power, Focus Efficiency

**Calculation Timing**: At action initiation

**Storage**: ❌ Not persisted (computed on-demand)

**Purpose**: Combat mathematics and action resolution

**Input**: CoreEffective values

**Bonuses Applied**: Equipment, buffs, environmental modifiers (primarily Flat, %Inc, More/Less)

**Example Formulas**:
```
Attack = STR + WeaponScaling + Flat + %Inc + More/Less
Accuracy = DEX + SkillBonus + EquipmentBonus + ...
```

**Characteristics**:
- Results may be cached during action resolution
- Always recomputed from Core Stats when needed

### Layer 3: Speed / Action Cost

**Components**:
- Speed: Physical, Cognitive, Ritual
- Action Cost: final_cost

**Calculation Timing**: At action initiation

**Storage**: ❌ Not persisted (computed on-demand)

**Purpose**: Turn timeline and action economy

**Input**: CoreEffective (direct reference to Core, not Derived)

**Bonuses Applied**: Conditions (buffs/debuffs)

**Formulas**:
```
SpeedKind = base + weighted(CoreEffective) - penalties
final_cost = base_cost × Conditions × 100 / clamp(SpeedKind, 50, 200)
```

**Notes**:
- Speed values are clamped to [50, 200] range
- Conditions (e.g., Slow, Haste) apply as final multipliers

### Layer 4: Modifiers

**Components**: Roll modifiers for skill checks and attribute tests

**Calculation Timing**: At roll execution

**Storage**: ❌ Not persisted (computed on-demand)

**Purpose**: Success/failure adjudication for tests and checks

**Input**: CoreEffective

**Formula**:
```
modifier = floor((CoreEffective - 10) / 2) + Flat + %Inc
```

**Application**:
- Used in d20-style rolls: `d20 + modifier vs DC`
- May be further modified by Lucidity global scaling

### Layer 5: Resources

**Components**:
- Hit Points (HP)
- Mana Points (MP)
- Lucidity

**Calculation Timing**:
- Maximum values: Computed from Core Stats at state change
- Current values: Updated during gameplay

**Storage**:
- ✅ Current values are persisted
- ❌ Maximum values are computed

**Purpose**: Character survivability, action enablement, global scaling

**Input**: CoreEffective, some Derived (e.g., FocusEff may contribute)

**Formulas**:
```
HP_max = (CON × 10) + (Level × CON / 2)
MP_max = (WIL + INT) × 5 + (EGO × 2) + (Level × √WIL)
Lucidity_max = √Level × ((STR+DEX+CON)/2 + (INT+WIL+EGO)×2)
```

**Special Role of Lucidity**:
- Though categorized as a Resource, Lucidity acts as a global modifier
- Can influence Speed and Modifier calculations as a scaling factor
- Example: `EffectiveRoll × (Lucidity / 100)`

**Current Value Management**:
- Current HP/MP/Lucidity fluctuate during combat
- Damage, healing, and resource expenditure modify Current values
- Current values are part of the game state and are saved

## Inter-Layer Dependencies

| From → To | Dependency Type | Example |
|-----------|-----------------|---------|
| Core → Derived | Direct formula | `Attack = f(STR)` |
| Core → Speed | Direct formula | `SpeedPhysical = f(DEX, STR)` |
| Core → Modifier | Direct formula | `mod = floor((DEX-10)/2)` |
| Core → Resources | Direct formula | `HP_max = f(CON, Level)` |
| Derived → Cost | Indirect | `final_cost` uses `SpeedKind` + `Conditions` |
| Resources → Speed | Weak reverse | Low HP → SpeedPenalty (via condition) |
| Lucidity → Modifier | Global scaling | `EffectiveRoll × (Lucidity/100)` |
| Buffs → All Layers | Layer-specific | Haste→Speed, Bless→Derived, etc. |

**Circular Dependency Prevention**:
- Reverse dependencies (lower → upper layer) are forbidden in formulas
- Instead, implement as **conditions** that are applied during computation
- Conditions never directly reference lower-layer computed values

## Bonus Calculation Stack

All bonuses follow the same computational order within each layer:

```
① Flat (additive)
   ↓
② %Inc (percentage increases, summed)
   ↓
③ More (sequential multipliers)
   ↓
④ Less (sequential divisors/reducers)
   ↓
⑤ Clamp (bounds enforcement)
   ↓
⑥ Conditions (final multipliers, e.g., Slow, Overload)
```

**Example**:
```
Base = 10
Flat = +5
%Inc = +20% + +15% = +35%
More = ×1.5
Less = ×0.9
Clamp = [5, 100]

Result = clamp((10 + 5) × 1.35 × 1.5 × 0.9, 5, 100)
       = clamp(15 × 1.35 × 1.5 × 0.9, 5, 100)
       = clamp(27.3375, 5, 100)
       = 27
```

## Computation Flow

### High-Level Flow

```
[ CoreStats + Buffs ]
      ↓ (Flat→%Inc→More→Less→Clamp)
[ CoreEffective ]
      ↓
┌─────────────┬──────────────┬──────────────┬──────────────┐
│ Derived     │ Speed/Cost   │ Modifier     │ ResourcesMax │
│ (Combat)    │ (Timeline)   │ (Rolls)      │ (HP/MP/Luc)  │
└─────────────┴──────────────┴──────────────┴──────────────┘
      ↓ (Conditions applied)
   [ Action Snapshot / Simulation ]
```

### Detailed Computation Sequence

1. **Core Stat Calculation**
   - Input: Base stats + equipment + buffs + environmental effects
   - Apply: Flat → %Inc → More → Less → Clamp
   - Output: CoreEffective values for STR, DEX, CON, INT, WIL, EGO

2. **Derived Stat Calculation**
   - Input: CoreEffective
   - Apply: Layer-specific bonuses (Flat, %Inc, More/Less)
   - Output: Attack, Accuracy, Evasion, AC, PsiPower, FocusEff
   - Cache or use immediately

3. **Speed / Cost Calculation**
   - Input: CoreEffective (direct, bypassing Derived)
   - Apply: Conditions (Haste, Slow, etc.)
   - Formula: `SpeedKind = base + weighted(CoreEffective) - penalties`
   - Formula: `final_cost = base_cost × Conditions × 100 / clamp(SpeedKind, 50, 200)`
   - Output: Speed values and action costs

4. **Modifier Calculation**
   - Input: CoreEffective
   - Apply: Flat bonuses, %Inc
   - Formula: `modifier = floor((CoreEffective - 10) / 2) + Flat + %Inc`
   - Output: Roll modifiers for checks

5. **Resource Calculation**
   - Input: CoreEffective, Level, (optionally Derived stats)
   - Compute: Maximum HP, MP, Lucidity
   - Maintain: Current HP, MP, Lucidity as part of game state
   - Output: Resource pools

## Snapshot Timing

**At Action Initiation**:
- CoreEffective is computed
- Derived stats are computed and locked
- Speed values are computed and locked
- Modifiers are computed and locked
- Maximum resources are computed
- Conditions are evaluated and locked

**Purpose**:
- Ensures consistent calculations throughout action resolution
- Prevents mid-action stat changes from affecting ongoing calculations
- Guarantees deterministic replay and proof generation

## Persistence Specification

### Stored State (SSOT)

✅ **Must Persist**:
- Core Stats: STR, DEX, CON, INT, WIL, EGO
- Level
- Current Resources: HP, MP, Lucidity
- Equipment state
- Active buffs/debuffs
- Timeline state

### Computed State (Cache)

❌ **Do Not Persist**:
- CoreEffective values
- Derived stats
- Speed values
- Modifiers
- Maximum resource values

**Rationale**: These are pure functions of the stored state and can always be recomputed deterministically.

## Condition System

Conditions represent reverse dependencies without violating layer hierarchy.

**Examples**:
- **Low HP → Speed Penalty**: Implemented as a condition that checks current HP ratio and applies a Speed multiplier
- **Overload → Cost Increase**: Condition that increases action costs based on equipment load
- **Lucidity → Roll Scaling**: Global condition that scales all roll outcomes

**Implementation**:
- Conditions are evaluated during snapshot
- Applied as final multipliers in the computation stack
- Never directly read lower-layer computed values
- May read stored state (e.g., current HP) or compare ratios

## Summary Table

| Aspect | Specification |
|--------|---------------|
| **Layer Direction** | Core → Derived → Speed → Modifier → Resource |
| **SSOT** | Core Stats + Level + ResourceCurrent |
| **Bonus Order** | Flat → %Inc(sum) → More/Less(multiply) → Clamp → Conditions |
| **Snapshot Timing** | Action initiation (locks Derived/Speed/Modifier/MaxResource) |
| **Persisted** | Core, Level, ResourceCurrent, Equipment, Buffs, Timeline |
| **Not Persisted** | Derived, Speed, Modifier cache |
| **Circular Prevention** | Upper layers only reference upper; reverse via conditions |
| **Lucidity Role** | Resource with global modifier capability (affects Speed/Modifier) |

## Design Rationale

### Why Layered Architecture?

1. **Determinism**: Clear dependency graph ensures reproducible calculations
2. **Provability**: Snapshot-based computation enables ZK proof generation
3. **Maintainability**: Each layer has a single, well-defined responsibility
4. **Performance**: Cached values can be reused within action resolution
5. **Modularity**: Layers can be tested and modified independently

### Why Unidirectional Flow?

1. **Prevents Circular Dependencies**: Eliminates infinite computation loops
2. **Simplifies Reasoning**: Dependencies are explicit and traceable
3. **Enables Incremental Computation**: Changes propagate predictably
4. **Facilitates Testing**: Each layer can be unit tested with mock inputs

### Why Conditions?

1. **Preserves Layer Hierarchy**: Reverse dependencies without formula coupling
2. **Flexibility**: Can model complex game mechanics (debuffs, status effects, environmental conditions)
3. **Composition**: Multiple conditions can stack independently
4. **Clarity**: Explicitly marks exceptional computation paths

## Implementation Guidelines

### Module Organization

The stat system should be organized into separate modules by responsibility:

- **Core Stats**: Base stat definitions and CoreEffective calculation
- **Derived Stats**: Combat stat formulas (Attack, Accuracy, Evasion, etc.)
- **Speed System**: Speed calculations and action cost formulas
- **Modifiers**: Roll modifier calculations for skill checks
- **Resources**: Resource pool management (HP, MP, Lucidity)
- **Bonuses**: Bonus application stack (Flat, %Inc, More/Less, Clamp)
- **Conditions**: Status effect system and trait definitions

### Design Requirements

1. **Pure Functions**: All calculation functions must be deterministic pure functions
   - Same inputs always produce same outputs
   - No side effects, I/O, or randomness
   - No references to global mutable state

2. **Snapshot Architecture**: Action resolution must use snapshots
   - All derived values computed and locked at action initiation
   - Snapshot includes: CoreEffective, Derived, Speed, Modifiers, Resources, Conditions
   - Prevents mid-action stat changes from affecting resolution

3. **Condition System**: Conditions must implement trait with methods for:
   - Speed modification
   - Cost modification
   - Roll modification
   - Must access only stored state (never lower-layer computed values)

## Future Considerations

### Planned Extensions

1. **Temporary Stats**: Short-duration stat modifications (potions, enchantments)
2. **Contextual Bonuses**: Context-dependent bonuses (vs specific enemy types, in certain terrain)
3. **Scaling Curves**: Non-linear stat scaling at high levels
4. **Derived Resources**: Secondary resource pools derived from primary stats

### Optimization Opportunities

1. **Partial Recomputation**: Cache CoreEffective and selectively update changed stats
2. **Lazy Evaluation**: Compute derived stats only when actually needed
3. **Snapshot Pooling**: Reuse snapshot allocations across actions
4. **SIMD Acceleration**: Vectorize bonus application for batch calculations

## Appendix: Formula Reference

### Core Stats → Derived Stats

```
Attack = STR × 1.5 + WeaponDamage + Flat + %Inc × More × Less
Accuracy = DEX + SkillRank × 2 + EquipmentBonus
Evasion = DEX × 0.5 + Armor.Evasion
AC = 10 + ArmorBase + DEX_modifier
PsiPower = INT × 0.8 + EGO × 0.5 + FocusItem
FocusEff = WIL × 1.2 + Concentration
```

### Core Stats → Speed

```
SpeedPhysical = 100 + DEX × 0.8 + STR × 0.2 - ArmorPenalty
SpeedCognitive = 100 + INT × 0.6 + WIL × 0.4
SpeedRitual = 100 + WIL × 0.5 + EGO × 0.5
```

### Core Stats → Modifiers

```
STR_mod = floor((STR - 10) / 2)
DEX_mod = floor((DEX - 10) / 2)
CON_mod = floor((CON - 10) / 2)
INT_mod = floor((INT - 10) / 2)
WIL_mod = floor((WIL - 10) / 2)
EGO_mod = floor((EGO - 10) / 2)
```

### Core Stats → Resources

```
HP_max = (CON × 10) + (Level × CON / 2)
MP_max = (WIL + INT) × 5 + (EGO × 2) + (Level × sqrt(WIL))
Lucidity_max = sqrt(Level) × ((STR+DEX+CON)/2 + (INT+WIL+EGO)×2)
```

### Speed → Cost

```
final_cost = base_cost × Conditions × 100 / clamp(SpeedKind, 50, 200)
```

where `SpeedKind` is Physical, Cognitive, or Ritual depending on action type.
