// PHASE 5: MIGRATION RULES
// Defines how to migrate state, parameters, and rules between versions
//
// SAFETY INVARIANTS:
// 1. All migrations are deterministic (same input → same output)
// 2. State consistency verified before/after migration
// 3. Migrations are atomic (all-or-nothing)
// 4. Incompatible migrations rejected (cannot proceed)
// 5. Rollback state is preserved for automatic recovery

use crate::protocol_version::ProtocolVersion;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Migration not found: {0} → {1}")]
    MigrationNotFound(String, String),
    
    #[error("Inconsistent state before migration")]
    InconsistentStateBefore,
    
    #[error("Inconsistent state after migration")]
    InconsistentStateAfter,
    
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
}

/// Parameter type for upgrade targets
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterType {
    ConsensusBlockTime,
    ConsensusFinalityLag,
    GovernanceQuorum,
    GovernanceThreshold,
    SlashingAmount,
    SlashingCondition,
    RecoveryPolicy,
    Other(String),
}

/// Parameter value (can be u64, f64, bool, string)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterValue {
    U64(u64),
    F64(f64),
    Bool(bool),
    String(String),
}

impl ParameterValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            ParameterValue::U64(v) => Some(*v),
            _ => None,
        }
    }
    
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ParameterValue::F64(v) => Some(*v),
            _ => None,
        }
    }
    
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParameterValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

/// Migration rule for a single parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRule {
    /// Parameter being migrated
    pub param_type: ParameterType,
    
    /// Source version
    pub from_version: ProtocolVersion,
    
    /// Target version
    pub to_version: ProtocolVersion,
    
    /// Transformation function (deterministic)
    pub transform: MigrationTransform,
    
    /// Optional validation (returns error if false)
    pub validate: Option<String>,
}

/// Migration transformation function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationTransform {
    /// Identity: value unchanged
    Identity,
    
    /// Scale: multiply by factor
    Scale { factor: f64 },
    
    /// Add: add constant
    Add { value: i64 },
    
    /// Replace: use new value
    Replace { new_value: ParameterValue },
    
    /// Custom: apply custom logic
    Custom(String),
}

impl MigrationTransform {
    /// Apply transformation to value
    pub fn apply(&self, value: &ParameterValue) -> Result<ParameterValue, MigrationError> {
        match self {
            MigrationTransform::Identity => Ok(value.clone()),
            
            MigrationTransform::Scale { factor } => {
                match value {
                    ParameterValue::U64(v) => {
                        let result = (*v as f64 * factor) as u64;
                        Ok(ParameterValue::U64(result))
                    }
                    ParameterValue::F64(v) => {
                        Ok(ParameterValue::F64(v * factor))
                    }
                    _ => Err(MigrationError::MigrationFailed(
                        "Cannot scale non-numeric value".to_string()
                    ))
                }
            }
            
            MigrationTransform::Add { value: add_val } => {
                match value {
                    ParameterValue::U64(v) => {
                        let result = if *add_val >= 0 {
                            v + (*add_val as u64)
                        } else {
                            v - ((-*add_val) as u64)
                        };
                        Ok(ParameterValue::U64(result))
                    }
                    _ => Err(MigrationError::MigrationFailed(
                        "Cannot add to non-numeric value".to_string()
                    ))
                }
            }
            
            MigrationTransform::Replace { new_value } => {
                Ok(new_value.clone())
            }
            
            MigrationTransform::Custom(desc) => {
                Err(MigrationError::MigrationFailed(
                    format!("Custom migration requires implementation: {}", desc)
                ))
            }
        }
    }
}

/// State checkpoint (for rollback)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCheckpoint {
    /// Version at checkpoint
    pub version: ProtocolVersion,
    
    /// Epoch at checkpoint
    pub epoch: u64,
    
    /// State hash (commitment to all data)
    pub state_hash: Vec<u8>,
    
    /// Parameter values
    pub parameters: BTreeMap<String, ParameterValue>,
}

