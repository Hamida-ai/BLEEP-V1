// src/bin/bleep_p2p.rs

use bleep_p2p::P2PNodeType;
use bleep_p2p::peer_manager::PeerManager;

use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("🌐 BLEEP P2P Engine Booting...");

    if let Err(e) = run_p2p_node() {
        error!("❌ P2P engine failed: {}", e);
        std::process::exit(1);
    }
}

fn run_p2p_node() -> Result<(), Box<dyn Error>> {
    // Step 1: Initialize blockchain state and peer manager
    let blockchain = std::sync::Arc::new(std::sync::Mutex::new(bleep_p2p::P2PNode::BlockchainState {}));
    let peer_manager = PeerManager::new();
    let id = "node-1".to_string();
    let addr: std::net::SocketAddr = match "127.0.0.1:9000".parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to parse socket address: {}", e);
            return Err(Box::new(e));
        }
    };

    // Step 2: Start core P2P node service
    let node = P2PNodeType::new(id, addr, blockchain.clone());
    info!("🔗 P2P Node initialized.");

    // Step 3: Broadcast message to peers
    info!("📢 Initializing message broadcast capability");
    let peer_list = peer_manager.get_active_peers();
    info!("Available peers: {}", peer_list.len());
    for peer in peer_list {
        info!("  - Peer connected: {}", peer);
    }

    Ok(())
}
