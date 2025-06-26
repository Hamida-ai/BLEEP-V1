// src/bin/bleep_p2p.rs

use bleep-p2p::P2PNode::P2PNode;
use bleep-p2p::gossip_protocol::start_gossip_layer;
use bleep-p2p::dark_routing::start_dark_routing;
use bleep-p2p::peer_manager::PeerManager;

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
    // Step 1: Initialize peer manager and load peers
    let mut peer_manager = PeerManager::new();
    peer_manager.load_peers()?;
    info!("🧑‍🤝‍🧑 Peer manager loaded with {} peers.", peer_manager.count());

    // Step 2: Start core P2P node service
    let mut node = P2PNode::new(peer_manager);
    node.bootstrap()?;
    info!("🔗 P2P Node bootstrapped.");

    // Step 3: Launch gossip protocol
    start_gossip_layer(&mut node)?;
    info!("📢 Gossip layer running.");

    // Step 4: Start dark routing overlay
    start_dark_routing(&mut node)?;
    info!("🕶️ Dark routing enabled.");

    // Step 5: Enter P2P message loop
    node.run()?;
    info!("✅ BLEEP P2P Node operational.");

    Ok(())
}
