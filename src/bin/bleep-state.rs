// src/bin/bleep_state.rs

use bleep_state::state_manager::{StateManager, StateError};
use bleep_core::transaction_pool::TransactionPool;
use bleep_core::blockchain::Blockchain;

use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("📦 BLEEP State Engine Initializing...");

    if let Err(e) = run_state_engine() {
        error!("❌ State engine failed: {}", e);
        std::process::exit(1);
    }
}

fn run_state_engine() -> Result<(), Box<dyn Error>> {
    // Step 1: Load blockchain and transaction pool
    let blockchain = Blockchain::load_or_initialize()?;
    let tx_pool = TransactionPool::load()?;
    info!("🔄 Loaded blockchain and transaction pool.");

    // Step 2: Initialize and sync state manager
    let mut state = StateManager::new(blockchain);
    state.sync_with_transactions(tx_pool.pending_transactions())?;
    info!("✅ State synchronized with mempool transactions.");

    // Step 3: Persist state snapshot
    state.save_snapshot()?;
    info!("💾 State snapshot saved.");

    Ok(())
}
