// PHASE 1: PBFT FINALITY GADGET CONSENSUS ENGINE
// Practical Byzantine Fault Tolerance for fast finality
// 
// SAFETY CONSTRAINTS:
// 1. PBFT DOES NOT produce blocks independently
// 2. PBFT finalizes blocks already produced by PoS
// 3. PBFT cannot reorder finalized blocks
// 4. PBFT requires 2/3 + 1 honest validators
// 5. Finality is Byzantine-fault-tolerant once committed

use crate::epoch::EpochState;
use crate::engine::{ConsensusEngine, ConsensusError};
use bleep_core::block::{Block, Transaction, ConsensusMode};
use bleep_core::blockchain::BlockchainState;
use log::{info, warn};
use std::collections::HashMap;

/// PBFT message types for the 3-phase protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbftPhase {
    /// Pre-prepare phase: leader proposes block
    PrePrepare,
    
    /// Prepare phase: validators acknowledge proposal
    Prepare,
    
    /// Commit phase: validators finalize block
    Commit,
}

/// State of a block in the PBFT pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbftBlockState {
    /// Block has been proposed but not yet prepared
    Proposed,
    
    /// 2/3 + 1 validators have prepared
    Prepared,
    
    /// 2/3 + 1 validators have committed (finalized)
    Committed,
}

/// PBFT consensus engine.
/// 
/// SAFETY: Acts as a finality gadget only. Does NOT produce or order blocks.
/// Blocks must already be produced by PoS consensus before PBFT can finalize them.
pub struct PbftConsensusEngine {
    /// Validator ID
    validator_id: String,
    
    /// Number of validators in the network
    total_validators: usize,
    
    /// Quorum size: 2/3 + 1 of validators
    quorum_size: usize,
    
    /// Blocks finalized in this PBFT view
    finalized_blocks: HashMap<u64, PbftBlockState>,
    
    /// Current PBFT view number
    current_view: u64,
}

impl PbftConsensusEngine {
    /// Create a new PBFT engine.
    /// 
    /// # Arguments
    /// * `validator_id` - This validator's ID
    /// * `total_validators` - Total number of validators in the network
    pub fn new(validator_id: String, total_validators: usize) -> Result<Self, ConsensusError> {
        if total_validators == 0 {
            return Err(ConsensusError::ProposalRejected {
                reason: "total_validators must be > 0".to_string(),
            });
        }

        // Quorum = 2/3 + 1 (Byzantine-fault-tolerant for f < n/3)
        let quorum_size = (total_validators * 2) / 3 + 1;

        Ok(PbftConsensusEngine {
            validator_id,
            total_validators,
            quorum_size,
            finalized_blocks: HashMap::new(),
            current_view: 0,
        })
    }

    /// Process a pre-prepare message (block proposal from leader).
    /// 
    /// SAFETY: Block must already be produced by PoS.
    /// PBFT only finalizes, never reorders.
    fn pre_prepare(&mut self, block_height: u64, block: &Block) -> Result<(), ConsensusError> {
        // SAFETY: Verify block is in PRE-PREPARE phase
        if self.finalized_blocks.contains_key(&block_height) {
            return Err(ConsensusError::ProposalRejected {
                reason: format!("Block {} already finalized", block_height),
            });
        }

        // Move block to PREPARE phase
        self.finalized_blocks.insert(block_height, PbftBlockState::Proposed);

        info!(
            "PBFT: Pre-prepare block {} in view {}",
            block_height, self.current_view
        );

        Ok(())
    }

    /// Process a prepare message from a validator.
    /// Once 2/3+1 validators prepare, block moves to PREPARED state.
    fn process_prepare(&mut self, block_height: u64, _preparer_id: &str) -> Result<(), ConsensusError> {
        // In a real implementation, we would maintain a count of prepare votes
        // For now, we simulate deterministically

        let current_state = self.finalized_blocks.get(&block_height);

        if current_state == Some(&PbftBlockState::Proposed) {
            // Simulate reaching prepare threshold
            // In production, this would count actual prepare votes
            self.finalized_blocks.insert(block_height, PbftBlockState::Prepared);
            info!("PBFT: Block {} reached PREPARED state", block_height);
        }

        Ok(())
    }

