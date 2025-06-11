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
    pub fn new(users: Vec<Vec<u8>>) -> Self {
        let rng = &mut ark_std::test_rng();
        let merkle_tree = MerkleTree::new(users.clone(), rng).unwrap();

        Self { merkle_tree }
    }

    pub fn generate_proof(&self, user_hash: Vec<u8>) -> IdentityProof {
        let rng = &mut ark_std::test_rng();

        let merkle_path = self.merkle_tree.generate_path(user_hash.clone()).unwrap();
        let proof = create_random_proof(&self.merkle_tree, rng).unwrap();

        IdentityProof {
            merkle_path,
            leaf_hash: user_hash,
            proof: proof.to_bytes(),
        }
    }

    pub fn verify_proof(&self, proof: &IdentityProof) -> bool {
        self.merkle_tree.verify_path(&proof.merkle_path, &proof.leaf_hash)
    }
}