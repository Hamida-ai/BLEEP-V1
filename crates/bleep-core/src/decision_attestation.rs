/// Decision Attestation Framework
/// 
/// This module implements cryptographic signing and attestation for all protocol decisions.
/// Every decision (consensus, governance, AI advisory, state recovery) must be:
/// - Signed by authorized parties
/// - Timestamped with deterministic nonce protection
/// - Hash-committed to a merkle root
/// - Cryptographically verifiable
/// 
/// ARCHITECTURAL PRINCIPLE:
/// No decision executes without full cryptographic attestation.
/// Silent defaults and implicit trust are prohibited.

use crate::protocol_invariants::InvariantResult;
use bleep_crypto::pq_crypto::{SphincsSignatureScheme, SphincsPublicKey, SphincsSignature, HashFunctions};
use bleep_crypto::merkle_commitment::{Commitment, MerkleTree};
use serde::{Serialize, Deserialize};
use sha3::{Digest, Sha3_256};
use std::collections::BTreeMap;

// ==================== ERROR TYPES ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttestationError {
    /// Signature verification failed
    SignatureVerificationFailed(String),
    
    /// Nonce invalid or reused
    NonceViolation(String),
    
    /// Timestamp is invalid (future or too old)
    TimestampViolation(String),
    
    /// Decision is not properly attested
    MissingAttestation(String),
    
    /// Signer is not authorized
    UnauthorizedSigner(String),
    
    /// Hash commitment mismatch
    CommitmentMismatch(String),
    
    /// Decision already executed
    DuplicateDecision(String),
}

impl std::fmt::Display for AttestationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttestationError::SignatureVerificationFailed(msg) => write!(f, "Signature verification failed: {}", msg),
            AttestationError::NonceViolation(msg) => write!(f, "Nonce violation: {}", msg),
            AttestationError::TimestampViolation(msg) => write!(f, "Timestamp violation: {}", msg),
            AttestationError::MissingAttestation(msg) => write!(f, "Missing attestation: {}", msg),
            AttestationError::UnauthorizedSigner(msg) => write!(f, "Unauthorized signer: {}", msg),
            AttestationError::CommitmentMismatch(msg) => write!(f, "Commitment mismatch: {}", msg),
            AttestationError::DuplicateDecision(msg) => write!(f, "Duplicate decision: {}", msg),
        }
    }
}

impl std::error::Error for AttestationError {}

pub type AttestationResult<T> = Result<T, AttestationError>;

// ==================== CORE TYPES ====================

/// A cryptographically signed decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationSignature {
    /// The actual signature
    pub signature: SphincsSignature,
    
    /// Public key of signer
    pub signer: String,
    
    /// Timestamp (seconds since epoch)
    pub timestamp: u64,
    
    /// Nonce to prevent replay attacks
    pub nonce: Vec<u8>,
}

/// A decision that requires attestation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionType {
    /// Consensus mode change
    ConsensusModeChange {
        epoch: u64,
        new_mode: String,
    },
    
    /// Governance proposal execution
    GovernanceExecution {
        proposal_id: String,
        execution_data: Vec<u8>,
    },
    
    /// AI decision/advisory
    AIAdvisory {
        decision_id: String,
        advisory_data: Vec<u8>,
    },
    
    /// Validator slashing
    ValidatorSlashing {
        validator_id: String,
        amount: u128,
        reason: String,
    },
    
    /// Emergency state recovery
    StateRecovery {
        recovery_height: u64,
        recovery_data: Vec<u8>,
    },
}

impl DecisionType {
    /// Get deterministic decision ID
    pub fn decision_id(&self) -> String {
        let mut hasher = Sha3_256::new();
        // Serialize with error handling
        if let Ok(serialized) = bincode::serialize(self) {
            hasher.update(&serialized);
        } else {
            // Fallback to hashing the decision variant name
            hasher.update(b"decision_serialization_error");
        }
        let hash = hasher.finalize();
        hex::encode(&hash)
    }
    
    /// Serialize for signing
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_else(|_| vec![])
    }
}

/// Fully attested decision (ready for execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestedDecision {
    /// The decision itself
    pub decision: DecisionType,
    
    /// Unique decision ID
    pub decision_id: String,
    
    /// All signatures on this decision
    pub attestations: Vec<AttestationSignature>,
    
    /// Hash commitment of decision
    pub commitment: Commitment,
    
    /// Whether this decision has been executed
    pub executed: bool,
    
    /// Block height when executed (None if not executed)
    pub execution_height: Option<u64>,
}

