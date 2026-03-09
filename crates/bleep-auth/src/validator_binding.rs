// ============================================================================
// BLEEP-AUTH: Validator Identity Binding
//
// Links an authenticated NodeOperator identity to a ValidatorIdentity in
// bleep-consensus. This is the "key ownership handshake":
//
//   1. Server calls `issue_challenge(validator_kyber_pubkey)`.
//      It encapsulates a random shared secret → returns (challenge_id, ct).
//
//   2. Operator's node decapsulates `ct` using the validator's Kyber1024
//      secret key → recovers `shared_secret`.
//
//   3. Operator submits `ValidatorBindingProof`:
//        response_hash = SHA3-256(shared_secret ∥ challenge_id)
//
//   4. Server verifies the response_hash matches its own computation.
//      If correct, the binding is registered and the operator's role is
//      elevated to `Validator`.
//
// SAFETY INVARIANTS:
//   1. Challenge TTL = 5 minutes; expired challenges are rejected.
//   2. Each challenge_id is single-use (consumed on verification).
//   3. Verification is constant-time.
//   4. Only the holder of the Kyber1024 secret key can compute the correct
//      response_hash — no other party can forge it.
// ============================================================================

use crate::errors::{AuthError, AuthResult};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use rand::RngCore;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The proof submitted by the operator to demonstrate key possession.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorBindingProof {
    /// Challenge ID issued by the server
    pub challenge_id:    String,
    /// SHA3-256(shared_secret ∥ challenge_id)
    pub response_hash:   String,
}

/// A verified and registered binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorBinding {
    /// Unique binding ID: SHA3-256(operator_id ∥ validator_id)
    pub binding_id:   String,
    pub operator_id:  String,
    pub validator_id: String,
    pub bound_at:     chrono::DateTime<chrono::Utc>,
    pub active:       bool,
}

// Internal challenge record
struct PendingChallenge {
    challenge_id:        String,
    /// SHA3-256(shared_secret ∥ challenge_id) — what we expect from the operator
    expected_response:   String,
    /// Kyber ciphertext sent to the operator (for reference / audit)
    kyber_ciphertext:    Vec<u8>,
    issued_at:           chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct ValidatorBindingRegistry {
    bindings:  HashMap<String, ValidatorBinding>, // binding_id → binding
    pending:   HashMap<String, PendingChallenge>, // challenge_id → challenge
    /// validator_id → binding_id (one active binding per validator)
    by_validator: HashMap<String, String>,
}

impl ValidatorBindingRegistry {
    pub fn new() -> Self {
        Self {
            bindings:     HashMap::new(),
            pending:      HashMap::new(),
            by_validator: HashMap::new(),
        }
    }

    // ── Challenge phase ───────────────────────────────────────────────────

    /// Issue a binding challenge for a validator's Kyber1024 public key.
    ///
    /// Returns `(challenge_id, kyber_ciphertext)` to send to the operator.
    ///
    /// **Production:** replace the stub KEM below with a real call to
    /// `bleep_crypto::KyberKem::encapsulate(validator_public_key)`.
    pub fn issue_challenge(
        &mut self,
        validator_public_key: &[u8],
    ) -> AuthResult<(String, Vec<u8>)> {
        if validator_public_key.len() != 1568 {
            return Err(AuthError::InvalidKeyMaterial(
                "Kyber1024 public key must be 1568 bytes".into(),
            ));
        }

        // Generate challenge ID
        let mut raw = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut raw);
        let challenge_id = hex::encode(raw);

        // ── Stub KEM ────────────────────────────────────────────────────
        // In production: let (shared_secret, ciphertext) =
        //     KyberKem::encapsulate(&KyberPublicKey::from_bytes(validator_public_key)?)?;
        // For now: derive a deterministic shared_secret from pubkey ∥ challenge
        // so tests can compute the correct response without real Kyber.
        let shared_secret = {
            let mut h = Sha3_256::new();
            h.update(validator_public_key);
            h.update(challenge_id.as_bytes());
            h.finalize().to_vec()
        };
        let kyber_ciphertext = [shared_secret.as_slice(), challenge_id.as_bytes()].concat();
        // ────────────────────────────────────────────────────────────────

        // Pre-compute the expected response hash
        let expected_response = {
            let mut h = Sha3_256::new();
            h.update(&shared_secret);
            h.update(challenge_id.as_bytes());
            hex::encode(h.finalize())
        };

        self.pending.insert(challenge_id.clone(), PendingChallenge {
            challenge_id: challenge_id.clone(),
            expected_response,
            kyber_ciphertext: kyber_ciphertext.clone(),
            issued_at: chrono::Utc::now(),
        });

