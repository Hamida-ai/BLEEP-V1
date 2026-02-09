// PHASE 5: UPGRADE ENGINE
// Orchestrates protocol upgrades with validation, rollback, and governance integration
//
// SAFETY INVARIANTS:
// 1. Upgrades are governed (governance approval required)
// 2. Upgrades are versioned (can verify and audit)
// 3. Upgrades are reversible (rollback available)
// 4. Upgrades are deterministic (all validators same result)
// 5. Chain is always canonical (never diverges)

use crate::protocol_version::{ProtocolSpec, ProtocolVersion, ProtocolVersionManager};
use crate::migration_rules::{MigrationEngine, MigrationPlan, MigrationRule, ParameterValue};
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpgradeError {
    #[error("Upgrade not approved by governance")]
    NotApproved,
    
    #[error("Upgrade validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Upgrade execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
    
    #[error("Incompatible version: {0}")]
    IncompatibleVersion(String),
}

/// Upgrade proposal (submitted by governance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeProposal {
    /// Unique proposal ID
    pub id: Vec<u8>,
    
    /// Target protocol version
    pub target_version: ProtocolVersion,
    
    /// Activation epoch
    pub activation_epoch: u64,
    
    /// Migration plan
    pub migration_plan: MigrationPlan,
    
    /// Approval status
    pub approved: bool,
    
    /// Governance vote count (for verification)
    pub vote_count: u64,
    
    /// Proposal hash (for commitment)
    pub proposal_hash: Vec<u8>,
}

impl UpgradeProposal {
    pub fn new(
        target_version: ProtocolVersion,
        activation_epoch: u64,
        migration_plan: MigrationPlan,
    ) -> Self {
        let mut proposal = UpgradeProposal {
            id: vec![],
            target_version,
            activation_epoch,
            migration_plan,
            approved: false,
            vote_count: 0,
            proposal_hash: vec![],
        };
        
        // Generate ID and hash
        proposal.id = proposal.compute_id();
        proposal.proposal_hash = proposal.compute_hash();
        
        proposal
    }
    
    pub fn compute_id(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.target_version.major.to_le_bytes());
        hasher.update(self.target_version.minor.to_le_bytes());
        hasher.update(self.target_version.patch.to_le_bytes());
        hasher.update(self.activation_epoch.to_le_bytes());
        hasher.finalize().to_vec()
    }
    
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.id);
        hasher.update([if self.approved { 1u8 } else { 0u8 }]);
        hasher.update(self.vote_count.to_le_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Approve proposal (called by governance)
    pub fn approve(&mut self, vote_count: u64) {
        self.approved = true;
        self.vote_count = vote_count;
        self.proposal_hash = self.compute_hash();
    }
}

/// Upgrade execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeExecution {
    /// Upgrade proposal ID
    pub proposal_id: Vec<u8>,
    
    /// From version
    pub from_version: ProtocolVersion,
    
    /// To version
    pub to_version: ProtocolVersion,
    
    /// Execution epoch
    pub execution_epoch: u64,
    
    /// Success status
    pub successful: bool,
    
    /// Pre-upgrade checkpoint hash
    pub pre_checkpoint_hash: Vec<u8>,
    
    /// Post-upgrade checkpoint hash
    pub post_checkpoint_hash: Vec<u8>,
}

/// Rollback record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRecord {
    /// Original upgrade proposal ID
    pub original_proposal_id: Vec<u8>,
    
    /// From version
    pub from_version: ProtocolVersion,
    
    /// To version (that was rolled back)
    pub to_version: ProtocolVersion,
    
    /// Rollback epoch
    pub rollback_epoch: u64,
    
    /// Reason for rollback
    pub reason: String,
}

/// Compatibility check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResult {
    /// Backward compatible
    pub backward_compatible: bool,
    
    /// Forward compatible
    pub forward_compatible: bool,
    
    /// Compatibility issues (if any)
    pub issues: Vec<String>,
}

impl CompatibilityResult {
    pub fn is_compatible(&self) -> bool {
        self.backward_compatible && self.forward_compatible && self.issues.is_empty()
    }
}

/// Upgrade engine
pub struct UpgradeEngine {
    /// Version manager
    version_manager: ProtocolVersionManager,
    
    /// Migration engine
    migration_engine: MigrationEngine,
    
    /// Pending upgrades
    pending_upgrades: Vec<UpgradeProposal>,
    
    /// Executed upgrades
    executed_upgrades: Vec<UpgradeExecution>,
    
    /// Rollback records
    rollback_records: Vec<RollbackRecord>,
    
    /// Current parameters
    current_parameters: BTreeMap<String, ParameterValue>,
}