impl AttestedDecision {
    /// Create a new attested decision
    pub fn new(decision: DecisionType, nonce: Vec<u8>) -> Self {
        let decision_id = decision.decision_id();
        let commitment = Commitment::commit(&decision.to_bytes(), &nonce);
        
        Self {
            decision,
            decision_id,
            attestations: Vec::new(),
            commitment,
            executed: false,
            execution_height: None,
        }
    }
    
    /// Add an attestation (signature) from an authorized signer
    pub fn add_attestation(&mut self, attestation: AttestationSignature) -> AttestationResult<()> {
        // Check for duplicate signatures from same signer
        if self.attestations.iter().any(|a| a.signer == attestation.signer) {
            return Err(AttestationError::DuplicateDecision(
                format!("Signer {} already attested to this decision", attestation.signer)
            ));
        }
        
        self.attestations.push(attestation);
        Ok(())
    }
    
    /// Get attestation count
    pub fn attestation_count(&self) -> usize {
        self.attestations.len()
    }
}

// ==================== ATTESTATION ENGINE ====================

/// Manages decision attestation and verification
pub struct DecisionAttestationEngine {
    /// Authorized signers and their public keys
    authorized_signers: BTreeMap<String, SphincsPublicKey>,
    
    /// Nonces that have been used (prevents replay)
    used_nonces: BTreeMap<Vec<u8>, u64>,
    
    /// Executed decisions (prevents double-execution)
    executed_decisions: BTreeMap<String, u64>,
    
    /// Time tolerance for timestamp validation (seconds)
    time_tolerance: u64,
}

impl DecisionAttestationEngine {
    /// Create a new attestation engine
    pub fn new(time_tolerance: u64) -> Self {
        Self {
            authorized_signers: BTreeMap::new(),
            used_nonces: BTreeMap::new(),
            executed_decisions: BTreeMap::new(),
            time_tolerance,
        }
    }
    
    /// Register an authorized signer
    pub fn register_signer(
        &mut self,
        signer_id: String,
        public_key: SphincsPublicKey,
    ) -> AttestationResult<()> {
        if self.authorized_signers.contains_key(&signer_id) {
            return Err(AttestationError::UnauthorizedSigner(
                format!("Signer {} already registered", signer_id)
            ));
        }
        
        self.authorized_signers.insert(signer_id, public_key);
        Ok(())
    }
    
    /// Verify a single attestation
    pub fn verify_attestation(
        &mut self,
        decision: &DecisionType,
        attestation: &AttestationSignature,
        current_time: u64,
    ) -> AttestationResult<()> {
        // 1. Check signer is authorized
        let public_key = self.authorized_signers.get(&attestation.signer)
            .ok_or_else(|| AttestationError::UnauthorizedSigner(
                format!("Signer {} not authorized", attestation.signer)
            ))?;
        
        // 2. Check nonce is fresh
        if let Some(used_at) = self.used_nonces.get(&attestation.nonce) {
            return Err(AttestationError::NonceViolation(
                format!("Nonce reuse detected: previously used at block {}", used_at)
            ));
        }
        
        // 3. Check timestamp is valid
        let time_diff = if current_time >= attestation.timestamp {
            current_time - attestation.timestamp
        } else {
            return Err(AttestationError::TimestampViolation(
                "Attestation timestamp is in the future".to_string()
            ));
        };
        
        if time_diff > self.time_tolerance {
            return Err(AttestationError::TimestampViolation(
                format!("Attestation timestamp is too old: {} seconds", time_diff)
            ));
        }
        
        // 4. Verify signature over decision
        let mut hasher = Sha3_256::new();
        hasher.update(&decision.to_bytes());
        hasher.update(&attestation.nonce);
        hasher.update(attestation.timestamp.to_le_bytes());
        let message_hash = hasher.finalize();
        
        SphincsSignatureScheme::verify(&message_hash, &attestation.signature, public_key)
            .map_err(|e| AttestationError::SignatureVerificationFailed(e.to_string()))?;
        
        // 5. Mark nonce as used
        self.used_nonces.insert(attestation.nonce.clone(), current_time);
        
        Ok(())
    }
    
