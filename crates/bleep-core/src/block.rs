use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};
use crate::crypto::{SphincsPlus, Kyber}; // Quantum-secure crypto
use crate::transactions::Transaction;
use crate::ai::AnomalyDetector; // AI-based block security
use chrono::Utc;

/// Core block structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub merkle_root: String,
    pub validator_signature: Vec<u8>,
    pub zk_proof: Vec<u8>,
}

impl Block {
    /// Create a new block
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
        }
    }

    /// Compute block hash using SHA3-256
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update(format!(
            "{}{}{}{}",
            self.index, self.timestamp, self.previous_hash, self.merkle_root
        ));
        hex::encode(hasher.finalize())
    }

    /// Generate a quantum-secure digital signature
    pub fn sign_block(&mut self, private_key: &[u8]) {
        self.validator_signature = SphincsPlus::sign(private_key, &self.compute_hash().as_bytes());
    }

    /// Verify block signature
    pub fn verify_signature(&self, public_key: &[u8]) -> bool {
        SphincsPlus::verify(public_key, &self.compute_hash().as_bytes(), &self.validator_signature)
    }

    /// Generate a ZKP to prove block validity
    pub fn generate_zkp(&mut self) {
        self.zk_proof = Kyber::generate_zkp(&self.compute_hash().as_bytes());
    }

    /// Validate the ZKP for block integrity
    pub fn verify_zkp(&self) -> bool {
        Kyber::verify_zkp(&self.zk_proof, &self.compute_hash().as_bytes())
    }

    /// Compute Merkle root from transactions
    pub fn calculate_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return String::new();
        }

        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| hex::encode(Sha3_256::digest(&bincode::serialize(tx).unwrap())))
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