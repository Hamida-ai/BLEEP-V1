// src/bin/bleep_consensus.rs

use bleep_consensus::ConsensusEngine;
use bleep_core::{Blockchain, Mempool};
use bleep_core::block::Transaction;
use bleep_crypto::zkp_verification::verify_transaction_zkp;

use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("🔷 BLEEP Consensus Engine Starting...");

    if let Err(e) = run_consensus_engine() {
        error!("❌ Consensus engine failed: {}", e);
        std::process::exit(1);
    }
}

fn run_consensus_engine() -> Result<(), Box<dyn Error>> {
    // Step 1: Load blockchain and mempool
    let genesis_block = bleep_core::Block::new(0, vec![], "0".repeat(64));
    let tx_pool = bleep_core::transaction_pool::TransactionPool::new(10000);
    let mut blockchain = Blockchain::new(genesis_block, Default::default(), tx_pool.clone());
    let mempool = Mempool::new();
    info!("📊 Loaded chain and mempool.");

    // Step 2: Initialize consensus engine
    let mut engine = ConsensusEngine::new(&mut blockchain);
    info!("⚙️ Consensus engine initialized.");

    // Step 3: Filter and validate transactions
    let valid_txs: Vec<Transaction> = mempool
        .pending_transactions()
        .into_iter()
        .filter(|tx| verify_transaction_zkp(tx).unwrap_or(false))
        .collect();
    info!("🔍 Validated {} transactions via zkSNARKs", valid_txs.len());

    // Step 4: Execute consensus step
    engine.produce_block(valid_txs)?;
    info!("✅ New block produced and added to chain");

    Ok(())
}
