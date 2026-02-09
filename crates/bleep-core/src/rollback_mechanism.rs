// PHASE 5: ROLLBACK MECHANISM
// Automatic detection and recovery from failed upgrades
//
// SAFETY INVARIANTS:
// 1. Failures detected deterministically (same on all validators)
// 2. Rollback is automatic (no manual intervention required)
// 3. Chain remains canonical during rollback
// 4. Previous state is recoverable (not destructively deleted)
// 5. Rollback is verifiable (all validators same result)

use crate::protocol_version::ProtocolVersion;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RollbackError {
    #[error("Invalid health check: {0}")]
    HealthCheckFailed(String),
    
    #[error("No snapshot available for rollback")]
    NoSnapshotAvailable,
    
    #[error("Snapshot corruption detected")]
    SnapshotCorrupted,
    
    #[error("Rollback already in progress")]
    AlreadyRollingBack,
}

/// Health check metric
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HealthMetric {
    /// % of blocks finalized successfully
    BlockFinalizationRate,
    
    /// % of validators producing blocks
    ValidatorProduction,
    
    /// % of proposals accepted
    ProposalAcceptance,
    
    /// Network latency (ms)
    NetworkLatency,
    
    /// Consensus rounds per epoch
    ConsensusRounds,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Metric being checked
    pub metric: HealthMetric,
    
    /// Current value (0-100)
    pub value: f64,
    
    /// Threshold for failure (if value drops below)
    pub threshold: f64,
    
    /// Healthy (value >= threshold)
    pub healthy: bool,
    
    /// Epoch at check
    pub epoch: u64,
}

impl HealthCheck {
    pub fn new(metric: HealthMetric, value: f64, threshold: f64, epoch: u64) -> Self {
        let healthy = value >= threshold;
        HealthCheck {
            metric,
            value,
            threshold,
            healthy,
            epoch,
        }
    }
    
    /// Check if healthy
    pub fn is_healthy(&self) -> bool {
        self.healthy
    }
}

/// Version snapshot (for rollback)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSnapshot {
    /// Protocol version
    pub version: ProtocolVersion,
    
    /// Epoch when snapshot taken
    pub snapshot_epoch: u64,
    
    /// Health metrics at snapshot
    pub health_metrics: Vec<HealthCheck>,
    
    /// State hash (for verification)
    pub state_hash: Vec<u8>,
    
    /// Snapshot hash (commitment)
    pub snapshot_hash: Vec<u8>,
}

impl VersionSnapshot {
    pub fn new(
        version: ProtocolVersion,
        snapshot_epoch: u64,
        health_metrics: Vec<HealthCheck>,
        state_hash: Vec<u8>,
    ) -> Self {
        let mut snapshot = VersionSnapshot {
            version,
            snapshot_epoch,
            health_metrics,
            state_hash,
            snapshot_hash: vec![],
        };
        
        snapshot.snapshot_hash = snapshot.compute_hash();
        snapshot
    }
    
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.version.major.to_le_bytes());
        hasher.update(self.version.minor.to_le_bytes());
        hasher.update(self.version.patch.to_le_bytes());
        hasher.update(self.snapshot_epoch.to_le_bytes());
        
        for metric in &self.health_metrics {
            hasher.update(metric.value.to_le_bytes());
            hasher.update(metric.threshold.to_le_bytes());
            hasher.update([if metric.healthy { 1u8 } else { 0u8 }]);
        }
        
        hasher.update(&self.state_hash);
        hasher.finalize().to_vec()
    }
}

/// Rollback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackState {
    Idle,
    MonitoringHealth,
    FailureDetected,
    RollingBack,
    RollbackComplete,
}

/// Failure reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureReason {
    /// What failed
    pub description: String,
    
    /// Health checks that failed
    pub failed_metrics: Vec<HealthMetric>,
    
    /// Epoch failure detected
    pub detection_epoch: u64,
}

/// Rollback history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackHistoryEntry {
    /// Version rolled back from
    pub from_version: ProtocolVersion,
    
    /// Version rolled back to
    pub to_version: ProtocolVersion,
    
    /// Rollback epoch
    pub rollback_epoch: u64,
    
    /// Reason for rollback
    pub reason: FailureReason,
}

