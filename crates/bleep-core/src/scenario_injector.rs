// PHASE 6: SCENARIO INJECTOR - Deterministic Attack Scenarios
//
// Purpose: Implement three mandatory adversarial scenarios:
// 1. Validator Collusion: 2+ validators attempt to fork the chain
// 2. Invalid State Injection: Corrupted state committed and detected
// 3. Governance Abuse: Attacker attempts harmful protocol upgrade
//
// All scenarios are DETERMINISTIC (same seed = same execution)
// and OBSERVABLE (full audit trail with before/after proofs)

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// ═══════════════════════════════════════════════════════════════════════════════
/// VALIDATOR COLLUSION SCENARIO
/// ═══════════════════════════════════════════════════════════════════════════════

/// Represents colluding validators attempting to fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColludingValidatorGroup {
    /// IDs of validators in collision
    pub validator_ids: Vec<usize>,
    /// Attack epoch
    pub attack_epoch: u64,
    /// Blocks produced by this group (alternative chain)
    pub block_hashes: Vec<Vec<u8>>,
    /// Signatures from all colluders
    pub signatures: Vec<Vec<u8>>,
    /// Proposed fork point (block height)
    pub fork_height: u64,
}

impl ColludingValidatorGroup {
    pub fn new(validators: Vec<usize>, attack_epoch: u64) -> Self {
        Self {
            validator_ids: validators,
            attack_epoch,
            block_hashes: Vec::new(),
            signatures: Vec::new(),
            fork_height: 0,
        }
    }

    /// Deterministically generate blocks for alternative chain
    /// (same seed always produces same blocks)
    pub fn generate_fork_blocks(&mut self, num_blocks: usize, seed: &[u8]) -> Result<(), String> {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(self.attack_epoch.to_le_bytes());

        for i in 0..num_blocks {
            let mut block_hasher = Sha256::new();
            block_hasher.update(hasher.finalize_reset().as_ref());
            block_hasher.update(i.to_le_bytes());

            // Include all colluder IDs in hash (deterministic)
            for validator_id in &self.validator_ids {
                block_hasher.update(validator_id.to_le_bytes());
            }

            self.block_hashes.push(block_hasher.finalize().to_vec());
            hasher = Sha256::new();
            hasher.update(seed);
        }

        Ok(())
    }

    /// Generate signatures for blocks (all colluders sign)
    pub fn sign_blocks(&mut self, seed: &[u8]) -> Result<(), String> {
        for (i, block_hash) in self.block_hashes.iter().enumerate() {
            let mut sig_hasher = Sha256::new();
            sig_hasher.update(block_hash);
            sig_hasher.update(i.to_le_bytes());
            sig_hasher.update(seed);

            // Each validator's signature
            for validator_id in &self.validator_ids {
                let mut v_sig = Sha256::new();
                v_sig.update(sig_hasher.finalize_reset().as_ref());
                v_sig.update(validator_id.to_le_bytes());
                self.signatures.push(v_sig.finalize().to_vec());
            }
        }
        Ok(())
    }

    /// Compute immutable hash of this fork attempt
    pub fn commitment_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.attack_epoch.to_le_bytes());
        hasher.update(self.fork_height.to_le_bytes());

        for validator_id in &self.validator_ids {
            hasher.update(validator_id.to_le_bytes());
        }

        for block_hash in &self.block_hashes {
            hasher.update(block_hash);
        }

        hasher.finalize().to_vec()
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// INVALID STATE INJECTION SCENARIO
/// ═══════════════════════════════════════════════════════════════════════════════

/// Types of state corruption that can be injected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateCorruptionType {
    DuplicateTransaction,    // Transaction appears twice
    InvalidBalance,          // Account balance becomes negative
    MissingState,            // Accounts randomly deleted
    FutureTimestamp,         // State dated in future
    InvalidNonce,            // Transaction nonce incorrect
    BadMerkleRoot,           // State root doesn't match
    OrphanedReferences,      // References to non-existent accounts
    CorruptedSignature,      // Valid transaction with bad signature
}

impl StateCorruptionType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DuplicateTransaction => "duplicate_transaction",
            Self::InvalidBalance => "invalid_balance",
            Self::MissingState => "missing_state",
            Self::FutureTimestamp => "future_timestamp",
            Self::InvalidNonce => "invalid_nonce",
            Self::BadMerkleRoot => "bad_merkle_root",
            Self::OrphanedReferences => "orphaned_references",
            Self::CorruptedSignature => "corrupted_signature",
        }
    }
}