    /// Verify a complete attested decision
    pub fn verify_decision(
        &mut self,
        attested_decision: &AttestedDecision,
        required_attestations: usize,
        current_time: u64,
    ) -> AttestationResult<()> {
        // 1. Check not already executed
        if attested_decision.executed {
            return Err(AttestationError::DuplicateDecision(
                format!("Decision {} already executed", attested_decision.decision_id)
            ));
        }
        
        // 2. Check sufficient attestations
        if attested_decision.attestations.len() < required_attestations {
            return Err(AttestationError::MissingAttestation(
                format!(
                    "Need {} attestations, have {}",
                    required_attestations,
                    attested_decision.attestations.len()
                )
            ));
        }
        
        // 3. Verify commitment
        attested_decision.commitment.verify(&attested_decision.decision.to_bytes())
            .map_err(|e| AttestationError::CommitmentMismatch(e.to_string()))?;
        
        // 4. Verify each attestation
        for attestation in &attested_decision.attestations {
            self.verify_attestation(&attested_decision.decision, attestation, current_time)?;
        }
        
        Ok(())
    }
    
    /// Mark a decision as executed (prevents double-execution)
    pub fn mark_executed(
        &mut self,
        decision_id: String,
        block_height: u64,
    ) -> AttestationResult<()> {
        if self.executed_decisions.contains_key(&decision_id) {
            return Err(AttestationError::DuplicateDecision(
                format!("Decision {} already executed at block {}", decision_id, 
                    self.executed_decisions[&decision_id])
            ));
        }
        
        self.executed_decisions.insert(decision_id, block_height);
        Ok(())
    }
    
    /// Check if decision has been executed
    pub fn is_executed(&self, decision_id: &str) -> bool {
        self.executed_decisions.contains_key(decision_id)
    }
    
    /// Get execution block height
    pub fn get_execution_height(&self, decision_id: &str) -> Option<u64> {
        self.executed_decisions.get(decision_id).copied()
    }
}

// ==================== DECISION BATCH COMMITMENT ====================

/// Commit to a batch of decisions for merkle proof
pub struct DecisionBatch {
    decisions: Vec<AttestedDecision>,
    merkle_tree: Option<MerkleTree>,
}

impl DecisionBatch {
    /// Create a new decision batch
    pub fn new() -> Self {
        Self {
            decisions: Vec::new(),
            merkle_tree: None,
        }
    }
    
    /// Add a decision to the batch
    pub fn add_decision(&mut self, decision: AttestedDecision) -> AttestationResult<()> {
        if self.merkle_tree.is_some() {
            return Err(AttestationError::MissingAttestation(
                "Cannot add to finalized batch".to_string()
            ));
        }
        
        self.decisions.push(decision);
        Ok(())
    }
    
    /// Finalize the batch (create merkle root)
    pub fn finalize(&mut self) -> AttestationResult<[u8; 32]> {
        if self.decisions.is_empty() {
            return Err(AttestationError::MissingAttestation(
                "Cannot finalize empty batch".to_string()
            ));
        }
        
        let mut tree = MerkleTree::new();
        
        for decision in &self.decisions {
            let decision_bytes = decision.decision.to_bytes();
            tree.add_leaf(decision_bytes)
                .map_err(|e| AttestationError::CommitmentMismatch(e.to_string()))?;
        }
        
        let root = tree.finalize()
            .map_err(|e| AttestationError::CommitmentMismatch(e.to_string()))?;
        
        self.merkle_tree = Some(tree);
        Ok(root)
    }
    
    /// Get the root hash
    pub fn root(&self) -> AttestationResult<[u8; 32]> {
        self.merkle_tree.as_ref()
            .and_then(|tree| tree.root().ok())
            .ok_or_else(|| AttestationError::MissingAttestation("Batch not finalized".to_string()))
    }
    
    /// Get number of decisions
    pub fn len(&self) -> usize {
        self.decisions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_type_serialization() {
        let decision = DecisionType::ConsensusModeChange {
            epoch: 1,
            new_mode: "PBFT".to_string(),
        };
        
        let bytes = decision.to_bytes();
        assert!(!bytes.is_empty());
        assert_eq!(decision.decision_id().len(), 64); // hex-encoded SHA3-256
    }

    #[test]
    fn test_attested_decision_creation() {
        let decision = DecisionType::GovernanceExecution {
            proposal_id: "prop_1".to_string(),
            execution_data: vec![1, 2, 3],
        };
        
        let nonce = vec![1, 2, 3, 4];
        let attested = AttestedDecision::new(decision, nonce);
        
        assert_eq!(attested.attestations.len(), 0);
        assert!(!attested.executed);
    }

    #[test]
    fn test_decision_batch_creation() {
        let mut batch = DecisionBatch::new();
        
        let decision = DecisionType::ConsensusModeChange {
            epoch: 1,
            new_mode: "PBFT".to_string(),
        };
        
        let attested = AttestedDecision::new(decision, vec![1, 2, 3]);
        batch.add_decision(attested).unwrap();
        
        assert_eq!(batch.len(), 1);
    }
}
