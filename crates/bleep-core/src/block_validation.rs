use crate::block::Block;
use crate::crypto::{SphincsPlus, Kyber}; // Quantum-secure crypto
use crate::ai::AnomalyDetector;
use crate::networking::PeerManager;
use crate::transactions::Transaction;

pub struct BlockValidator;

impl BlockValidator {
    /// **Validate block integrity (Signature + ZKP)**
    pub fn validate_block(block: &Block, public_key: &[u8]) -> bool {
        // Check quantum-secure signature
        if !block.verify_signature(public_key) {
            log::warn!("Block {} failed signature verification!", block.index);
            return false;
        }

        // Verify Zero-Knowledge Proof (ZKP) for block validity
        if !block.verify_zkp() {
            log::warn!("Block {} failed ZKP verification!", block.index);
            return false;
        }

        true
    }

    /// **AI-based anomaly detection for malicious blocks**
    pub fn ai_validate(block: &Block) -> bool {
        let anomaly_detected = AnomalyDetector::detect_block_tampering(block);

        if anomaly_detected {
            log::error!("Anomaly detected in Block {}! Possible tampering.", block.index);
        }

        !anomaly_detected
    }

    /// **Ensure new block links correctly to the previous block**
    pub fn validate_block_link(prev_block: &Block, current_block: &Block) -> bool {
        let expected_previous_hash = prev_block.compute_hash();
        
        if current_block.previous_hash != expected_previous_hash {
            log::error!(
                "Block {} hash mismatch! Expected {}, got {}",
                current_block.index,
                expected_previous_hash,
                current_block.previous_hash
            );
            return false;
        }

        true
    }

    /// **Network-wide peer consensus verification**
    pub fn network_validate(block: &Block) -> bool {
        let consensus_result = PeerManager::broadcast_block_validation(block);

        if !consensus_result {
            log::warn!("Block {} failed peer consensus validation!", block.index);
        }

        consensus_result
    }

    /// **Full block validation pipeline**
    pub fn validate_full_block(prev_block: &Block, block: &Block, public_key: &[u8]) -> bool {
        let valid_integrity = Self::validate_block(block, public_key);
        let valid_ai = Self::ai_validate(block);
        let valid_link = Self::validate_block_link(prev_block, block);
        let valid_network = Self::network_validate(block);

        let is_valid = valid_integrity && valid_ai && valid_link && valid_network;

        if !is_valid {
            log::error!("Block {} is INVALID!", block.index);
        } else {
            log::info!("Block {} successfully validated!", block.index);
        }

        is_valid
    }
}