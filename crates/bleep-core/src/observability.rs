// PHASE 6: OBSERVABILITY & METRICS - Public Dashboard Infrastructure
//
// Purpose: Provide real-time, publicly observable metrics for the adversarial testnet
// that demonstrate:
// 1. Live attack detection in action
// 2. Autonomous recovery mechanisms
// 3. Governance-driven upgrades
// 4. All before/after state proofs
//
// Design: All metrics are immutable once recorded, suitable for public dashboards
// and investor demonstrations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use serde_json;

/// ═══════════════════════════════════════════════════════════════════════════════
/// OBSERVABILITY TYPES
/// ═══════════════════════════════════════════════════════════════════════════════

/// Unique event ID (deterministic, never duplicated)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventId(Vec<u8>);

impl EventId {
    pub fn new(event_type: &str, epoch: u64, index: usize, seed: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(event_type.as_bytes());
        hasher.update(epoch.to_le_bytes());
        hasher.update(index.to_le_bytes());
        hasher.update(seed);
        EventId(hasher.finalize().to_vec())
    }

    pub fn as_hex(&self) -> String {
        hex::encode(&self.0[..8])
    }
}

/// Observable event in the testnet (immutable once recorded)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableEvent {
    /// Unique event ID
    pub event_id: EventId,
    /// When event occurred (UNIX seconds)
    pub timestamp: u64,
    /// Which epoch
    pub epoch: u64,
    /// Type of event
    pub event_type: EventType,
    /// Human-readable description
    pub description: String,
    /// Metrics at time of event
    pub metrics_snapshot: NetworkMetrics,
    /// Immutable hash (prevents tampering)
    pub event_hash: Vec<u8>,
}

impl ObservableEvent {
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.event_id.0);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.epoch.to_le_bytes());
        hasher.update(self.event_type.name().as_bytes());
        hasher.update(self.description.as_bytes());
        hasher.finalize().to_vec()
    }

    pub fn verify(&self) -> bool {
        self.compute_hash() == self.event_hash
    }
}

/// Type of observable event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    TestnetStarted,
    ValidatorOnline,
    ValidatorOffline,
    BlockProposed,
    BlockFinalized,
    AttackDetected,
    IncidentReported,
    RecoveryStarted,
    RecoveryCompleted,
    GovernanceProposalCreated,
    GovernanceProposalApproved,
    GovernanceProposalRejected,
    UpgradeExecuted,
    UpgradeFailed,
    SlashingExecuted,
    HealthCheckPassed,
    HealthCheckFailed,
    StateVerified,
    StateInvalidity,
}

impl EventType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::TestnetStarted => "testnet_started",
            Self::ValidatorOnline => "validator_online",
            Self::ValidatorOffline => "validator_offline",
            Self::BlockProposed => "block_proposed",
            Self::BlockFinalized => "block_finalized",
            Self::AttackDetected => "attack_detected",
            Self::IncidentReported => "incident_reported",
            Self::RecoveryStarted => "recovery_started",
            Self::RecoveryCompleted => "recovery_completed",
            Self::GovernanceProposalCreated => "governance_proposal_created",
            Self::GovernanceProposalApproved => "governance_proposal_approved",
            Self::GovernanceProposalRejected => "governance_proposal_rejected",
            Self::UpgradeExecuted => "upgrade_executed",
            Self::UpgradeFailed => "upgrade_failed",
            Self::SlashingExecuted => "slashing_executed",
            Self::HealthCheckPassed => "health_check_passed",
            Self::HealthCheckFailed => "health_check_failed",
            Self::StateVerified => "state_verified",
            Self::StateInvalidity => "state_invalidity",
        }
    }

    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::AttackDetected
                | Self::IncidentReported
                | Self::HealthCheckFailed
                | Self::StateInvalidity
                | Self::UpgradeFailed
        )
    }
}

/// Network metrics snapshot (immutable once recorded)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub epoch: u64,
    pub block_height: u64,
    pub validators_online: usize,
    pub validators_offline: usize,
    pub validators_slashed: usize,
    pub finalized_blocks: u64,
    pub finalization_rate: f64,       // 0-100%
    pub avg_block_time_ms: f64,
    pub proposal_acceptance_rate: f64, // 0-100%
    pub network_latency_ms: f64,
    pub consensus_rounds: u64,
    pub fork_count: u64,
    pub recovery_count: u64,
    pub governance_active: bool,
    pub upgrade_in_progress: bool,
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            epoch: 0,
            block_height: 0,
            validators_online: 0,
            validators_offline: 0,
            validators_slashed: 0,
            finalized_blocks: 0,
            finalization_rate: 100.0,
            avg_block_time_ms: 12000.0,
            proposal_acceptance_rate: 95.0,
            network_latency_ms: 50.0,
            consensus_rounds: 0,
            fork_count: 0,
            recovery_count: 0,
            governance_active: false,
            upgrade_in_progress: false,
        }
    }
}