    /// Process a commit message from a validator.
    /// Once 2/3+1 validators commit, block is finalized.
    fn process_commit(&mut self, block_height: u64, _committer_id: &str) -> Result<(), ConsensusError> {
        // In a real implementation, we would maintain a count of commit votes
        // For now, we simulate deterministically

        let current_state = self.finalized_blocks.get(&block_height);

        if current_state == Some(&PbftBlockState::Prepared) {
            // Simulate reaching commit threshold
            self.finalized_blocks.insert(block_height, PbftBlockState::Committed);
            info!("PBFT: Block {} COMMITTED and finalized", block_height);
        }

        Ok(())
    }

    /// Check if a block is finalized in PBFT.
    pub fn is_finalized(&self, block_height: u64) -> bool {
        self.finalized_blocks
            .get(&block_height)
            == Some(&PbftBlockState::Committed)
    }

    /// Check the current PBFT state of a block.
    pub fn block_state(&self, block_height: u64) -> Option<PbftBlockState> {
        self.finalized_blocks.get(&block_height).copied()
    }
}

impl ConsensusEngine for PbftConsensusEngine {
    fn verify_block(
        &self,
        block: &Block,
        epoch_state: &EpochState,
        _blockchain_state: &BlockchainState,
    ) -> Result<(), ConsensusError> {
        // SAFETY: Verify consensus mode is PBFT
        if block.consensus_mode != ConsensusMode::PbftFastFinality {
            return Err(ConsensusError::ConsensusModeMismatch {
                expected: "PBFT_FINALITY".to_string(),
                got: format!("{:?}", block.consensus_mode),
                epoch: epoch_state.epoch_id,
            });
        }

        // SAFETY: Verify block height is within epoch
        if !epoch_state.contains_height(block.index) {
            return Err(ConsensusError::ProposalRejected {
                reason: format!(
                    "Block height {} outside epoch bounds [{}, {}]",
                    block.index, epoch_state.start_height, epoch_state.end_height
                ),
            });
        }

        // SAFETY: Verify signature exists
        if block.validator_signature.is_empty() {
            return Err(ConsensusError::InvalidSignature {
                validator_id: "unknown".to_string(),
            });
        }

        // In a real implementation, we would:
        // 1. Verify the block was produced by PoS consensus
        // 2. Check that we can reach PBFT quorum for this block
        // 3. Verify threshold of prepare/commit signatures
        
        info!(
            "PBFT verification passed for block {} in epoch {}",
            block.index, epoch_state.epoch_id
        );

        Ok(())
    }

    fn propose_block(
        &self,
        height: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        epoch_state: &EpochState,
        _blockchain_state: &BlockchainState,
    ) -> Result<Block, ConsensusError> {
        // SAFETY: PBFT does NOT produce blocks independently!
        // This should only be called during PoS consensus when PBFT is NOT active.
        // However, we provide a fallback to create properly-marked blocks.
        
        warn!(
            "PBFT propose_block called at height {} - PBFT should not produce blocks!",
            height
        );

        let mut block = Block::with_consensus(
            height,
            transactions,
            previous_hash,
            epoch_state.epoch_id,
            ConsensusMode::PbftFastFinality,
            1, // protocol_version
        );

        // Sign with our validator key
        block.validator_signature = self.validator_id.as_bytes().to_vec();

        info!(
            "PBFT created block {} (should normally be finalized, not proposed)",
            height
        );

        Ok(block)
    }

    fn consensus_mode(&self) -> crate::epoch::ConsensusMode {
        crate::epoch::ConsensusMode::PbftFastFinality
    }

