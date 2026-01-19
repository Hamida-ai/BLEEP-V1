use ark_crypto_primitives::merkle_tree::{MerkleTree, Path};
use ark_bls12_381::Bls12_381;
use ark_ff::Field;
use ark_std::rand::Rng;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityProof {
    pub merkle_path: Path<Bls12_381>,
    pub leaf_hash: Vec<u8>,
    pub proof: Vec<u8>, // zk-SNARK proof
}

pub struct ProofOfIdentity {
    merkle_tree: MerkleTree<Bls12_381>,
}

impl ProofOfIdentity {
    /// Create a new identity proof system
    /// Returns error if merkle tree construction fails
    pub fn new(users: Vec<Vec<u8>>) -> Result<Self, String> {
        let rng = &mut ark_std::test_rng();
        let merkle_tree = MerkleTree::new(users.clone(), rng)
            .map_err(|e| format!("Merkle tree construction failed: {:?}", e))?;

        Ok(Self { merkle_tree })
    }

    /// Generate a proof for a user
    /// Returns error if path generation or proof construction fails
    pub fn generate_proof(&self, user_hash: Vec<u8>) -> Result<IdentityProof, String> {
        let rng = &mut ark_std::test_rng();

        let merkle_path = self.merkle_tree.generate_path(user_hash.clone())
            .map_err(|e| format!("Merkle path generation failed: {:?}", e))?;
        let proof = create_random_proof(&self.merkle_tree, rng)
            .map_err(|e| format!("Proof creation failed: {:?}", e))?;

        Ok(IdentityProof {
            merkle_path,
            leaf_hash: user_hash,
            proof: proof.to_bytes(),
        })
    }

    pub fn verify_proof(&self, proof: &IdentityProof) -> bool {
        self.merkle_tree.verify_path(&proof.merkle_path, &proof.leaf_hash)
    }
}