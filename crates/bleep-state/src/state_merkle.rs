use blake3::Hasher; // Replaces SHA-256 for efficiency
use serde::{Deserialize, Serialize};
use crate::crypto::sphincs::verify_merkle_proof; // SPHINCS+ for quantum-secure verification

/// **Merkle Tree Node**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MerkleNode {
    pub hash: String,
}

/// **Optimized Merkle Tree Structure**
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MerkleTree {
    pub root: String,
    pub leaves: Vec<MerkleNode>,
}

impl MerkleTree {
    /// **Constructs a new Merkle Tree from data**
    pub fn new<T: AsRef<[u8]>>(data: &[T]) -> Self {
        if data.is_empty() {
            return MerkleTree { root: String::new(), leaves: vec![] };
        }

        let mut hashes: Vec<String> = data
            .iter()
            .map(|item: &T| {
                let mut hasher = Hasher::new();
                hasher.update(item.as_ref());
                hex::encode(hasher.finalize().as_bytes())
            })
            .collect();

        while hashes.len() > 1 {
            hashes = hashes
                .chunks(2)
                .map(|chunk| {
                    let left = &chunk[0];
                    let right = if chunk.len() > 1 { &chunk[1] } else { left };
                    let mut hasher = Hasher::new();
                    hasher.update(left.as_bytes());
                    hasher.update(right.as_bytes());
                    hex::encode(hasher.finalize().as_bytes())
                })
                .collect();
        }

        MerkleTree {
            root: hashes[0].clone(),
            leaves: data
                .iter()
                .map(|d| MerkleNode { hash: hex::encode(blake3::hash(d.as_ref()).as_bytes()) })
                .collect(),
        }
    }

    /// **Verifies a Merkle Proof using SPHINCS+ (Quantum-Secure)**
    pub fn verify_merkle_proof(&self, proof: &[String], target_hash: &String) -> bool {
        // Convert root, proof, and target_hash to Vec<u8>
        let root_bytes = self.root.as_bytes();
        let proof_bytes: Vec<u8> = proof.join("").as_bytes().to_vec();
        let target_bytes = target_hash.as_bytes();
        verify_merkle_proof(root_bytes, &proof_bytes, target_bytes)
    }

    /// **Retrieves the root hash of the Merkle Tree**
    pub fn get_root(&self) -> String {
        self.root.clone()
    }
}

/// **Compute the Merkle root directly from raw data**
pub fn calculate_merkle_root<T: AsRef<[u8]>>(data: &[T]) -> String {
    MerkleTree::new(data).root
}
    // The following duplicate definition is removed