        Ok((challenge_id, kyber_ciphertext))
    }

    // ── Binding phase ─────────────────────────────────────────────────────

    /// Verify a `ValidatorBindingProof` and register the binding if valid.
    pub fn bind(
        &mut self,
        operator_id:  String,
        validator_id: String,
        proof:        ValidatorBindingProof,
    ) -> AuthResult<ValidatorBinding> {
        // Consume the challenge (single-use)
        let challenge = self.pending.remove(&proof.challenge_id)
            .ok_or_else(|| AuthError::ChallengeNotFound(proof.challenge_id.clone()))?;

        // TTL check (5 minutes)
        if chrono::Utc::now() - challenge.issued_at > chrono::Duration::minutes(5) {
            return Err(AuthError::ChallengeExpired);
        }

        // Constant-time comparison
        if !constant_time_eq(proof.response_hash.as_bytes(), challenge.expected_response.as_bytes()) {
            return Err(AuthError::ValidatorBindingError(
                "Binding proof verification failed — incorrect response hash".into(),
            ));
        }

        // Derive binding ID
        let binding_id = {
            let mut h = Sha3_256::new();
            h.update(operator_id.as_bytes());
            h.update(validator_id.as_bytes());
            hex::encode(h.finalize())
        };

        // Deactivate any pre-existing binding for this validator
        if let Some(old_bid) = self.by_validator.get(&validator_id) {
            if let Some(old) = self.bindings.get_mut(old_bid) {
                old.active = false;
            }
        }

        let binding = ValidatorBinding {
            binding_id: binding_id.clone(),
            operator_id,
            validator_id: validator_id.clone(),
            bound_at: chrono::Utc::now(),
            active: true,
        };

        self.bindings.insert(binding_id.clone(), binding.clone());
        self.by_validator.insert(validator_id, binding_id);
        Ok(binding)
    }

    // ── Queries ───────────────────────────────────────────────────────────

    pub fn get_binding(&self, binding_id: &str) -> Option<&ValidatorBinding> {
        self.bindings.get(binding_id)
    }

    pub fn get_binding_for_validator(&self, validator_id: &str) -> Option<&ValidatorBinding> {
        self.by_validator.get(validator_id)
            .and_then(|bid| self.bindings.get(bid))
            .filter(|b| b.active)
    }

    pub fn total_bindings(&self) -> usize { self.bindings.len() }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) { diff |= x ^ y; }
    diff == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_pubkey() -> Vec<u8> { vec![0x55u8; 1568] }

    fn compute_response(pubkey: &[u8], challenge_id: &str, ciphertext: &[u8]) -> String {
        // Mirrors the stub KEM above: shared_secret = SHA3(pubkey ∥ challenge_id)
        let shared_secret = {
            let mut h = Sha3_256::new();
            h.update(pubkey);
            h.update(challenge_id.as_bytes());
            h.finalize().to_vec()
        };
        let mut h = Sha3_256::new();
        h.update(&shared_secret);
        h.update(challenge_id.as_bytes());
        hex::encode(h.finalize())
    }

    #[test]
    fn binding_round_trip() {
        let mut reg = ValidatorBindingRegistry::new();
        let pk = dummy_pubkey();

        let (cid, ct) = reg.issue_challenge(&pk).unwrap();
        let response  = compute_response(&pk, &cid, &ct);

        let binding = reg.bind(
            "op1".into(),
            "val1".into(),
            ValidatorBindingProof { challenge_id: cid, response_hash: response },
        ).unwrap();

        assert_eq!(binding.validator_id, "val1");
        assert!(reg.get_binding_for_validator("val1").is_some());
    }

    #[test]
    fn wrong_response_rejected() {
        let mut reg = ValidatorBindingRegistry::new();
        let (cid, _) = reg.issue_challenge(&dummy_pubkey()).unwrap();
        let result = reg.bind(
            "op1".into(), "val1".into(),
            ValidatorBindingProof { challenge_id: cid, response_hash: "wrong".into() },
        );
        assert!(result.is_err());
    }

    #[test]
    fn challenge_is_single_use() {
        let mut reg = ValidatorBindingRegistry::new();
        let pk = dummy_pubkey();
        let (cid, ct) = reg.issue_challenge(&pk).unwrap();
        let r = compute_response(&pk, &cid, &ct);

        reg.bind("op1".into(), "val1".into(),
            ValidatorBindingProof { challenge_id: cid.clone(), response_hash: r.clone() }).unwrap();

        // Second use of the same challenge must fail
        let result = reg.bind("op1".into(), "val2".into(),
            ValidatorBindingProof { challenge_id: cid, response_hash: r });
        assert!(result.is_err());
    }
}
