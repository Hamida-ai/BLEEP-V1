//! # LoadBalancer
//!
//! Tracks per-shard validator load and emits rebalance signals.
//! An earlier version imported non-existent `crate::modules::energy` — removed.

use crate::metrics::MetricGauge;
use std::collections::HashMap;

/// Tracks CPU / message-queue load per shard and recommends rebalancing.
pub struct LoadBalancer {
    /// Minimum acceptable load score (0.0–100.0) before rebalancing triggers.
    load_threshold: f64,
    /// Per-shard load gauges.
    shard_load: HashMap<u64, MetricGauge>,
}

impl LoadBalancer {
    /// Create a balancer with the given threshold.
    pub fn new(load_threshold: f64) -> Self {
        Self {
            load_threshold,
            shard_load: HashMap::new(),
        }
    }

    /// Record the current load score for a shard (0–100).
    pub fn record_shard_load(&mut self, shard_id: u64, score: i64) {
        let gauge = self
            .shard_load
            .entry(shard_id)
            .or_insert_with(|| MetricGauge::new(&format!("bleep_shard_{}_load", shard_id)));
        gauge.set(score);
    }

    /// Returns `true` and logs a warning if any shard is below threshold.
    pub fn check_balance(&self) -> bool {
        let mut needs_rebalance = false;
        for (shard_id, gauge) in &self.shard_load {
            let score = gauge.get() as f64;
            if score < self.load_threshold {
                log::warn!(
                    "LoadBalancer: shard {} load score {:.1} below threshold {:.1} — rebalance advised",
                    shard_id, score, self.load_threshold
                );
                needs_rebalance = true;
            }
        }
        needs_rebalance
    }

    /// Return the average load across all tracked shards, or 100.0 if none.
    pub fn average_load(&self) -> f64 {
        if self.shard_load.is_empty() {
            return 100.0;
        }
        let total: i64 = self.shard_load.values().map(|g| g.get()).sum();
        total as f64 / self.shard_load.len() as f64
    }
}
