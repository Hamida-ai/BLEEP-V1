use crate::transaction::ZKTransaction;
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
    /// 
    /// SAFETY: Enforces max_size limit to prevent unbounded memory growth.
    /// Returns false if transaction is invalid or pool is at capacity.
    pub async fn add_transaction(&self, transaction: ZKTransaction) -> bool {
        let mut pool = self.pool.lock().await;
        
        // Check if pool is at capacity
        if pool.len() >= self.max_size {
            log::warn!(
                "Transaction pool at capacity ({}), rejecting transaction",
                self.max_size
            );
            return false;
        }
        
        // Basic transaction validation: check required fields
        if transaction.sender.is_empty() || transaction.receiver.is_empty() {
            log::error!("Transaction validation failed: sender or receiver is empty");
            return false;
        }
        
        if transaction.amount == 0 {
            log::error!("Transaction validation failed: amount cannot be zero");
            return false;
        }
        
        if transaction.signature.is_empty() {
            log::error!("Transaction validation failed: missing signature");
            return false;
        }
        
        if transaction.sender == transaction.receiver {
            log::error!("Transaction validation failed: sender cannot equal receiver");
            return false;
        }
        
        // Check for duplicate transaction
        let tx_hash = format!("{}:{}:{}:{}", 
            transaction.sender, transaction.receiver, transaction.amount, transaction.timestamp);
        if pool.iter().any(|tx| {
            format!("{}:{}:{}:{}", tx.sender, tx.receiver, tx.amount, tx.timestamp) == tx_hash
        }) {
            log::warn!("Duplicate transaction rejected: {}", tx_hash);
            return false;
        }
        
        // Add transaction to pool (FIFO)
        pool.push_back(transaction);
        
        log::debug!(
            "Transaction added to pool. Pool size: {}/{}",
            pool.len(),
            self.max_size
        );
        
        true
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

    /// Remove a specific transaction by its canonical ID string
    /// `"sender:receiver:amount:timestamp"` after it has been included in a block.
    pub async fn remove_confirmed(&self, tx_id: &str) {
        let mut pool = self.pool.lock().await;
        pool.retain(|tx| {
            format!("{}:{}:{}:{}", tx.sender, tx.receiver, tx.amount, tx.timestamp) != tx_id
        });
        log::debug!("Removed confirmed tx {} from pool (remaining: {})", tx_id, pool.len());
    }

    /// Drain up to `limit` transactions for block inclusion, leaving them in the pool
    /// until `remove_confirmed` is called after the block is accepted.
    pub async fn peek_for_block(&self, limit: usize) -> Vec<ZKTransaction> {
        let pool = self.pool.lock().await;
        pool.iter().take(limit).cloned().collect()
    }
