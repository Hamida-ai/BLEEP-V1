// src/bin/bleep_consensus.rs

use bleep_consensus::consensus::ConsensusEngine;
use bleep_core::blockchain::Blockchain;
use bleep_core::mempool::Mempool;
use bleep_core::transaction::Transaction;
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
    let mut blockchain = Blockchain::load_or_initialize()?;
    let mut mempool = Mempool::load()?;
    info!("📊 Loaded chain and mempool. {} txs pending", mempool.len());

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