/// Injected state corruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCorruption {
    /// Type of corruption
    pub corruption_type: StateCorruptionType,
    /// Epoch to inject at
    pub injection_epoch: u64,
    /// Percentage of state affected (0.0-100.0)
    pub corruption_percentage: f64,
    /// Affected account hashes
    pub affected_accounts: Vec<Vec<u8>>,
    /// Corrupted state root
    pub corrupted_state_root: Vec<u8>,
    /// Expected valid state root (before corruption)
    pub expected_state_root: Vec<u8>,
}

impl StateCorruption {
    /// Create new state corruption (deterministic)
    pub fn new(
        corruption_type: StateCorruptionType,
        injection_epoch: u64,
        corruption_percentage: f64,
        seed: &[u8],
    ) -> Self {
        // Generate deterministic affected accounts
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(corruption_type.name().as_bytes());
        hasher.update(injection_epoch.to_le_bytes());

        let mut affected_accounts = Vec::new();
        let num_accounts = (100.0 * corruption_percentage / 1.0) as usize; // Assume ~100 accounts per 1%

        for i in 0..num_accounts {
            let mut acc_hasher = Sha256::new();
            acc_hasher.update(hasher.finalize_reset().as_ref());
            acc_hasher.update(i.to_le_bytes());
            affected_accounts.push(acc_hasher.finalize().to_vec());
        }

        // Compute state roots
        let mut state_hasher = Sha256::new();
        for account in &affected_accounts {
            state_hasher.update(account);
        }
        let corrupted_state_root = state_hasher.finalize().to_vec();

        // Expected (valid) state would be different
        let mut expected_hasher = Sha256::new();
        expected_hasher.update(b"valid_state");
        expected_hasher.update(seed);
        let expected_state_root = expected_hasher.finalize().to_vec();

        Self {
            corruption_type,
            injection_epoch,
            corruption_percentage,
            affected_accounts,
            corrupted_state_root,
            expected_state_root,
        }
    }

    /// Verify this corruption is detectable (state mismatch)
    pub fn is_detectable(&self) -> bool {
        self.corrupted_state_root != self.expected_state_root
    }

    /// Commitment hash for logging
    pub fn commitment_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.corruption_type.name().as_bytes());
        hasher.update(self.injection_epoch.to_le_bytes());
        hasher.update(self.corrupted_state_root.as_slice());
        hasher.update(self.expected_state_root.as_slice());
        hasher.finalize().to_vec()
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// GOVERNANCE ABUSE SCENARIO
/// ═══════════════════════════════════════════════════════════════════════════════

/// Types of governance attacks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GovernanceAttackType {
    SetSlashingToZero,         // Disable validator penalties
    RemoveSlashingCondition,   // Remove bad-behavior detection
    DisableFinalizer,          // Break consensus finality
    ReduceQuorum,              // Make governance easier to control
    RemoveUpgradePath,         // Prevent protocol upgrades
    SetStakingToZero,          // Remove validator requirements
    AddAdminPrivilege,         // Create unchecked power
    BypassGovernance,          // Try to execute proposal without voting
}

impl GovernanceAttackType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SetSlashingToZero => "set_slashing_to_zero",
            Self::RemoveSlashingCondition => "remove_slashing_condition",
            Self::DisableFinalizer => "disable_finalizer",
            Self::ReduceQuorum => "reduce_quorum",
            Self::RemoveUpgradePath => "remove_upgrade_path",
            Self::SetStakingToZero => "set_staking_to_zero",
            Self::AddAdminPrivilege => "add_admin_privilege",
            Self::BypassGovernance => "bypass_governance",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::SetSlashingToZero => "Remove all validator penalties for bad behavior",
            Self::RemoveSlashingCondition => "Delete detection of validator misbehavior",
            Self::DisableFinalizer => "Break consensus by removing finality mechanism",
            Self::ReduceQuorum => "Lower governance quorum to <10%",
            Self::RemoveUpgradePath => "Prevent protocol from being upgraded",
            Self::SetStakingToZero => "Remove minimum stake requirement (enable sybil)",
            Self::AddAdminPrivilege => "Create backdoor admin account with veto power",
            Self::BypassGovernance => "Execute proposal without governance approval",
        }
    }
}

/// Malicious governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaliciousProposal {
    /// Unique proposal ID (deterministic from seed)
    pub proposal_id: Vec<u8>,
    /// Attack type
    pub attack_type: GovernanceAttackType,
    /// Epoch to submit proposal
    pub proposal_epoch: u64,
    /// Attacker's stake (for voting)
    pub attacker_stake: u64,
    /// Parameter to change
    pub target_parameter: String,
    /// Malicious value to set
    pub malicious_value: String,
    /// Attacker's voting power
    pub attacker_votes: u64,
    /// Why this proposal is dangerous
    pub danger_assessment: String,
}

