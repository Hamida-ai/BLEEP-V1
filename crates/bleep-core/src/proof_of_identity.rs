use ark_bls12_381::{Bls12_381, Fr};
use ark_crypto_primitives::{
    crh::{pedersen, CRH},
    merkle_tree::{Config, MerkleTreePath},
    CRHGadget, PathVar,
};
use ark_ff::{Field, ToConstraintField, UniformRand};
use ark_groth16::{
    create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof,
    Proof, ProvingKey, VerifyingKey,
};
use ark_r1cs_std::{
    alloc::AllocVar,
    eq::EqGadget,
    fields::fp::FpVar,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_std::{rand::Rng, test_rng};
use serde::{Deserialize, Serialize};

/// Constraint system for proving Merkle membership
#[derive(Clone)]
pub struct IdentityMerkleCircuit {
    pub leaf: Fr,
    pub root: Fr,
    pub path: MerkleTreePath<Fr>,
}

impl ConstraintSynthesizer<Fr> for IdentityMerkleCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        type H = pedersen::CRH<Fr>;
        type HG = pedersen::CRHGadget<Fr>;

        // Allocate inputs
        let root_var = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.root))?;

        // Allocate witnesses
        let leaf_var = FpVar::<Fr>::new_witness(cs.clone(), || Ok(self.leaf))?;
        let path_var = PathVar::<Fr, HG>::new_witness(cs.clone(), || Ok(self.path.clone()))?;

        let parameters = H::setup(&mut test_rng())?;
        let parameters_var = HG::ParametersVar::new_constant(cs, &parameters)?;

        let computed_root = path_var.root_hash(&parameters_var, &leaf_var)?;
        computed_root.enforce_equal(&root_var)?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdentityProof {
    pub proof: Proof<Bls12_381>,
    pub root: Fr,
    pub leaf: Fr,
}

pub struct ProofOfIdentity {
    pub proving_key: ProvingKey<Bls12_381>,
    pub verifying_key: VerifyingKey<Bls12_381>,
}

impl ProofOfIdentity {
    /// Trusted setup for the identity circuit
    pub fn setup(depth: usize) -> Self {
        let dummy_leaf = Fr::rand(&mut test_rng());
        let dummy_path = MerkleTreePath {
            leaf_index: 0,
            auth_path: vec![(Fr::rand(&mut test_rng()), false); depth],
        };
        let dummy_root = Fr::rand(&mut test_rng());

        let circuit = IdentityMerkleCircuit {
            leaf: dummy_leaf,
            root: dummy_root,
            path: dummy_path,
        };

        let params =
            generate_random_parameters::<Bls12_381, _, _>(circuit, &mut test_rng()).unwrap();
        Self {
            proving_key: params.pk,
            verifying_key: params.vk,
        }
    }

    /// Prove Merkle membership for an identity leaf
    pub fn generate_proof(
        &self,
        leaf: Fr,
        root: Fr,
        path: MerkleTreePath<Fr>,
    ) -> IdentityProof {
        let circuit = IdentityMerkleCircuit { leaf, root, path };
        let proof =
            create_random_proof(circuit, &self.proving_key, &mut test_rng()).unwrap();
        IdentityProof { proof, root, leaf }
    }

    /// Verify the identity proof
    pub fn verify_proof(&self, identity_proof: &IdentityProof) -> bool {
        let pvk = prepare_verifying_key(&self.verifying_key);
        let public_inputs = identity_proof
            .root
            .to_constraint_field::<Fr>();
        verify_proof(&pvk, &identity_proof.proof, &public_inputs).unwrap_or(false)
    }
  }
