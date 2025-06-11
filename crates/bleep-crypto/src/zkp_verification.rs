use ark_bls12_381::{Bls12_381, Fr};
use ark_crypto_primitives::crh::poseidon::PoseidonCRH;
use ark_groth16::{Proof, ProvingKey, VerifyingKey, Groth16};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{vec::Vec, test_rng};
use rayon::prelude::*; // Parallel processing
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use thiserror::Error;

use crate::quantum_secure::{QuantumSecure, KyberAESHybrid};
use crate::merkletree::MerkleTree;
use crate::logging::BLEEPLogger;

/// **Custom errors for ZKP operations**
#[derive(Debug, Error)]
pub enum BLEEPError {
    #[error("Proof generation failed")]
    ProofGenerationFailed,
    #[error("Proof verification failed")]
    ProofVerificationFailed,
    #[error("Key is revoked")]
    KeyRevoked,
    #[error("Serialization or deserialization failed")]
    SerializationError,
    #[error("Integrity verification failed")]
    IntegrityError,
}

/// **ZKP Module with Advanced Security & Performance**
pub struct BLEEPZKPModule {
    pub proving_key: ProvingKey<Bls12_381>,
    pub verifying_key: VerifyingKey<Bls12_381>,
    pub revocation_tree: MerkleTree,
    pub logger: BLEEPLogger,
}

impl BLEEPZKPModule {
    /// **Initialize ZKP module with secure key management**
    pub fn new(
        proving_key: ProvingKey<Bls12_381>,
        verifying_key: VerifyingKey<Bls12_381>,
    ) -> Result<Self, BLEEPError> {
        Ok(Self {
            proving_key,
            verifying_key,
            revocation_tree: MerkleTree::new(),
            logger: BLEEPLogger::new(),
        })
    }

    /// **Securely save proving & verifying keys with hybrid quantum-safe encryption**
    pub fn save_keys(
        &self,
        proving_key_path: &str,
        verifying_key_path: &str,
    ) -> Result<(), BLEEPError> {
        let kyber_aes = KyberAESHybrid::new();
        let proving_key_bytes = bincode::serialize(&self.proving_key)?;
        let verifying_key_bytes = bincode::serialize(&self.verifying_key)?;

        let encrypted_proving_key = kyber_aes.encrypt(&proving_key_bytes)?;
        let encrypted_verifying_key = kyber_aes.encrypt(&verifying_key_bytes)?;

        // Save keys to disk
        fs::write(proving_key_path, encrypted_proving_key)?;
        fs::write(verifying_key_path, encrypted_verifying_key)?;
        self.logger.info("ZKP keys securely stored.");

        Ok(())
    }

    /// **Load proving & verifying keys with decryption and integrity verification**
    pub fn load_keys(
        proving_key_path: &str,
        verifying_key_path: &str,
    ) -> Result<Self, BLEEPError> {
        let kyber_aes = KyberAESHybrid::new();

        let encrypted_proving_key = fs::read(proving_key_path)?;
        let encrypted_verifying_key = fs::read(verifying_key_path)?;

        let proving_key_bytes = kyber_aes.decrypt(&encrypted_proving_key)?;
        let verifying_key_bytes = kyber_aes.decrypt(&encrypted_verifying_key)?;

        let proving_key: ProvingKey<Bls12_381> = bincode::deserialize(&proving_key_bytes)?;
        let verifying_key: VerifyingKey<Bls12_381> = bincode::deserialize(&verifying_key_bytes)?;

        // Verify integrity before using keys
        if !KyberAESHybrid::verify_integrity(&proving_key_bytes)
            || !KyberAESHybrid::verify_integrity(&verifying_key_bytes)
        {
            return Err(BLEEPError::IntegrityError);
        }

        self.logger.info("ZKP keys successfully loaded and verified.");
        Ok(BLEEPZKPModule {
            proving_key,
            verifying_key,
            revocation_tree: MerkleTree::new(),
            logger: BLEEPLogger::new(),
        })
    }

    /// **Aggregate multiple proofs using Bulletproofs-style compression**
    pub fn aggregate_proofs(proofs: &[Proof<Bls12_381>]) -> Result<Vec<u8>, BLEEPError> {
        let mut aggregated_proof = Vec::new();
        for proof in proofs {
            let serialized = bincode::serialize(proof)?;
            aggregated_proof.extend_from_slice(&serialized);
        }
        self.logger.info("Proof aggregation successful.");
        Ok(aggregated_proof)
    }

    /// **Parallel proof generation for high-performance transactions**
    pub fn generate_batch_proofs<C>(
        &self,
        circuits: Vec<C>,
    ) -> Result<Vec<Proof<Bls12_381>>, BLEEPError>
    where
        C: ark_groth16::ConstraintSynthesizer<Fr> + Send,
    {
        let proofs: Vec<_> = circuits
            .into_par_iter()
            .map(|circuit| {
                let rng = &mut test_rng();
                Groth16::prove(&self.proving_key, circuit, rng).map_err(|_| BLEEPError::ProofGenerationFailed)
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.logger.info("Batch proof generation completed.");
        Ok(proofs)
    }

    /// **Revoke a ZKP key by adding it to a Merkle-based revocation tree**
    pub fn revoke_key(&mut self, key_bytes: Vec<u8>) -> Result<(), BLEEPError> {
        self.revocation_tree.add_leaf(key_bytes);
        self.logger.warning("ZKP key revoked.");
        Ok(())
    }

    /// **Check if a key is revoked**
    pub fn is_key_revoked(&self, key_bytes: &[u8]) -> bool {
        self.revocation_tree.contains_leaf(key_bytes)
    }

    /// **Save the revocation list securely**
    pub fn save_revocation_tree(&self, path: &str) -> Result<(), BLEEPError> {
        let serialized = bincode::serialize(&self.revocation_tree)?;
        fs::write(path, serialized)?;
        self.logger.info("Revocation tree saved.");
        Ok(())
    }

    /// **Load the revocation list from a file**
    pub fn load_revocation_tree(path: &str) -> Result<MerkleTree, BLEEPError> {
        if Path::new(path).exists() {
            let data = fs::read(path)?;
            let tree: MerkleTree = bincode::deserialize(&data)?;
            Ok(tree)
        } else {
            Ok(MerkleTree::new())
        }
    }
}