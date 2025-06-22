// src/bin/main.rs

use bleep_ai::ai_assistant::start_ai_services;
use bleep_block::blockchain::initialize_blockchain;
use bleep_consensus::consensus::run_consensus_engine;
use bleep_crypto::quantum_resistance::init_crypto_layer;
use bleep_governance::governance_engine::init_governance;
use bleep_p2p::P2PNode::start_p2p_network;
use bleep_wallet_core::wallet::init_wallet_services;
use bleep_state::state_manager::start_state_services;
use bleep_telemetry::metrics::init_telemetry;
use bleep_pat::asset_token::launch_asset_token_logic;

use std::error::Error;
use log::{info, error};

fn main() {
    // Initialize logger
    env_logger::init();

    info!("Starting BLEEP Blockchain Node...");

    if let Err(e) = run() {
        error!("BLEEP Node failed to start: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Step 1: Initialize core cryptographic and ZK systems
    init_crypto_layer()?;
    info!("Quantum-safe cryptography initialized.");

    // Step 2: Load and initialize the blockchain
    initialize_blockchain()?;
    info!("Blockchain loaded and genesis verified.");

    // Step 3: Launch the P2P node network
    start_p2p_network()?;
    info!("P2P node started.");

    // Step 4: Start AI-driven services
    start_ai_services()?;
    info!("AI automation services running.");

    // Step 5: Launch wallet and asset token logic
    init_wallet_services()?;
    launch_asset_token_logic()?;
    info!("Wallet and token layers active.");

    // Step 6: Start state management and mempool
    start_state_services()?;
    info!("State services and mempool started.");

    // Step 7: Run consensus engine
    run_consensus_engine()?;
    info!("Consensus mechanism operational.");

    // Step 8: Initialize governance modules
    init_governance()?;
    info!("Governance logic activated.");

    // Step 9: Launch telemetry
    init_telemetry()?;
    info!("Telemetry and metrics collection active.");

    info!("BLEEP Blockchain Node successfully launched.");
    Ok(())
}