impl UpgradeEngine {
    pub fn new(initial_version: ProtocolVersion, initial_epoch: u64) -> Self {
        UpgradeEngine {
            version_manager: ProtocolVersionManager::new(initial_version, initial_epoch),
            migration_engine: MigrationEngine::new(),
            pending_upgrades: Vec::new(),
            executed_upgrades: Vec::new(),
            rollback_records: Vec::new(),
            current_parameters: BTreeMap::new(),
        }
    }
    
    /// Submit upgrade proposal
    pub fn submit_proposal(
        &mut self,
        target_version: ProtocolVersion,
        activation_epoch: u64,
        migration_plan: MigrationPlan,
    ) -> Result<Vec<u8>, UpgradeError> {
        let proposal = UpgradeProposal::new(
            target_version,
            activation_epoch,
            migration_plan,
        );
        
        let id = proposal.id.clone();
        self.pending_upgrades.push(proposal);
        
        Ok(id)
    }
    
    /// Get pending upgrade
    pub fn get_pending_upgrade(&self, id: &[u8]) -> Option<&UpgradeProposal> {
        self.pending_upgrades.iter().find(|p| p.id == id)
    }
    
    /// Approve upgrade (called by governance)
    pub fn approve_upgrade(
        &mut self,
        id: &[u8],
        vote_count: u64,
    ) -> Result<(), UpgradeError> {
        let proposal = self.pending_upgrades.iter_mut()
            .find(|p| p.id == id)
            .ok_or(UpgradeError::ValidationFailed("Proposal not found".to_string()))?;
        
        proposal.approve(vote_count);
        
        Ok(())
    }
    
    /// Check backward compatibility
    pub fn check_backward_compatibility(
        &self,
        target_version: ProtocolVersion,
    ) -> CompatibilityResult {
        let current = self.version_manager.current_version();
        
        // Backward compatible if target is >= current
        let backward_compatible = target_version >= current;
        
        // Forward compatible if no breaking changes expected
        let forward_compatible = target_version.major == current.major;
        
        let mut issues = Vec::new();
        
        if target_version < current {
            issues.push("Cannot downgrade protocol version".to_string());
        }
        
        if target_version.major != current.major && target_version.major > current.major {
            issues.push("Major version change may require operator intervention".to_string());
        }
        
        CompatibilityResult {
            backward_compatible,
            forward_compatible,
            issues,
        }
    }
    
    /// Execute approved upgrade
    pub fn execute_upgrade(
        &mut self,
        id: &[u8],
        current_epoch: u64,
    ) -> Result<(), UpgradeError> {
        // Find proposal and extract needed data
        let (target_version, activation_epoch, migration_plan) = {
            let proposal = self.pending_upgrades.iter_mut()
                .find(|p| p.id == id)
                .ok_or(UpgradeError::ValidationFailed("Proposal not found".to_string()))?;
            
            if !proposal.approved {
                return Err(UpgradeError::NotApproved);
            }
            
            // Check activation epoch
            if current_epoch < proposal.activation_epoch {
                return Err(UpgradeError::ValidationFailed(
                    format!("Activation epoch {} not reached (current: {})", 
                        proposal.activation_epoch, current_epoch),
                ));
            }
            
            (proposal.target_version, proposal.activation_epoch, proposal.migration_plan.clone())
        };
        
        // Check backward compatibility
        let compat = self.check_backward_compatibility(target_version);
        if !compat.is_compatible() && !compat.backward_compatible {
            return Err(UpgradeError::IncompatibleVersion(
                format!("Compatibility issues: {:?}", compat.issues),
            ));
        }
        
        // Register migration plan
        self.migration_engine.register_migration(migration_plan);
        
        // Execute migration
        let new_params = self.migration_engine.execute_migration(
            self.version_manager.current_version(),
            target_version,
            &self.current_parameters,
            current_epoch,
        ).map_err(|e| UpgradeError::ExecutionFailed(e.to_string()))?;
        
        // Update version
        self.version_manager.schedule_upgrade(
            target_version,
            activation_epoch,
        ).map_err(|e| UpgradeError::ExecutionFailed(e.to_string()))?;
        
        self.version_manager.activate_upgrade(current_epoch)
            .map_err(|e| UpgradeError::ExecutionFailed(e.to_string()))?;
        
        // Update parameters
        self.current_parameters = new_params;
        
        // Record execution
        let execution = UpgradeExecution {
            proposal_id: id.to_vec(),
            from_version: self.version_manager.current_version(),
            to_version: target_version,
            execution_epoch: current_epoch,
            successful: true,
            pre_checkpoint_hash: vec![],
            post_checkpoint_hash: vec![],
        };
        
        self.executed_upgrades.push(execution);
        
        // Remove from pending
        self.pending_upgrades.retain(|p| p.id != id);
        
        Ok(())
    }
    