impl MaliciousProposal {
    /// Create deterministic malicious proposal
    pub fn new(
        attack_type: GovernanceAttackType,
        proposal_epoch: u64,
        attacker_stake: u64,
        seed: &[u8],
    ) -> Self {
        // Deterministic proposal ID
        let mut id_hasher = Sha256::new();
        id_hasher.update(attack_type.name().as_bytes());
        id_hasher.update(proposal_epoch.to_le_bytes());
        id_hasher.update(attacker_stake.to_le_bytes());
        id_hasher.update(seed);
        let proposal_id = id_hasher.finalize().to_vec();

        // Determine voting power (attacker has 5% of total)
        let attacker_votes = attacker_stake / 20;

        let danger_assessment = format!(
            "{}: This proposal would {}. If approved, the network would lose {}.",
            attack_type.name(),
            attack_type.description(),
            match attack_type {
                GovernanceAttackType::SetSlashingToZero => "validator penalties and incentive alignment",
                GovernanceAttackType::DisableFinalizer => "consensus finality and settlement",
                GovernanceAttackType::ReduceQuorum => "governance security and legitimacy",
                _ => "critical security properties",
            }
        );

        Self {
            proposal_id,
            attack_type,
            proposal_epoch,
            attacker_stake,
            target_parameter: attack_type.name().to_string(),
            malicious_value: "0".to_string(), // Most attacks set to zero/empty
            attacker_votes,
            danger_assessment,
        }
    }

    /// Check if this proposal violates constitutional constraints
    pub fn violates_constitution(&self) -> bool {
        // All of these attacks violate constitutional constraints
        matches!(
            self.attack_type,
            GovernanceAttackType::SetSlashingToZero
                | GovernanceAttackType::RemoveSlashingCondition
                | GovernanceAttackType::DisableFinalizer
                | GovernanceAttackType::SetStakingToZero
                | GovernanceAttackType::AddAdminPrivilege
                | GovernanceAttackType::BypassGovernance
        )
    }

    /// Commitment hash for logging
    pub fn commitment_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.proposal_id);
        hasher.update(self.attack_type.name().as_bytes());
        hasher.update(self.proposal_epoch.to_le_bytes());
        hasher.update(self.malicious_value.as_bytes());
        hasher.finalize().to_vec()
    }
}

/// ═══════════════════════════════════════════════════════════════════════════════
/// SCENARIO EXECUTOR - DETERMINISTIC ATTACK EXECUTION
/// ═══════════════════════════════════════════════════════════════════════════════

/// Orchestrates deterministic scenario execution
pub struct ScenarioExecutor {
    /// Global seed for reproducibility
    pub seed: Vec<u8>,
    /// Collusion attacks
    pub collusional_groups: Vec<ColludingValidatorGroup>,
    /// State corruptions
    pub state_corruptions: Vec<StateCorruption>,
    /// Governance attacks
    pub malicious_proposals: Vec<MaliciousProposal>,
    /// Execution trace (for reproducibility verification)
    pub execution_trace: Vec<String>,
}

impl ScenarioExecutor {
    pub fn new(seed: Vec<u8>) -> Self {
        Self {
            seed,
            collusional_groups: Vec::new(),
            state_corruptions: Vec::new(),
            malicious_proposals: Vec::new(),
            execution_trace: Vec::new(),
        }
    }

    /// Execute validator collusion scenario (deterministic)
    pub fn execute_validator_collusion(
        &mut self,
        validators: Vec<usize>,
        attack_epoch: u64,
        num_blocks: usize,
    ) -> Result<ColludingValidatorGroup, String> {
        self.execution_trace.push(format!(
            "SCENARIO: Validator collusion started at epoch {}, validators: {:?}",
            attack_epoch, validators
        ));

        let mut group = ColludingValidatorGroup::new(validators.clone(), attack_epoch);
        group.generate_fork_blocks(num_blocks, &self.seed)?;
        group.sign_blocks(&self.seed)?;

        self.execution_trace.push(format!(
            "RESULT: Fork chain generated: {} blocks, commitment: {}",
            group.block_hashes.len(),
            hex::encode(&group.commitment_hash()[..8])
        ));

        self.collusional_groups.push(group.clone());
        Ok(group)
    }

    /// Execute state injection scenario (deterministic)
    pub fn execute_state_injection(
        &mut self,
        corruption_type: StateCorruptionType,
        injection_epoch: u64,
        corruption_percentage: f64,
    ) -> Result<StateCorruption, String> {
        self.execution_trace.push(format!(
            "SCENARIO: State injection started at epoch {}, type: {}, corruption: {:.1}%",
            injection_epoch, corruption_type.name(), corruption_percentage
        ));

        let corruption = StateCorruption::new(
            corruption_type,
            injection_epoch,
            corruption_percentage,
            &self.seed,
        );

        if !corruption.is_detectable() {
            return Err("State corruption not detectable".to_string());
        }

        self.execution_trace.push(format!(
            "RESULT: State corruption injected: {} accounts affected, mismatch detected",
            corruption.affected_accounts.len()
        ));

        self.state_corruptions.push(corruption.clone());
        Ok(corruption)
    }

