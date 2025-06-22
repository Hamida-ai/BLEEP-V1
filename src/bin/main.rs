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

    info!("🔷 BLEEP Blockchain Node Initialization Started");

    if let Err(e) = run() {
        error!("❌ BLEEP Node failed to start: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Step 1: Initialize post-quantum cryptography and zkSNARK verification systems
    info!("🔐 Initializing cryptography layer...");
    init_crypto_layer()?;
    info!("✅ Quantum-safe cryptography initialized.");

    // Step 2: Set up and verify blockchain state and genesis block
    info!("⛓️ Loading blockchain and verifying genesis...");
    initialize_blockchain()?;
    info!("✅ Blockchain initialized.");

    // Step 3: Launch peer-to-peer gossip and dark routing
    info!("🌐 Starting peer-to-peer networking...");
    start_p2p_network()?;
    info!("✅ P2P network operational.");

    // Step 4: Enable AI smart contract assistants and automation
    info!("🧠 Activating AI automation services...");
    start_ai_services()?;
    info!("✅ AI services running.");

    // Step 5: Set up user wallet and asset token smart contracts
    info!("💼 Initializing wallet services and programmable asset tokens...");
    init_wallet_services()?;
    launch_asset_token_logic()?;
    info!("✅ Wallet and token infrastructure initialized.");

    // Step 6: Enable blockchain state management and mempool
    info!("📦 Launching state management engine...");
    start_state_services()?;
    info!("✅ State engine active.");

    // Step 7: Execute adaptive consensus protocol
    info!("⚖️ Running consensus engine...");
    run_consensus_engine()?;
    info!("✅ Consensus operational.");

    // Step 8: Launch on-chain/off-chain governance system
    info!("🏛️ Initializing governance protocols...");
    init_governance()?;
    info!("✅ Governance system online.");

    // Step 9: Begin telemetry and performance monitoring
    info!("📊 Starting telemetry services...");
    init_telemetry()?;
    info!("✅ Telemetry enabled.");

    info!("🚀 BLEEP Blockchain Node launched successfully.");
    Ok(())
}