    /// Rollback upgrade (restore previous version)
    pub fn rollback_upgrade(
        &mut self,
        proposal_id: &[u8],
        reason: &str,
        current_epoch: u64,
    ) -> Result<(), UpgradeError> {
        // Find execution record
        let execution = self.executed_upgrades.iter()
            .find(|e| e.proposal_id == proposal_id)
            .cloned()
            .ok_or(UpgradeError::RollbackFailed("Execution not found".to_string()))?;
        
        // Create rollback record
        let rollback = RollbackRecord {
            original_proposal_id: proposal_id.to_vec(),
            from_version: execution.from_version,
            to_version: execution.to_version,
            rollback_epoch: current_epoch,
            reason: reason.to_string(),
        };
        
        // Revert to previous version
        self.version_manager.schedule_upgrade(
            execution.from_version,
            current_epoch,
        ).map_err(|e| UpgradeError::RollbackFailed(e.to_string()))?;
        
        self.version_manager.activate_upgrade(current_epoch)
            .map_err(|e| UpgradeError::RollbackFailed(e.to_string()))?;
        
        self.rollback_records.push(rollback);
        
        Ok(())
    }
    
    /// Get current version
    pub fn current_version(&self) -> ProtocolVersion {
        self.version_manager.current_version()
    }
    
    /// Get execution history
    pub fn get_execution_history(&self) -> &[UpgradeExecution] {
        &self.executed_upgrades
    }
    
    /// Get rollback history
    pub fn get_rollback_history(&self) -> &[RollbackRecord] {
        &self.rollback_records
    }
    
    /// Get pending upgrades
    pub fn get_pending_upgrades(&self) -> &[UpgradeProposal] {
        &self.pending_upgrades
    }
    
    /// Set parameter value
    pub fn set_parameter(&mut self, name: String, value: ParameterValue) {
        self.current_parameters.insert(name, value);
    }
    
    /// Get parameter value
    pub fn get_parameter(&self, name: &str) -> Option<&ParameterValue> {
        self.current_parameters.get(name)
    }
}

impl Default for UpgradeEngine {
    fn default() -> Self {
        Self::new(ProtocolVersion::new(1, 0, 0), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration_rules::MigrationTransform;

    #[test]
    fn test_upgrade_proposal() {
        let plan = MigrationPlan::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 1, 0),
        );
        
        let proposal = UpgradeProposal::new(
            ProtocolVersion::new(1, 1, 0),
            100,
            plan,
        );
        
        assert!(!proposal.id.is_empty());
        assert!(!proposal.approved);
    }

    #[test]
    fn test_upgrade_approval() {
        let plan = MigrationPlan::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 1, 0),
        );
        
        let mut proposal = UpgradeProposal::new(
            ProtocolVersion::new(1, 1, 0),
            100,
            plan,
        );
        
        proposal.approve(50);
        assert!(proposal.approved);
        assert_eq!(proposal.vote_count, 50);
    }

    #[test]
    fn test_upgrade_engine_submission() {
        let mut engine = UpgradeEngine::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        let plan = MigrationPlan::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 1, 0),
        );
        
        let id = engine.submit_proposal(
            ProtocolVersion::new(1, 1, 0),
            100,
            plan,
        ).unwrap();
        
        assert!(!id.is_empty());
        assert_eq!(engine.get_pending_upgrades().len(), 1);
    }

    #[test]
    fn test_backward_compatibility() {
        let engine = UpgradeEngine::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        let compat = engine.check_backward_compatibility(
            ProtocolVersion::new(1, 1, 0),
        );
        
        assert!(compat.backward_compatible);
        assert!(compat.forward_compatible);
    }

    #[test]
    fn test_downgrade_incompatible() {
        let engine = UpgradeEngine::new(
            ProtocolVersion::new(1, 1, 0),
            0,
        );
        
        let compat = engine.check_backward_compatibility(
            ProtocolVersion::new(1, 0, 0),
        );
        
        assert!(!compat.backward_compatible);
        assert!(!compat.issues.is_empty());
    }

    #[test]
    fn test_upgrade_execution() {
        let mut engine = UpgradeEngine::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        let mut plan = MigrationPlan::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 1, 0),
        );
        
        let rule = MigrationRule {
            param_type: crate::migration_rules::ParameterType::ConsensusBlockTime,
            from_version: ProtocolVersion::new(1, 0, 0),
            to_version: ProtocolVersion::new(1, 1, 0),
            transform: MigrationTransform::Identity,
            validate: None,
        };
        
        plan.add_rule(rule);
        
        let id = engine.submit_proposal(
            ProtocolVersion::new(1, 1, 0),
            100,
            plan,
        ).unwrap();
        
        engine.approve_upgrade(&id, 50).unwrap();
        
        let result = engine.execute_upgrade(&id, 100);
        assert!(result.is_ok());
        assert_eq!(engine.get_execution_history().len(), 1);
    }
}
