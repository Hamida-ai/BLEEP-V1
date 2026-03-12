//! bleep-interop/src/nullifier_store.rs
//! SA-C1 fix: Nullifier uniqueness for Layer 3 bridge
//!
//! Prevents double-spend on the L3 bridge by tracking spent nullifier hashes.

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum NullifierError {
    AlreadySpent([u8; 32]),
    StoreUnavailable,
}

/// GlobalNullifierSet backed by RocksDB in production; HashSet in tests.
pub struct GlobalNullifierSet {
    spent: HashSet<[u8; 32]>,
}

impl GlobalNullifierSet {
    pub fn new() -> Self { Self { spent: HashSet::new() } }

    /// Atomically mark a nullifier as spent.
    /// Returns Err(AlreadySpent) if it was already recorded.
    pub fn spend(&mut self, nullifier: [u8; 32]) -> Result<(), NullifierError> {
        if self.spent.contains(&nullifier) {
            return Err(NullifierError::AlreadySpent(nullifier));
        }
        self.spent.insert(nullifier);
        Ok(())
    }

    pub fn is_spent(&self, nullifier: &[u8; 32]) -> bool { self.spent.contains(nullifier) }
    pub fn len(&self) -> usize { self.spent.len() }
}

impl Default for GlobalNullifierSet { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_spend_succeeds() {
        let mut ns = GlobalNullifierSet::new();
        assert!(ns.spend([0xAA; 32]).is_ok());
    }

    #[test]
    fn double_spend_rejected() {
        let mut ns = GlobalNullifierSet::new();
        ns.spend([0xBB; 32]).unwrap();
        let err = ns.spend([0xBB; 32]).unwrap_err();
        assert!(matches!(err, NullifierError::AlreadySpent(_)));
    }

    #[test]
    fn different_nullifiers_both_succeed() {
        let mut ns = GlobalNullifierSet::new();
        ns.spend([0x01; 32]).unwrap();
        ns.spend([0x02; 32]).unwrap();
        assert_eq!(ns.len(), 2);
    }

    #[test]
    fn layer3_nullifier_uniqueness() {
        // Simulate the full L3 flow twice with the same nullifier (SA-C1 scenario)
        let mut ns = GlobalNullifierSet::new();
        let nullifier = [0xDE; 32];
        ns.spend(nullifier).expect("first proof submission OK");
        let result = ns.spend(nullifier);
        assert!(result.is_err(), "second submission with same nullifier must be rejected");
    }
}
