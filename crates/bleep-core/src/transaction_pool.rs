use crate::core::transaction::ZKTransaction;
use crate::crypto::proof_of_identity::ProofOfIdentity;
use crate::networking::encryption::QuantumEncryption;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use std::sync::Arc;

/// A high-performance transaction pool that stores recent transactions efficiently
pub struct TransactionPool {
    pool: Mutex<VecDeque<ZKTransaction>>,  // FIFO structure for transactions
    max_size: usize,                       // Maximum pool size to prevent overflows
}

impl TransactionPool {
    /// Initializes a new transaction pool with a defined max size
    pub fn new(max_size: usize) -> Arc<Self> {
        Arc::new(Self {
            pool: Mutex::new(VecDeque::with_capacity(max_size)),
            max_size,
        })
    }

    /// Adds a transaction while ensuring pool size constraints
    pub async fn add_transaction(&self, transaction: ZKTransaction) -> bool {
        let mut pool = self.pool.lock().await;
        
        // Ensure transaction validity before adding
        if transaction.verify(&QuantumEncryption::get_public_key()) {
            if pool.len() >= self.max_size {
                pool.pop_front(); // Remove oldest transaction if at capacity
            }
            pool.push_back(transaction);
            return true;
        }
        
        false
    }

    /// Retrieves all transactions from the pool
    pub async fn get_transactions(&self) -> Vec<ZKTransaction> {
        let pool = self.pool.lock().await;
        pool.iter().cloned().collect()
    }

    /// Clears all transactions from the pool (e.g., after block finalization)
    pub async fn clear_pool(&self) {
        let mut pool = self.pool.lock().await;
        pool.clear();
    }

    /// Gets the current pool size
    pub async fn pool_size(&self) -> usize {
        let pool = self.pool.lock().await;
        pool.len()
    }
}