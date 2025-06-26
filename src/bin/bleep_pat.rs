// src/bin/bleep_pat.rs

use bleep-pat::asset_token::{AssetTokenEngine, TokenError};
use bleep-core::transaction_pool::TransactionPool;
use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("🪙 BLEEP Programmable Asset Token (PAT) Engine Launching...");

    if let Err(e) = run_pat_engine() {
        error!("❌ PAT engine failed: {}", e);
        std::process::exit(1);
    }
}

fn run_pat_engine() -> Result<(), Box<dyn Error>> {
    // Step 1: Initialize the token engine
    let mut token_engine = AssetTokenEngine::new();
    info!("✅ PAT engine initialized.");

    // Step 2: Load pending asset-related transactions
    let tx_pool = TransactionPool::load()?;
    let asset_txs = tx_pool.pending_asset_token_transactions();
    info!("📄 Loaded {} programmable asset token transactions.", asset_txs.len());

    // Step 3: Process and execute token smart contracts
    token_engine.process_transactions(asset_txs)?;
    info!("🔁 Asset token transactions processed.");

    // Step 4: Persist updated state
    token_engine.save_state()?;
    info!("💾 PAT engine state persisted.");

    Ok(())
}