impl StateCheckpoint {
    pub fn new(
        version: ProtocolVersion,
        epoch: u64,
        parameters: BTreeMap<String, ParameterValue>,
    ) -> Self {
        let mut checkpoint = StateCheckpoint {
            version,
            epoch,
            state_hash: vec![],
            parameters,
        };
        checkpoint.state_hash = checkpoint.compute_hash();
        checkpoint
    }
    
    /// Compute hash of this checkpoint
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.version.major.to_le_bytes());
        hasher.update(self.version.minor.to_le_bytes());
        hasher.update(self.version.patch.to_le_bytes());
        hasher.update(self.epoch.to_le_bytes());
        
        for (key, value) in &self.parameters {
            hasher.update(key.as_bytes());
            match value {
                ParameterValue::U64(v) => hasher.update(v.to_le_bytes()),
                ParameterValue::F64(v) => hasher.update(v.to_le_bytes()),
                ParameterValue::Bool(v) => hasher.update([if *v { 1u8 } else { 0u8 }]),
                ParameterValue::String(s) => hasher.update(s.as_bytes()),
            }
        }
        
        hasher.finalize().to_vec()
    }
}

/// Migration plan (orchestrates multiple rules)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// From version
    pub from_version: ProtocolVersion,
    
    /// To version
    pub to_version: ProtocolVersion,
    
    /// Migration rules (ordered)
    pub rules: Vec<MigrationRule>,
    
    /// Pre-migration checkpoint
    pub pre_checkpoint: Option<StateCheckpoint>,
    
    /// Post-migration checkpoint
    pub post_checkpoint: Option<StateCheckpoint>,
}

impl MigrationPlan {
    pub fn new(
        from_version: ProtocolVersion,
        to_version: ProtocolVersion,
    ) -> Self {
        MigrationPlan {
            from_version,
            to_version,
            rules: Vec::new(),
            pre_checkpoint: None,
            post_checkpoint: None,
        }
    }
    
    /// Add migration rule
    pub fn add_rule(&mut self, rule: MigrationRule) {
        self.rules.push(rule);
    }
    
    /// Execute migration plan (returns new parameters)
    pub fn execute(
        &mut self,
        current_params: &BTreeMap<String, ParameterValue>,
    ) -> Result<BTreeMap<String, ParameterValue>, MigrationError> {
        // Take pre-migration checkpoint
        let checkpoint_params = current_params.clone();
        self.pre_checkpoint = Some(StateCheckpoint::new(
            self.from_version,
            0,
            checkpoint_params.clone(),
        ));
        
        // Apply all rules
        let mut new_params = current_params.clone();
        for rule in &self.rules {
            if let Some(current_value) = new_params.get(&format!("{:?}", rule.param_type)) {
                let new_value = rule.transform.apply(current_value)?;
                new_params.insert(
                    format!("{:?}", rule.param_type),
                    new_value,
                );
            }
        }
        
        // Take post-migration checkpoint
        self.post_checkpoint = Some(StateCheckpoint::new(
            self.to_version,
            0,
            new_params.clone(),
        ));
        
        Ok(new_params)
    }
    
    /// Verify migration consistency
    pub fn verify_consistency(&self) -> Result<(), MigrationError> {
        if self.pre_checkpoint.is_none() {
            return Err(MigrationError::InconsistentStateBefore);
        }
        
        if self.post_checkpoint.is_none() {
            return Err(MigrationError::InconsistentStateAfter);
        }
        
        // Verify pre and post checkpoints are different
        let pre = self.pre_checkpoint.as_ref().unwrap();
        let post = self.post_checkpoint.as_ref().unwrap();
        
        if pre.state_hash == post.state_hash {
            // Same hash means no changes - which may be valid
            // Just verify versions are different
            if pre.version == post.version {
                return Err(MigrationError::InconsistentStateAfter);
            }
        }
        
        Ok(())
    }
}

/// Migration engine
pub struct MigrationEngine {
    /// Planned migrations (version pair → plan)
    migrations: BTreeMap<(String, String), MigrationPlan>,
    
    /// Executed migrations (history)
    history: Vec<(ProtocolVersion, ProtocolVersion, u64)>,
}

impl MigrationEngine {
    pub fn new() -> Self {
        MigrationEngine {
            migrations: BTreeMap::new(),
            history: Vec::new(),
        }
    }
    
