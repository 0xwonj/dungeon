//! Combat resolution system.
//!
//! This module provides pure functions for resolving combat interactions.
//! All combat logic is deterministic and side-effect free.
//!
//! # Architecture
//!
//! - **Pure Functions**: All functions are side-effect free
//! - **Used by Actions**: AttackAction and other combat actions call these functions
//! - **Stats-based**: Uses StatsSnapshot for all calculations
//!
//! # Core Functions
//!
//! - `resolve_attack`: Complete attack resolution (hit check + damage)
//! - `calculate_hit_chance`: Accuracy vs Evasion calculation
//! - `calculate_damage`: Damage calculation with attack/defense
//! - `apply_damage`: HP reduction (clamped to 0)

pub mod damage;
pub mod hit;
pub mod result;

pub use damage::{DamageType, apply_damage, calculate_damage};
pub use hit::{calculate_hit_chance, check_hit};
pub use result::{AttackOutcome, AttackResult, resolve_attack};
