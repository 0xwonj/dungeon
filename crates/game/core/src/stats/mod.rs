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
//! ## Bonus Stack
//!
//! All layers use the same calculation order:
//! `Flat → %Inc → More → Less → Clamp → Conditions`

pub mod actor;
pub mod bonus;
pub mod conditions;
pub mod core;
pub mod derived;
pub mod modifiers;
pub mod resources;
pub mod snapshot;
pub mod speed;

// Re-export primary types
pub use actor::ActorStats;
pub use bonus::{Bonus, BonusStack};
pub use conditions::{Condition, ConditionSet, common as condition_effects};
pub use core::{CoreEffective, CoreStatBonuses, CoreStats};
pub use derived::{DerivedBonuses, DerivedStats};
pub use modifiers::{FinalModifiers, ModifierBonuses, StatModifiers};
pub use resources::{ResourceCurrent, ResourceMaximums, ResourceMeter, ResourceMeters};
pub use snapshot::{ActionSnapshot, SnapshotBuilder};
pub use speed::{SpeedConditions, SpeedKind, SpeedStats, calculate_action_cost};
