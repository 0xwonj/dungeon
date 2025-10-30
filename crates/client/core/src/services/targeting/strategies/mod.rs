//! Built-in targeting strategy implementations.

pub mod fastest;
pub mod lowest_health;
pub mod nearest;
pub mod next_to_act;
pub mod threat_based;

pub use fastest::FastestStrategy;
pub use lowest_health::LowestHealthStrategy;
pub use nearest::NearestStrategy;
pub use next_to_act::NextToActStrategy;
pub use threat_based::ThreatBasedStrategy;
