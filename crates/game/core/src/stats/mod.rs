//! Stat System - Layered architecture v2.0
//!
//! This module implements the complete stat system following the layered architecture
//! specified in docs/stat-system.md.
//!
//! # Architecture
//!
//! ```text
//! [ Core Stats (Layer 1) ]
//!      ↓
//! [ Derived Stats (Layer 2) ]
//!      ↓
//! [ Speed / Cost (Layer 3) ]
//!      ↓
//! [ Modifiers (Layer 4) ]
//!      ↓
//! [ Resources (Layer 5) ]
//! ```
//!
//! ## Principles
//!
//! 1. **SSOT**: Core stats, level, and current resources only
//! 2. **Unidirectional Flow**: Upper layers never depend on lower layers
//! 3. **Snapshot Consistency**: All values locked at action initiation
//! 4. **Deterministic**: Pure functions, no I/O or randomness
//! 5. **Storage Boundaries**: Explicit persistent vs computed state
//!
//! ## Unified Bonus Calculation
//!
//! All layers use the same calculation pattern via `BonusStack::apply()`:
//!
//! ```text
//! Final = clamp((base + Flat) × (1 + %Inc/100) × More × Less, min, max)
//! ```
//!
//! ### Application Order
//! 1. **Flat**: Additive bonuses (summed)
//! 2. **%Inc**: Percentage increases (summed, then multiplied)
//! 3. **More**: Sequential multipliers (e.g., ×1.5)
//! 4. **Less**: Sequential reductions (e.g., ×0.9)
//! 5. **Clamp**: Bounded to [min, max] range
//!
//! ### Per-Layer Bounds
//! - **Layer 1 (Core)**: [1, 99] - Stat balance
//! - **Layer 2 (Derived)**: [0, 9999] - Combat values
//! - **Layer 3 (Speed)**: [50, 200] - Timeline balance
//! - **Layer 4 (Modifiers)**: [-20, 50] - Roll balance
//! - **Layer 5 (Resources)**: Formula-based (no bonus system)
//!
//! ## Trait-Based Design
//!
//! All stat layers implement the `StatLayer` trait, which defines the
//! `Base -> Bonuses -> Final` computation pattern:
//!
//! ```rust,ignore
//! pub trait StatLayer {
//!     type Base;      // Input type
//!     type Bonuses;   // Bonus holder type
//!     type Final;     // Output type
//!
//!     fn compute(base: &Self::Base, bonuses: &Self::Bonuses) -> Self::Final;
//!     fn empty_bonuses() -> Self::Bonuses;
//!     fn from_base(base: &Self::Base) -> Self::Final;
//! }
//! ```
//!
//! This enables:
//! - **Consistent API**: All layers use the same pattern
//! - **Generic programming**: Write functions that work with any layer
//! - **Type safety**: Compiler enforces correct types at each layer
//! - **Clear dependencies**: Base/Bonuses/Final relationships are explicit
//!
//! ## ActorBonuses Aggregation
//!
//! The `ActorBonuses` type aggregates bonuses from all 5 layers for efficient
//! ZK proof generation. Instead of recomputing bonuses from inventory on every
//! action, bonuses are cached and only recomputed when inventory/effects change.
//! This amortizes the cost of bonus calculation across equipment changes rather
//! than every action execution.

pub mod bonus;
pub mod core;
pub mod derived;
pub mod modifiers;
pub mod resources;
pub mod snapshot;
pub mod speed;

// Re-export primary types
pub use bonus::{ActorBonuses, Bonus, BonusStack, StatBounds, StatLayer, compute_actor_bonuses};
pub use core::{CoreEffective, CoreStatBonuses, CoreStats};
pub use derived::{DerivedBonuses, DerivedStats};
pub use modifiers::{ModifierBonuses, StatModifiers};
pub use resources::{ResourceBonuses, ResourceCurrent, ResourceMaximums};
pub use snapshot::{StatsSnapshot, StatsSnapshotBuilder};
pub use speed::{SpeedBonuses, SpeedKind, SpeedStats, calculate_action_cost};
