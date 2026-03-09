// src/bin/bleep_block.rs


use bleep_core::{Block, Blockchain, TransactionPool, Mempool};
use bleep_core::block::Transaction;

use std::error::Error;
use log::{info, error};
use chrono::Utc;

fn main() {
    env_logger::init();
    info!("🔷 BLEEP Block Module Starting...");

    if let Err(e) = run_block_module() {
        error!("❌ Block module failed: {}", e);
        std::process::exit(1);
    }
}

fn run_block_module() -> Result<(), Box<dyn Error>> {
    // Step 1: Initialize blockchain with genesis block if needed
    let genesis_block = Block::new(0, vec![], "0".repeat(64));
    let tx_pool = TransactionPool::new(10000); // Set max_size as appropriate
    let mut blockchain = Blockchain::new(genesis_block, Default::default(), tx_pool.clone());
    info!("✅ Blockchain initialized with {} blocks", blockchain.chain.len());

    // Step 2: Collect transactions from mempool (async, simplified for demo)
    let mempool = Mempool::new();
    // In real code, this would be async: mempool.get_pending_transactions().await
    let pending_txs: Vec<bleep_core::block::Transaction> = vec![]; // Placeholder for pending txs
    info!("📦 {} transactions collected from mempool", pending_txs.len());

    // Step 3: Create new block
    let last_block = match blockchain.chain.back() {
        Some(block) => block,
        None => {
            error!("Blockchain is empty. Cannot create a new block.");
            return Err(ConsensusError::EmptyBlockchain);
        }
    };
    let last_hash = last_block.compute_hash();
    let new_block = Block::new(
        last_block.index + 1,
        pending_txs,
        last_hash,
    );
    info!("📄 New block created with hash: {}", new_block.compute_hash());

    // Step 4: Validate and append block
    blockchain.add_block(new_block, &[]);
    info!("✅ New block added.");

    Ok(())
}