/// Rollback mechanism
pub struct RollbackMechanism {
    /// Current rollback state
    state: RollbackState,
    
    /// Snapshots (circular buffer of last 10)
    snapshots: VecDeque<VersionSnapshot>,
    max_snapshots: usize,
    
    /// Rollback history
    history: Vec<RollbackHistoryEntry>,
    
    /// Current version
    current_version: ProtocolVersion,
    
    /// Current epoch
    current_epoch: u64,
}

impl RollbackMechanism {
    pub fn new(initial_version: ProtocolVersion, initial_epoch: u64) -> Self {
        RollbackMechanism {
            state: RollbackState::Idle,
            snapshots: VecDeque::new(),
            max_snapshots: 10,
            history: Vec::new(),
            current_version: initial_version,
            current_epoch: initial_epoch,
        }
    }
    
    /// Take snapshot of current version
    pub fn take_snapshot(
        &mut self,
        health_metrics: Vec<HealthCheck>,
        state_hash: Vec<u8>,
    ) -> Result<(), RollbackError> {
        if self.state != RollbackState::Idle {
            return Err(RollbackError::AlreadyRollingBack);
        }
        
        let snapshot = VersionSnapshot::new(
            self.current_version,
            self.current_epoch,
            health_metrics,
            state_hash,
        );
        
        // Remove oldest snapshot if at capacity
        if self.snapshots.len() >= self.max_snapshots {
            self.snapshots.pop_front();
        }
        
        self.snapshots.push_back(snapshot);
        
        Ok(())
    }
    
    /// Start health monitoring
    pub fn start_monitoring(&mut self) {
        self.state = RollbackState::MonitoringHealth;
    }
    
    /// Detect failure from health checks
    pub fn detect_failure(
        &mut self,
        health_checks: &[HealthCheck],
        current_epoch: u64,
    ) -> Result<(), RollbackError> {
        // Find all failed checks
        let failed_metrics: Vec<HealthMetric> = health_checks
            .iter()
            .filter(|h| !h.is_healthy())
            .map(|h| h.metric)
            .collect();
        
        if !failed_metrics.is_empty() {
            self.state = RollbackState::FailureDetected;
            return Ok(());
        }
        
        Ok(())
    }
    
    /// Execute automatic rollback
    pub fn execute_rollback(
        &mut self,
        reason: &str,
        failed_metrics: Vec<HealthMetric>,
        _current_epoch: u64,
    ) -> Result<ProtocolVersion, RollbackError> {
        if self.state != RollbackState::FailureDetected {
            return Err(RollbackError::HealthCheckFailed(
                "Failure not detected".to_string()
            ));
        }
        
        // Get previous snapshot (one before current)
        if self.snapshots.len() < 2 {
            return Err(RollbackError::NoSnapshotAvailable);
        }
        
        // Get previous version
        let previous_snapshot = &self.snapshots[self.snapshots.len() - 2];
        let previous_version = previous_snapshot.version;
        
        // Verify snapshot integrity
        let computed_hash = previous_snapshot.compute_hash();
        if computed_hash != previous_snapshot.snapshot_hash {
            return Err(RollbackError::SnapshotCorrupted);
        }
        
        // Record rollback
        let entry = RollbackHistoryEntry {
            from_version: self.current_version,
            to_version: previous_version,
            rollback_epoch: current_epoch,
            reason: FailureReason {
                description: reason.to_string(),
                failed_metrics,
                detection_epoch: current_epoch,
            },
        };
        
        self.history.push(entry);
        
        // Revert to previous version
        self.current_version = previous_version;
        self.current_epoch = current_epoch;
        self.state = RollbackState::RollbackComplete;
        
        // Clear snapshots of failed version
        self.snapshots.pop_back();
        
        Ok(previous_version)
    }
    
    /// Resume normal operation
    pub fn resume_normal_operation(&mut self) {
        self.state = RollbackState::Idle;
    }
    
    /// Get current state
    pub fn get_state(&self) -> RollbackState {
        self.state
    }
    