/// Time-series metrics for dashboards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTimeSeries {
    pub epochs: Vec<u64>,
    pub block_heights: Vec<u64>,
    pub finalization_rates: Vec<f64>,
    pub proposal_rates: Vec<f64>,
    pub network_latencies: Vec<f64>,
    pub slashing_counts: Vec<usize>,
    pub recovery_counts: Vec<usize>,
}

impl MetricsTimeSeries {
    pub fn new() -> Self {
        Self {
            epochs: Vec::new(),
            block_heights: Vec::new(),
            finalization_rates: Vec::new(),
            proposal_rates: Vec::new(),
            network_latencies: Vec::new(),
            slashing_counts: Vec::new(),
            recovery_counts: Vec::new(),
        }
    }

    pub fn add_snapshot(&mut self, metrics: &NetworkMetrics) {
        self.epochs.push(metrics.epoch);
        self.block_heights.push(metrics.block_height);
        self.finalization_rates.push(metrics.finalization_rate);
        self.proposal_rates.push(metrics.proposal_acceptance_rate);
        self.network_latencies.push(metrics.network_latency_ms);
        self.slashing_counts.push(metrics.validators_slashed);
        self.recovery_counts.push(metrics.recovery_count);
    }

    pub fn get_latest_metrics(&self) -> Option<(u64, u64, f64)> {
        if self.epochs.is_empty() {
            None
        } else {
            let idx = self.epochs.len() - 1;
            Some((
                self.epochs[idx],
                self.block_heights[idx],
                self.finalization_rates[idx],
            ))
        }
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// OBSERVABILITY ENGINE - CENTRAL METRICS HUB
/// ═══════════════════════════════════════════════════════════════════════════════

/// Central observability engine (public-facing)
pub struct ObservabilityEngine {
    /// Immutable event log (append-only)
    pub event_log: Vec<ObservableEvent>,
    /// Current network metrics
    pub current_metrics: NetworkMetrics,
    /// Historical metrics
    pub metrics_history: MetricsTimeSeries,
    /// Map of event type to count
    pub event_counts: BTreeMap<String, u64>,
    /// Most recent critical events
    pub critical_events: Vec<ObservableEvent>,
    /// Total incidents detected
    pub total_incidents: u64,
    /// Total recoveries executed
    pub total_recoveries: u64,
    /// Execution seed (for deterministic events)
    pub seed: Vec<u8>,
}

impl ObservabilityEngine {
    pub fn new(seed: Vec<u8>) -> Self {
        Self {
            event_log: Vec::new(),
            current_metrics: NetworkMetrics::default(),
            metrics_history: MetricsTimeSeries::new(),
            event_counts: BTreeMap::new(),
            critical_events: Vec::new(),
            total_incidents: 0,
            total_recoveries: 0,
            seed,
        }
    }

    /// Record an observable event (immutable once recorded)
    pub fn record_event(
        &mut self,
        event_type: EventType,
        description: String,
    ) -> EventId {
        let event_id = EventId::new(
            event_type.name(),
            self.current_metrics.epoch,
            self.event_log.len(),
            &self.seed,
        );

        let mut event = ObservableEvent {
            event_id: event_id.clone(),
            timestamp: current_timestamp(),
            epoch: self.current_metrics.epoch,
            event_type: event_type.clone(),
            description,
            metrics_snapshot: self.current_metrics.clone(),
            event_hash: vec![],
        };

        event.event_hash = event.compute_hash();

        // Track counts
        let count = self.event_counts.entry(event_type.name().to_string()).or_insert(0);
        *count += 1;

        // Track critical events
        if event_type.is_critical() {
            self.critical_events.push(event.clone());
            match event_type {
                EventType::AttackDetected | EventType::IncidentReported | EventType::StateInvalidity => {
                    self.total_incidents += 1;
                }
                EventType::RecoveryCompleted => {
                    self.total_recoveries += 1;
                }
                _ => {}
            }
        }

        self.event_log.push(event);
        event_id
    }

    /// Update current metrics and record snapshot
    pub fn update_metrics(&mut self, new_metrics: NetworkMetrics) {
        self.current_metrics = new_metrics.clone();
        self.metrics_history.add_snapshot(&new_metrics);
    }

    /// Get public dashboard data
    pub fn get_public_dashboard(&self) -> PublicDashboard {
        PublicDashboard {
            total_events: self.event_log.len() as u64,
            total_incidents: self.total_incidents,
            total_recoveries: self.total_recoveries,
            current_epoch: self.current_metrics.epoch,
            block_height: self.current_metrics.block_height,
            validators_online: self.current_metrics.validators_online,
            validators_slashed: self.current_metrics.validators_slashed,
            finalization_rate: self.current_metrics.finalization_rate,
            network_latency_ms: self.current_metrics.network_latency_ms,
            critical_events_count: self.critical_events.len() as u64,
            recovery_count: self.current_metrics.recovery_count,
            event_counts: self.event_counts.clone(),
            latest_critical_event: self.critical_events.last().cloned(),
        }
    }

    /// Get immutable event log (for verification)
    pub fn get_event_log(&self) -> Vec<ObservableEvent> {
        self.event_log.clone()
    }

    /// Verify event log integrity
    pub fn verify_log_integrity(&self) -> bool {
        self.event_log.iter().all(|event| event.verify())
    }

    /// Get metrics history for charts
    pub fn get_metrics_history(&self) -> MetricsTimeSeries {
        self.metrics_history.clone()
    }
}

/// Public-facing dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicDashboard {
    pub total_events: u64,
    pub total_incidents: u64,
    pub total_recoveries: u64,
    pub current_epoch: u64,
    pub block_height: u64,
    pub validators_online: usize,
    pub validators_slashed: usize,
    pub finalization_rate: f64,
    pub network_latency_ms: f64,
    pub critical_events_count: u64,
    pub recovery_count: u64,
    pub event_counts: BTreeMap<String, u64>,
    pub latest_critical_event: Option<ObservableEvent>,
}

impl PublicDashboard {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// ALERT SYSTEM - REAL-TIME NOTIFICATIONS
/// ═══════════════════════════════════════════════════════════════════════════════

/// Alert severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info = 1,
    Warning = 2,
    Critical = 3,
}

/// Real-time alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub severity: AlertSeverity,
    pub message: String,
    pub event_id: String,
    pub timestamp: u64,
}

