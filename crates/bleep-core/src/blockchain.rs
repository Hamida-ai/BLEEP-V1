use std::collections::{VecDeque, HashMap};
use crate::block::{Block, BlockValidator};
use crate::networking::PeerManager;
use crate::state::BlockchainState;
use crate::transactions::TransactionPool;
use std::sync::{Arc, RwLock};

pub struct Blockchain {
    pub chain: VecDeque<Block>,
    pub state: Arc<RwLock<BlockchainState>>,
    pub transaction_pool: Arc<RwLock<TransactionPool>>,
}

impl Blockchain {
    /// **Initialize blockchain with genesis block**
    pub fn new(genesis_block: Block, state: BlockchainState, tx_pool: TransactionPool) -> Self {
        let mut chain = VecDeque::new();
        chain.push_back(genesis_block);

        Self {
            chain,
            state: Arc::new(RwLock::new(state)),
            transaction_pool: Arc::new(RwLock::new(tx_pool)),
        }
    }

    /// **Validate and add a new block to the chain**
    pub fn add_block(&mut self, block: Block, public_key: &[u8]) -> bool {
        let last_block = self.chain.back().unwrap();

        // **Full Validation: Block + Transactions + Network Consensus**
        if !BlockValidator::validate_full_block(last_block, &block, public_key) {
            log::error!("Block {} failed validation.", block.index);
            return false;
        }

        // **Transaction Check: Remove included transactions from pool**
        {
            let mut tx_pool = self.transaction_pool.write().unwrap();
            tx_pool.remove_transactions(&block.transactions);
        }

        // **State Update Before Adding Block**
        {
            let mut state = self.state.write().unwrap();
            if !state.apply_block(&block) {
                log::error!("State update failed for Block {}", block.index);
                return false;
            }
        }

        self.chain.push_back(block);
        log::info!("Block {} successfully added to the blockchain.", block.index);

        // **Broadcast to network peers**
        PeerManager::broadcast_new_block(&block);

        true
    }

    /// **Verify the integrity of the entire blockchain**
    pub fn verify_chain(&self, public_key: &[u8]) -> bool {
        for i in 1..self.chain.len() {
            let prev = &self.chain[i - 1];
            let current = &self.chain[i];

            if !BlockValidator::validate_block_link(prev, current)
                || !BlockValidator::validate_block(current, public_key) {
                log::error!("Blockchain integrity check failed at Block {}.", current.index);
                return false;
            }
        }
        log::info!("Blockchain integrity verified successfully.");
        true
    }

    /// **Handle potential chain forks**
    pub fn handle_fork(&mut self, new_chain: VecDeque<Block>) {
        if new_chain.len() > self.chain.len() {
            log::warn!("Fork detected! Switching to longer valid chain.");
            self.chain = new_chain;
        }
    }

    /// **Rollback blockchain state if a block is found invalid later**
    pub fn rollback(&mut self) {
        if let Some(removed_block) = self.chain.pop_back() {
            let mut state = self.state.write().unwrap();
            state.revert_block(&removed_block);
            log::warn!("Block {} removed due to rollback.", removed_block.index);
        }
    }

    /// **Fetch a block by index**
    pub fn get_block_by_index(&self, index: u64) -> Option<&Block> {
        self.chain.iter().find(|b| b.index == index)
    }

    /// **Fetch a block by hash**
    pub fn get_block_by_hash(&self, hash: &str) -> Option<&Block> {
        self.chain.iter().find(|b| b.compute_hash() == hash)
    }
}