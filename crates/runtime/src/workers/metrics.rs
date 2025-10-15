//! Proof generation metrics and statistics.
//!
//! Tracks proof generation performance, success rates, and queue status.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Proof generation metrics tracked by ProverWorker.
///
/// Provides real-time statistics about proof generation performance
/// and queue status for monitoring and debugging.
///
/// Uses atomics for lock-free access across threads.
#[derive(Debug, Default)]
pub struct ProofMetrics {
    /// Total number of proofs successfully generated
    generated: AtomicU64,

    /// Total number of proof generations that failed
    failed: AtomicU64,

    /// Number of actions currently queued for proving
    queue_depth: AtomicU64,

    /// Total time spent generating proofs (sum of all durations, in nanoseconds)
    total_proving_time_nanos: AtomicU64,

    /// Peak queue depth observed
    peak_queue_depth: AtomicU64,
}

impl ProofMetrics {
    /// Creates a new empty metrics tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a successful proof generation.
    pub fn record_success(&self, proving_time: Duration) {
        self.generated.fetch_add(1, Ordering::Relaxed);
        self.total_proving_time_nanos
            .fetch_add(proving_time.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Records a failed proof generation.
    pub fn record_failure(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Updates queue depth and tracks peak.
    pub fn set_queue_depth(&self, depth: u64) {
        self.queue_depth.store(depth, Ordering::Relaxed);

        // Update peak using compare-and-swap loop
        let mut current_peak = self.peak_queue_depth.load(Ordering::Relaxed);
        while depth > current_peak {
            match self.peak_queue_depth.compare_exchange_weak(
                current_peak,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = actual,
            }
        }
    }

    /// Returns snapshot of current generated count.
    pub fn generated(&self) -> u64 {
        self.generated.load(Ordering::Relaxed)
    }

    /// Returns snapshot of current failed count.
    pub fn failed(&self) -> u64 {
        self.failed.load(Ordering::Relaxed)
    }

    /// Returns current queue depth.
    pub fn queue_depth(&self) -> u64 {
        self.queue_depth.load(Ordering::Relaxed)
    }

    /// Returns peak queue depth observed.
    pub fn peak_queue_depth(&self) -> u64 {
        self.peak_queue_depth.load(Ordering::Relaxed)
    }

    /// Calculates average proof generation time.
    pub fn avg_proving_time(&self) -> Duration {
        let generated = self.generated.load(Ordering::Relaxed);
        if generated == 0 {
            Duration::ZERO
        } else {
            let total_nanos = self.total_proving_time_nanos.load(Ordering::Relaxed);
            Duration::from_nanos(total_nanos / generated)
        }
    }

    /// Returns success rate as a percentage (0-100).
    pub fn success_rate(&self) -> f64 {
        let generated = self.generated.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let total = generated + failed;

        if total == 0 {
            100.0
        } else {
            (generated as f64 / total as f64) * 100.0
        }
    }

    /// Returns total number of proof requests (success + failure).
    pub fn total_requests(&self) -> u64 {
        self.generated.load(Ordering::Relaxed) + self.failed.load(Ordering::Relaxed)
    }

    /// Creates a snapshot of all metrics for display/logging.
    ///
    /// Note: This is not atomic across all fields - individual fields
    /// are read atomically but the snapshot as a whole may be inconsistent
    /// if metrics are being updated concurrently.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            generated: self.generated(),
            failed: self.failed(),
            queue_depth: self.queue_depth(),
            peak_queue_depth: self.peak_queue_depth(),
            avg_proving_time: self.avg_proving_time(),
            success_rate: self.success_rate(),
        }
    }
}

/// Snapshot of metrics at a point in time.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub generated: u64,
    pub failed: u64,
    pub queue_depth: u64,
    pub peak_queue_depth: u64,
    pub avg_proving_time: Duration,
    pub success_rate: f64,
}