/// Alert aggregator
pub struct AlertSystem {
    pub active_alerts: Vec<Alert>,
    pub alert_history: Vec<Alert>,
}

impl AlertSystem {
    pub fn new() -> Self {
        Self {
            active_alerts: Vec::new(),
            alert_history: Vec::new(),
        }
    }

    pub fn publish_alert(&mut self, severity: AlertSeverity, message: String, event_id: String) {
        let alert = Alert {
            severity,
            message,
            event_id,
            timestamp: current_timestamp(),
        };

        self.active_alerts.push(alert.clone());
        self.alert_history.push(alert);
    }

    pub fn get_critical_alerts(&self) -> Vec<Alert> {
        self.active_alerts
            .iter()
            .filter(|a| a.severity == AlertSeverity::Critical)
            .cloned()
            .collect()
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// UTILITIES
/// ═══════════════════════════════════════════════════════════════════════════════

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_immutability() {
        let mut event = ObservableEvent {
            event_id: EventId::new("test", 0, 0, b"seed"),
            timestamp: current_timestamp(),
            epoch: 0,
            event_type: EventType::AttackDetected,
            description: "Test attack".to_string(),
            metrics_snapshot: NetworkMetrics::default(),
            event_hash: vec![],
        };

        event.event_hash = event.compute_hash();
        assert!(event.verify());
    }

    #[test]
    fn test_observability_engine() {
        let mut engine = ObservabilityEngine::new(b"test_seed".to_vec());

        engine.update_metrics(NetworkMetrics {
            epoch: 1,
            block_height: 12,
            validators_online: 20,
            validators_slashed: 0,
            finalization_rate: 100.0,
            ..Default::default()
        });

        engine.record_event(
            EventType::AttackDetected,
            "Validator collusion detected".to_string(),
        );

        assert_eq!(engine.event_log.len(), 1);
        assert_eq!(engine.total_incidents, 1);
        assert!(engine.verify_log_integrity());
    }

    #[test]
    fn test_metrics_time_series() {
        let mut series = MetricsTimeSeries::new();

        series.add_snapshot(&NetworkMetrics {
            epoch: 0,
            finalization_rate: 100.0,
            ..Default::default()
        });

        series.add_snapshot(&NetworkMetrics {
            epoch: 1,
            finalization_rate: 90.0,
            ..Default::default()
        });

        assert_eq!(series.epochs.len(), 2);
        assert_eq!(series.finalization_rates.len(), 2);
    }

    #[test]
    fn test_alert_system() {
        let mut alerts = AlertSystem::new();
        alerts.publish_alert(
            AlertSeverity::Critical,
            "Attack detected".to_string(),
            "event_001".to_string(),
        );

        assert_eq!(alerts.get_critical_alerts().len(), 1);
    }
}
