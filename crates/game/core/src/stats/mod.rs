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
//! ## Trait-Based Design
//!
//! All stat layers implement the `StatLayer` trait, which defines the
//! `Base -> Bonuses -> Final` computation pattern:

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
