use std::sync::{Arc, Mutex};
use rand::seq::SliceRandom;
use tokio::sync::mpsc;
use crate::message_protocol::{MessageProtocol, SecureMessage};
use crate::peer_manager::{PeerManager, Peer};
use crate::quantum_crypto::{Kyber, SphincsPlus};
use crate::ai_security::PeerScoring;

const MAX_HOPS: usize = 5;

/// Multi-Hop Routing with Quantum-Secure Encryption & AI Peer Selection
#[derive(Debug)]
pub struct MultiHopRouting {
    peer_manager: Arc<PeerManager>,
    message_protocol: MessageProtocol,
    peer_scoring: PeerScoring,
}

impl MultiHopRouting {
    /// Initializes Multi-Hop Routing with AI-driven peer selection
    pub fn new(peer_manager: Arc<PeerManager>, message_protocol: MessageProtocol) -> Self {
        Self {
            peer_manager,
            message_protocol,
            peer_scoring: PeerScoring::new(),
        }
    }

    /// Selects AI-ranked relay nodes for multi-hop transmission
    fn select_relay_nodes(&self, sender_id: &str) -> Vec<String> {
        let peers: Vec<Peer> = self.peer_manager.get_peers();
        let mut peer_list: Vec<String> = peers
            .into_iter()
            .map(|p| p.id)
            .filter(|p| p != sender_id)
            .collect();

        // AI-Based Reputation Filtering: Prioritize High-Quality Nodes
        let ranked_peers = self.peer_scoring.rank_peers(peer_list.clone());
        let mut secure_relay_nodes = ranked_peers.iter().take(MAX_HOPS).cloned().collect::<Vec<_>>();
        
        // Randomize the order to prevent traceability
        secure_relay_nodes.shuffle(&mut rand::thread_rng());
        secure_relay_nodes
    }

    /// Encrypts and forwards the message through multi-hop relays
    pub async fn relay_message(&self, mut message: SecureMessage) {
        let relay_nodes = self.select_relay_nodes(&message.sender_id);

        for (i, relay) in relay_nodes.iter().enumerate() {
            if let Some(relay_addr) = self.peer_manager.get_peer_address(relay) {
                message.payload = Self::encrypt_message(&message.payload, relay);
                message.hop_count = i + 1;
                self.message_protocol.send_message(relay_addr, message.clone()).await;
            }
        }
    }

    /// Encrypts the message using **Quantum-Secure Kyber + SPHINCS+**
    fn encrypt_message(payload: &[u8], recipient: &str) -> Vec<u8> {
        let encrypted_payload = Kyber::encrypt(payload, recipient);
        SphincsPlus::sign(&encrypted_payload)
    }

    /// Handles incoming relayed messages
    pub async fn handle_relayed_message(&self, message: SecureMessage, sender: String) {
        if message.hop_count < MAX_HOPS {
            self.relay_message(message).await;
        } else {
            // Final recipient processes the message
            println!("Final destination reached: {:?}", message);
        }
    }
}