    /// Get last snapshot
    pub fn get_last_snapshot(&self) -> Option<&VersionSnapshot> {
        self.snapshots.back()
    }
    
    /// Get rollback history
    pub fn get_history(&self) -> &[RollbackHistoryEntry] {
        &self.history
    }
    
    /// Get snapshot count
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

impl Default for RollbackMechanism {
    fn default() -> Self {
        Self::new(ProtocolVersion::new(1, 0, 0), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        let check = HealthCheck::new(
            HealthMetric::BlockFinalizationRate,
            95.0,
            80.0,
            100,
        );
        
        assert!(check.is_healthy());
        assert_eq!(check.value, 95.0);
        assert_eq!(check.threshold, 80.0);
    }

    #[test]
    fn test_health_check_failure() {
        let check = HealthCheck::new(
            HealthMetric::BlockFinalizationRate,
            50.0,
            80.0,
            100,
        );
        
        assert!(!check.is_healthy());
    }

    #[test]
    fn test_version_snapshot() {
        let check = HealthCheck::new(
            HealthMetric::BlockFinalizationRate,
            95.0,
            80.0,
            100,
        );
        
        let snapshot = VersionSnapshot::new(
            ProtocolVersion::new(1, 1, 0),
            100,
            vec![check],
            vec![1, 2, 3],
        );
        
        assert!(!snapshot.snapshot_hash.is_empty());
        assert_eq!(snapshot.version, ProtocolVersion::new(1, 1, 0));
    }

    #[test]
    fn test_snapshot_hash_determinism() {
        let check = HealthCheck::new(
            HealthMetric::BlockFinalizationRate,
            95.0,
            80.0,
            100,
        );
        
        let snapshot1 = VersionSnapshot::new(
            ProtocolVersion::new(1, 1, 0),
            100,
            vec![check.clone()],
            vec![1, 2, 3],
        );
        
        let snapshot2 = VersionSnapshot::new(
            ProtocolVersion::new(1, 1, 0),
            100,
            vec![check],
            vec![1, 2, 3],
        );
        
        assert_eq!(snapshot1.snapshot_hash, snapshot2.snapshot_hash);
    }

    #[test]
    fn test_rollback_mechanism() {
        let mut mechanism = RollbackMechanism::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        // Take snapshot at v1.0.0
        let check1 = HealthCheck::new(
            HealthMetric::BlockFinalizationRate,
            95.0,
            80.0,
            0,
        );
        mechanism.take_snapshot(vec![check1], vec![1]).unwrap();
        
        // Upgrade to v1.1.0
        mechanism.current_version = ProtocolVersion::new(1, 1, 0);
        mechanism.current_epoch = 10;
        
        // Take snapshot at v1.1.0
        let check2 = HealthCheck::new(
            HealthMetric::BlockFinalizationRate,
            50.0,  // Failed!
            80.0,
            10,
        );
        mechanism.take_snapshot(vec![check2.clone()], vec![2]).unwrap();
        
        // Detect failure
        mechanism.start_monitoring();
        mechanism.detect_failure(&[check2], 10).unwrap();
        
        // Rollback
        let result = mechanism.execute_rollback(
            "Health check failed",
            vec![HealthMetric::BlockFinalizationRate],
            10,
        );
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProtocolVersion::new(1, 0, 0));
        assert_eq!(mechanism.get_history().len(), 1);
    }

    #[test]
    fn test_snapshot_circular_buffer() {
        let mut mechanism = RollbackMechanism::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        // Take more snapshots than max
        for i in 0..15 {
            let check = HealthCheck::new(
                HealthMetric::BlockFinalizationRate,
                95.0,
                80.0,
                i,
            );
            mechanism.take_snapshot(vec![check], vec![i as u8]).unwrap();
        }
        
        // Should only have max_snapshots
        assert_eq!(mechanism.snapshot_count(), mechanism.max_snapshots);
    }

    #[test]
    fn test_no_rollback_without_snapshot() {
        let mut mechanism = RollbackMechanism::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        mechanism.state = RollbackState::FailureDetected;
        
        let result = mechanism.execute_rollback(
            "Test failure",
            vec![],
            10,
        );
        
        assert!(result.is_err());
    }
}