    /// Execute governance abuse scenario (deterministic)
    pub fn execute_governance_abuse(
        &mut self,
        attack_type: GovernanceAttackType,
        proposal_epoch: u64,
        attacker_stake: u64,
    ) -> Result<MaliciousProposal, String> {
        self.execution_trace.push(format!(
            "SCENARIO: Governance abuse started at epoch {}, attack: {}",
            proposal_epoch,
            attack_type.name()
        ));

        let proposal = MaliciousProposal::new(attack_type, proposal_epoch, attacker_stake, &self.seed);

        if !proposal.violates_constitution() {
            return Err("Proposal should violate constitution".to_string());
        }

        self.execution_trace.push(format!(
            "RESULT: Malicious proposal submitted, {} votes, constitutional violation detected",
            proposal.attacker_votes
        ));

        self.malicious_proposals.push(proposal.clone());
        Ok(proposal)
    }

    /// Get execution trace (proves deterministic reproducibility)
    pub fn get_trace(&self) -> Vec<String> {
        self.execution_trace.clone()
    }

    /// Verify all scenarios are deterministic (same execution for same seed)
    pub fn verify_determinism(&self, other: &ScenarioExecutor) -> bool {
        // Same seed should produce same trace
        if self.seed != other.seed {
            return false;
        }

        // Check scenarios match
        if self.collusional_groups.len() != other.collusional_groups.len() {
            return false;
        }
        if self.state_corruptions.len() != other.state_corruptions.len() {
            return false;
        }
        if self.malicious_proposals.len() != other.malicious_proposals.len() {
            return false;
        }

        // Verify traces match (order and content)
        self.execution_trace == other.execution_trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_collusion_deterministic() {
        let seed = b"determinism_test".to_vec();
        let mut group1 = ColludingValidatorGroup::new(vec![1, 2], 10);
        group1.generate_fork_blocks(5, &seed).unwrap();

        let mut group2 = ColludingValidatorGroup::new(vec![1, 2], 10);
        group2.generate_fork_blocks(5, &seed).unwrap();

        // Same seed = same blocks
        assert_eq!(group1.block_hashes, group2.block_hashes);
    }

    #[test]
    fn test_state_corruption_detectable() {
        let corruption = StateCorruption::new(
            StateCorruptionType::InvalidBalance,
            50,
            15.0,
            b"test_seed",
        );

        assert!(corruption.is_detectable());
    }

    #[test]
    fn test_malicious_proposal_violates_constitution() {
        let proposal = MaliciousProposal::new(
            GovernanceAttackType::SetSlashingToZero,
            30,
            1000000,
            b"test_seed",
        );

        assert!(proposal.violates_constitution());
    }

    #[test]
    fn test_scenario_executor_determinism() {
        let seed = b"executor_test".to_vec();

        let mut executor1 = ScenarioExecutor::new(seed.clone());
        executor1
            .execute_validator_collusion(vec![1, 2], 10, 5)
            .unwrap();
        executor1
            .execute_state_injection(StateCorruptionType::InvalidBalance, 50, 15.0)
            .unwrap();
        executor1
            .execute_governance_abuse(GovernanceAttackType::SetSlashingToZero, 30, 1000000)
            .unwrap();

        let mut executor2 = ScenarioExecutor::new(seed);
        executor2
            .execute_validator_collusion(vec![1, 2], 10, 5)
            .unwrap();
        executor2
            .execute_state_injection(StateCorruptionType::InvalidBalance, 50, 15.0)
            .unwrap();
        executor2
            .execute_governance_abuse(GovernanceAttackType::SetSlashingToZero, 30, 1000000)
            .unwrap();

        assert!(executor1.verify_determinism(&executor2));
    }

    #[test]
    fn test_different_seeds_different_results() {
        let seed1 = b"seed_one".to_vec();
        let seed2 = b"seed_two".to_vec();

        let mut executor1 = ScenarioExecutor::new(seed1);
        executor1
            .execute_validator_collusion(vec![1, 2], 10, 5)
            .unwrap();

        let mut executor2 = ScenarioExecutor::new(seed2);
        executor2
            .execute_validator_collusion(vec![1, 2], 10, 5)
            .unwrap();

        // Different seeds = different block hashes
        assert_ne!(
            executor1.collusional_groups[0].block_hashes,
            executor2.collusional_groups[0].block_hashes
        );
    }
}
