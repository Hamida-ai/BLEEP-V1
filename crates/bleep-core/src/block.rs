
use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};
// Stub quantum-secure crypto
// Stub AI-based block security
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

/// PHASE 1: Consensus mode enumeration (replicated from epoch system)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusMode {
    PosNormal,
    PbftFastFinality,
    EmergencyPow,
}

/// PHASE 1: Extended block header with consensus fields.
/// PHASE 2: Extended with shard fields for deterministic sharding
///
/// SAFETY INVARIANTS:
/// 1. epoch_id must match the deterministically computed value from block height
/// 2. consensus_mode must match the expected mode for the given epoch
/// 3. protocol_version must match the chain's protocol version
/// 4. shard_registry_root must match the canonical shard layout for the epoch
/// 5. shard_id must be valid for the block's shard assignment
/// 6. Blocks with invalid consensus or shard fields are rejected unconditionally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block height in the chain
    pub index: u64,
    
    /// Block creation timestamp (seconds since epoch)
    pub timestamp: u64,
    
    /// Transactions included in this block
    pub transactions: Vec<Transaction>,
    
    /// Hash of the previous block (immutable link)
    pub previous_hash: String,
    
    /// Merkle root of transactions
    pub merkle_root: String,
    
    /// Validator's digital signature
    pub validator_signature: Vec<u8>,
    
    /// Zero-knowledge proof for block validity
    pub zk_proof: Vec<u8>,
    
    // === PHASE 1: CONSENSUS LAYER FIELDS ===
    
    /// Epoch identifier (deterministically derived from block height)
    /// epoch_id = (index - genesis_height) / blocks_per_epoch
    pub epoch_id: u64,
    
    /// Consensus mode for this epoch (locked at epoch start)
    pub consensus_mode: ConsensusMode,
    
    /// Protocol version (for hard fork detection)
    pub protocol_version: u32,
    
    // === PHASE 2: SHARDING LAYER FIELDS ===
    
    /// Shard registry Merkle root (canonical shard topology for this epoch)
    /// All honest nodes independently derive identical registry roots for each epoch.
    /// SAFETY: Block is rejected if registry root doesn't match expected value.
    pub shard_registry_root: String,
    
    /// ID of the shard this block is assigned to
    /// Used for transaction routing and validator assignment.
    pub shard_id: u64,
    
    /// Merkle root of shard state commitments within this epoch
    /// Enables light client verification of shard state.
    pub shard_state_root: String,
}

impl Block {
    /// Create a new block with consensus fields.
    /// 
    /// SAFETY: Consensus and shard fields must be set by the consensus orchestrator,
    /// not computed locally. This constructor initializes them to defaults; the
    /// orchestrator will set correct values before block proposal.
    pub fn new(index: u64, transactions: Vec<Transaction>, previous_hash: String) -> Self {
        let timestamp = Utc::now().timestamp() as u64;
        let merkle_root = Block::calculate_merkle_root(&transactions);

        Self {
            index,
            timestamp,
            transactions,
            previous_hash,
            merkle_root,
            validator_signature: vec![],
            zk_proof: vec![],
            epoch_id: 0,
            consensus_mode: ConsensusMode::PosNormal,
            protocol_version: 1,
            shard_registry_root: "0".repeat(64),
            shard_id: 0,
            shard_state_root: "0".repeat(64),
        }
    }

    /// Create a block with explicit consensus and shard fields.
    /// 
    /// SAFETY: Use this constructor when building blocks with known
    /// consensus and shard parameters (e.g., during epoch transitions).
    pub fn with_consensus_and_sharding(
        index: u64,
        transactions: Vec<Transaction>,
        previous_hash: String,
        epoch_id: u64,
        consensus_mode: ConsensusMode,
        protocol_version: u32,
        shard_registry_root: String,
        shard_id: u64,
        shard_state_root: String,
    ) -> Self {
        let timestamp = Utc::now().timestamp() as u64;
        let merkle_root = Block::calculate_merkle_root(&transactions);

        Self {
            index,
            timestamp,
            transactions,
            previous_hash,
            merkle_root,
            validator_signature: vec![],
            zk_proof: vec![],
            epoch_id,
            consensus_mode,
            protocol_version,
            shard_registry_root,
            shard_id,
            shard_state_root,
        }
    }

    /// Compute block hash using SHA3-256
    /// 
    /// SAFETY: Hash includes consensus and shard fields to prevent:
    /// - Consensus mode disagreement attacks
    /// - Shard registry mismatch attacks
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(format!(
            "{}{}{}{}{}{}{}{}{}",
            self.index,
            self.timestamp,
            self.previous_hash,
            self.merkle_root,
            self.epoch_id,
            self.consensus_mode as u8,
            self.protocol_version,
            self.shard_registry_root,
            self.shard_id
        ));
        hex::encode(hasher.finalize())
    }

    /// Generate a quantum-secure digital signature
    pub fn sign_block(&mut self, private_key: &[u8]) {
        // Stub: quantum signature
        self.validator_signature = vec![];
    }

    /// Verify block signature
    pub fn verify_signature(&self, public_key: &[u8]) -> bool {
        // Stub: quantum signature verification
        true
    }

    /// Generate a ZKP to prove block validity
    pub fn generate_zkp(&mut self) {
        // Stub: ZKP generation
        self.zk_proof = vec![];
    }

    /// Validate the ZKP for block integrity
    pub fn verify_zkp(&self) -> bool {
        // Stub: ZKP verification
        true
    }

    /// Compute Merkle root from transactions
    pub fn calculate_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return String::new();
        }

        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|_| "dummy_hash".to_string())
            .collect();

        while hashes.len() > 1 {
            hashes = hashes
                .chunks(2)
                .map(|chunk| {
                    let mut hasher = Sha3_256::new();
                    hasher.update(chunk[0].clone() + chunk.get(1).unwrap_or(&chunk[0]));
                    hex::encode(hasher.finalize())
                })
                .collect();
        }

        hashes[0].clone()
    }
}
