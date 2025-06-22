// src/bin/bleep_core.rs

use bleep_core::blockchain::Blockchain;
use bleep_core::mempool::Mempool;
use bleep_core::transaction::Transaction;
use bleep_core::proof_of_identity::ProofOfIdentity;
use bleep_core::consensus::ConsensusEngine;
use bleep_core::storage::StorageEngine;
use bleep_core::runtime::RuntimeContext;
use bleep_core::config::CoreConfig;
use bleep_core::utils::{load_config, validate_block, sync_from_peers};

use std::sync::{Arc, Mutex};
use std::error::Error;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    info!("🚀 Starting bleep_core entry point...");

    // Load configuration
    let config: CoreConfig = load_config("config/core.toml")?;

    // Initialize shared state
    let mempool = Arc::new(Mutex::new(Mempool::new()));
    let storage = Arc::new(StorageEngine::new(&config.db_path)?);
    let proof_id = Arc::new(ProofOfIdentity::new());

    // Runtime and blockchain context
    let mut context = RuntimeContext::new(config.clone(), storage.clone(), proof_id.clone());
    let mut blockchain = Blockchain::new(config.chain_id.clone(), mempool.clone(), storage.clone());

    blockchain.load_chain()?;
    info!("✅ Blockchain loaded. Chain height: {}", blockchain.len());

    // Start consensus engine
    let mut consensus = ConsensusEngine::new(blockchain.clone(), mempool.clone(), storage.clone());
    consensus.bootstrap()?;
    info!("🧠 Consensus engine initialized.");

    // Main sync + block validation loop
    loop {
        if let Some(peer_blocks) = sync_from_peers(&config.peer_nodes).await {
            for block in peer_blocks {
                if validate_block(&block, &blockchain, &proof_id)? {
                    blockchain.append_block(block)?;
                }
            }
        }

        consensus.tick().await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
 