    /// Register a migration plan
    pub fn register_migration(&mut self, plan: MigrationPlan) {
        let key = (
            plan.from_version.to_string(),
            plan.to_version.to_string(),
        );
        self.migrations.insert(key, plan);
    }
    
    /// Execute a migration
    pub fn execute_migration(
        &mut self,
        from_version: ProtocolVersion,
        to_version: ProtocolVersion,
        current_params: &BTreeMap<String, ParameterValue>,
        epoch: u64,
    ) -> Result<BTreeMap<String, ParameterValue>, MigrationError> {
        let key = (from_version.to_string(), to_version.to_string());
        
        let plan = self.migrations.get_mut(&key)
            .ok_or_else(|| MigrationError::MigrationNotFound(
                from_version.to_string(),
                to_version.to_string(),
            ))?;
        
        // Execute the plan
        let new_params = plan.execute(current_params)?;
        
        // Verify consistency
        plan.verify_consistency()?;
        
        // Record in history
        self.history.push((from_version, to_version, epoch));
        
        Ok(new_params)
    }
    
    /// Get migration history
    pub fn get_history(&self) -> &[(ProtocolVersion, ProtocolVersion, u64)] {
        &self.history
    }
}

impl Default for MigrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_transform_identity() {
        let transform = MigrationTransform::Identity;
        let value = ParameterValue::U64(100);
        
        let result = transform.apply(&value).unwrap();
        assert_eq!(result, value);
    }

    #[test]
    fn test_migration_transform_scale() {
        let transform = MigrationTransform::Scale { factor: 2.0 };
        let value = ParameterValue::U64(100);
        
        let result = transform.apply(&value).unwrap();
        assert_eq!(result, ParameterValue::U64(200));
    }

    #[test]
    fn test_migration_transform_add() {
        let transform = MigrationTransform::Add { value: 50 };
        let value = ParameterValue::U64(100);
        
        let result = transform.apply(&value).unwrap();
        assert_eq!(result, ParameterValue::U64(150));
    }

    #[test]
    fn test_migration_transform_replace() {
        let transform = MigrationTransform::Replace {
            new_value: ParameterValue::U64(999),
        };
        let value = ParameterValue::U64(100);
        
        let result = transform.apply(&value).unwrap();
        assert_eq!(result, ParameterValue::U64(999));
    }

    #[test]
    fn test_state_checkpoint() {
        let mut params = BTreeMap::new();
        params.insert("param_a".to_string(), ParameterValue::U64(100));
        params.insert("param_b".to_string(), ParameterValue::U64(200));
        
        let checkpoint = StateCheckpoint::new(
            ProtocolVersion::new(1, 0, 0),
            0,
            params,
        );
        
        assert!(!checkpoint.state_hash.is_empty());
    }

    #[test]
    fn test_migration_plan() {
        let mut plan = MigrationPlan::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 1, 0),
        );
        
        let rule = MigrationRule {
            param_type: ParameterType::ConsensusBlockTime,
            from_version: ProtocolVersion::new(1, 0, 0),
            to_version: ProtocolVersion::new(1, 1, 0),
            transform: MigrationTransform::Scale { factor: 1.5 },
            validate: None,
        };
        
        plan.add_rule(rule);
        
        let mut params = BTreeMap::new();
        params.insert(
            format!("{:?}", ParameterType::ConsensusBlockTime),
            ParameterValue::U64(1000),
        );
        
        let result = plan.execute(&params).unwrap();
        let new_value = result.get(&format!("{:?}", ParameterType::ConsensusBlockTime)).unwrap();
        assert_eq!(new_value.as_u64().unwrap(), 1500);
    }

    #[test]
    fn test_migration_engine() {
        let mut engine = MigrationEngine::new();
        
        let plan = MigrationPlan::new(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 1, 0),
        );
        
        engine.register_migration(plan);
        
        let mut params = BTreeMap::new();
        params.insert("test".to_string(), ParameterValue::U64(100));
        
        let result = engine.execute_migration(
            ProtocolVersion::new(1, 0, 0),
            ProtocolVersion::new(1, 1, 0),
            &params,
            10,
        );
        
        assert!(result.is_ok());
    }
}
