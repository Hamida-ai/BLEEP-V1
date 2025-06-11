use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use rand::Rng;
use log::{info, warn};
use ring::{digest, rand::SystemRandom};
use pqcrypto::sign::sphincs::*;
use tch::{nn, Tensor}; // AI-based consensus prediction
use crate::{
    Transaction, BlockchainState, BLEEPError, Block, NetworkingModule, AIEngine,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusMode {
    PoS,   // Proof of Stake
    PBFT,  // Practical Byzantine Fault Tolerance
    PoW,   // Proof of Work
}

#[derive(Debug, Clone)]
pub struct Validator {
    pub id: String,
    pub reputation: f64,
    pub latency: u64,
    pub stake: u64,
    pub active: bool,
    pub last_signed_block: u64,
}

pub struct BLEEPAdaptiveConsensus {
    consensus_mode: ConsensusMode,
    network_reliability: f64,
    validators: HashMap<String, Validator>,
    pow_difficulty: usize,
    networking: Arc<NetworkingModule>,
    ai_engine: Arc<AIEngine>,
}

impl BLEEPAdaptiveConsensus {
    pub fn new(
        validators: HashMap<String, Validator>,
        networking: Arc<NetworkingModule>,
        ai_engine: Arc<AIEngine>,
    ) -> Self {
        let initial_mode = ConsensusMode::PoS;
        BLEEPAdaptiveConsensus {
            consensus_mode: initial_mode,
            network_reliability: 0.95,
            validators,
            pow_difficulty: 4,
            networking,
            ai_engine,
        }
    }

    pub fn switch_consensus_mode(&mut self, network_load: u64, avg_latency: u64) {
        let predicted_mode = self.ai_engine.predict_consensus(network_load, avg_latency);
        if self.consensus_mode != predicted_mode {
            info!("Switching consensus mode to {:?}", predicted_mode);
            self.consensus_mode = predicted_mode;
        }
    }

    pub fn finalize_block(&mut self, block: &Block, state: &mut BlockchainState) -> Result<(), BLEEPError> {
        let success = match self.consensus_mode {
            ConsensusMode::PoS => self.pos_algorithm(block, state),
            ConsensusMode::PBFT => self.pbft_algorithm(block, state),
            ConsensusMode::PoW => self.pow_algorithm(block),
        };

        if success {
            info!("Block finalized successfully using {:?}", self.consensus_mode);
            Ok(())
        } else {
            warn!("Block finalization failed. Adjusting strategy...");
            self.switch_consensus_mode(50, 40);
            self.finalize_block(block, state)
        }
    }

    fn pos_algorithm(&self, block: &Block, state: &mut BlockchainState) -> bool {
        let mut validators_sorted: Vec<&Validator> = self.validators.values().collect();
        validators_sorted.sort_by(|a, b| b.stake.cmp(&a.stake));
        let selected_validator = validators_sorted.first();

        if let Some(validator) = selected_validator {
            if validator.reputation > 0.8 {
                return state.add_block(block.clone()).is_ok();
            }
        }
        false
    }

    fn pow_algorithm(&mut self, block: &Block) -> bool {
        let target = "0".repeat(self.pow_difficulty);
        let mut nonce = 0;
        let mut hasher = digest::Context::new(&digest::SHA256);

        loop {
            hasher.update(format!("{:?}{}", block, nonce).as_bytes());
            let hash = hex::encode(hasher.clone().finish());

            if hash.starts_with(&target) {
                info!("PoW successful: Nonce = {}, Hash = {}", nonce, hash);
                self.adjust_pow_difficulty();
                return true;
            }

            nonce += 1;
            if nonce > 10_000_000 {
                warn!("PoW failed: Max attempts exceeded.");
                return false;
            }
        }
    }

    fn adjust_pow_difficulty(&mut self) {
        let avg_network_hashrate = self.networking.get_network_hashrate();
        if avg_network_hashrate > 500 {
            self.pow_difficulty += 1;
        } else if self.pow_difficulty > 2 {
            self.pow_difficulty -= 1;
        }
        info!("Adjusted PoW difficulty: {}", self.pow_difficulty);
    }

    fn pbft_algorithm(&self, block: &Block, state: &mut BlockchainState) -> bool {
        let leader = self.select_pbft_leader();
        if leader.is_none() {
            return false;
        }
        let leader_id = leader.unwrap().id.clone();

        if !self.networking.broadcast_proposal(&block, &leader_id) {
            return false;
        }

        let prepare_votes = self.collect_votes(block, "prepare");
        if !self.has_quorum(&prepare_votes) {
            warn!("PBFT: Insufficient quorum in prepare phase.");
            return false;
        }

        let commit_votes = self.collect_votes(block, "commit");
        if self.has_quorum(&commit_votes) {
            return state.add_block(block.clone()).is_ok();
        }

        warn!("PBFT: Commit phase failed.");
        false
    }

    fn select_pbft_leader(&self) -> Option<&Validator> {
        let active_validators: Vec<&Validator> = self
            .validators
            .values()
            .filter(|v| v.active && v.reputation > 0.7)
            .collect();

        if active_validators.is_empty() {
            warn!("No eligible PBFT leaders available.");
            return None;
        }

        let leader = active_validators.iter().max_by(|a, b| a.stake.cmp(&b.stake));
        leader.cloned()
    }

    fn collect_votes(&self, block: &Block, phase: &str) -> HashSet<String> {
        info!("Collecting {:?} votes for block {:?}", phase, block);
        let mut votes = HashSet::new();
        for (id, validator) in &self.validators {
            if validator.reputation > 0.75 {
                votes.insert(id.clone());
            }
        }
        votes
    }

    fn has_quorum(&self, votes: &HashSet<String>) -> bool {
        let required_votes = (self.validators.len() as f64 * 0.66).ceil() as usize;
        votes.len() >= required_votes
    }

    pub fn monitor_validators(&mut self) {
        let anomalies = self.ai_engine.detect_anomalies(&self.validators);
        for (id, score) in anomalies.iter() {
            if *score > 0.8 {
                warn!("Validator {} detected as malicious! Reducing reputation.", id);
                if let Some(validator) = self.validators.get_mut(id) {
                    validator.reputation *= 0.5;
                    validator.active = false;
                }
            }
        }
    }

    pub fn sign_block(&self, block: &Block, validator_id: &str) -> Vec<u8> {
        let sk = SecretKey::generate();
        let signature = sign(&block.hash(), &sk);
        signature.to_vec()
    }

    pub fn verify_signature(&self, block: &Block, signature: &[u8], validator_id: &str) -> bool {
        if let Some(validator) = self.validators.get(validator_id) {
            let pk = PublicKey::from_secret_key(&SecretKey::generate());
            verify(&block.hash(), signature, &pk).is_ok()
        } else {
            false
        }
    }
}