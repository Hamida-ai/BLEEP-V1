// src/bin/bleep_block.rs

use bleep-core::block::{Block, BlockHeader};
use bleep-core::blockchain::{Blockchain, BlockchainError};
use bleep-core::transaction::Transaction;
use bleep-core::mempool::Mempool;
use bleep-core::proof_of_identity::Identity;

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
    // Step 1: Load or initialize blockchain
    let mut blockchain = Blockchain::load_or_initialize()?;
    info!("✅ Blockchain loaded with {} blocks", blockchain.len());

    // Step 2: Collect transactions from mempool
    let mempool = Mempool::load()?;
    let pending_txs: Vec<Transaction> = mempool.pending_transactions();
    info!("📦 {} transactions collected from mempool", pending_txs.len());

    // Step 3: Create block header and new block
    let last_hash = blockchain.latest_block_hash();
    let header = BlockHeader::new(
        last_hash,
        Utc::now().timestamp() as u64,
        Identity::current_node_identity_hash()
    );

    let new_block = Block::new(header, pending_txs);
    info!("📄 New block created with hash: {}", new_block.hash());

    // Step 4: Validate and append block
    blockchain.validate_and_add_block(new_block)?;
    blockchain.persist()?;
    info!("✅ New block added and chain persisted.");

    Ok(())
}