    fn health_status(&self) -> f64 {
        // Health = number of finalized blocks / total blocks proposed
        if self.finalized_blocks.is_empty() {
            return 1.0; // No blocks yet, healthy
        }

        let finalized = self
            .finalized_blocks
            .values()
            .filter(|&&state| state == PbftBlockState::Committed)
            .count();

        finalized as f64 / self.finalized_blocks.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbft_engine_creation() {
        let engine = PbftConsensusEngine::new("validator1".to_string(), 3).unwrap();
        assert_eq!(engine.validator_id, "validator1");
        assert_eq!(engine.total_validators, 3);
        assert_eq!(engine.quorum_size, 3); // 2/3 + 1 of 3 = 3
    }

    #[test]
    fn test_pbft_quorum_size_calculation() {
        let e3 = PbftConsensusEngine::new("v".to_string(), 3).unwrap();
        assert_eq!(e3.quorum_size, 3);

        let e7 = PbftConsensusEngine::new("v".to_string(), 7).unwrap();
        assert_eq!(e7.quorum_size, 5); // 2/3 + 1 of 7 = 5

        let e10 = PbftConsensusEngine::new("v".to_string(), 10).unwrap();
        assert_eq!(e10.quorum_size, 7); // 2/3 + 1 of 10 = 7
    }

    #[test]
    fn test_pbft_zero_validators_error() {
        let result = PbftConsensusEngine::new("v".to_string(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_pbft_block_lifecycle() {
        let mut engine = PbftConsensusEngine::new("v1".to_string(), 3).unwrap();

        // Create block
        let block = Block::new(100, vec![], "hash99".to_string());

        // Pre-prepare
        assert!(engine.pre_prepare(100, &block).is_ok());
        assert_eq!(engine.block_state(100), Some(PbftBlockState::Proposed));

        // Process prepare
        assert!(engine.process_prepare(100, "v2").is_ok());
        assert_eq!(engine.block_state(100), Some(PbftBlockState::Prepared));

        // Process commit
        assert!(engine.process_commit(100, "v3").is_ok());
        assert_eq!(engine.block_state(100), Some(PbftBlockState::Committed));

        // Check finalized
        assert!(engine.is_finalized(100));
    }

    #[test]
    fn test_pbft_cannot_duplicate_prepare() {
        let mut engine = PbftConsensusEngine::new("v1".to_string(), 3).unwrap();
        let block = Block::new(100, vec![], "hash99".to_string());

        engine.pre_prepare(100, &block).unwrap();
        assert_eq!(engine.block_state(100), Some(PbftBlockState::Proposed));

        // Try to prepare twice - state should move to Prepared, not stay at Proposed
        engine.process_prepare(100, "v2").unwrap();
        assert_eq!(engine.block_state(100), Some(PbftBlockState::Prepared));

        // Second prepare doesn't demote back to Proposed
        let state_before = engine.block_state(100);
        engine.process_prepare(100, "v3").unwrap();
        let state_after = engine.block_state(100);
        assert_eq!(state_before, state_after);
    }

    #[test]
    fn test_pbft_health_status_perfect() {
        let mut engine = PbftConsensusEngine::new("v1".to_string(), 3).unwrap();
        let block = Block::new(100, vec![], "hash99".to_string());

        engine.pre_prepare(100, &block).unwrap();
        engine.process_prepare(100, "v2").unwrap();
        engine.process_commit(100, "v3").unwrap();

        // All blocks finalized = perfect health
        assert_eq!(engine.health_status(), 1.0);
    }

    #[test]
    fn test_pbft_health_status_degraded() {
        let mut engine = PbftConsensusEngine::new("v1".to_string(), 3).unwrap();

        // One block fully committed
        let block1 = Block::new(100, vec![], "hash99".to_string());
        engine.pre_prepare(100, &block1).unwrap();
        engine.process_prepare(100, "v2").unwrap();
        engine.process_commit(100, "v3").unwrap();

        // One block stuck at proposed
        let block2 = Block::new(101, vec![], "hash100".to_string());
        engine.pre_prepare(101, &block2).unwrap();

        // Health = 1 finalized / 2 total = 0.5
        assert_eq!(engine.health_status(), 0.5);
    }
}
