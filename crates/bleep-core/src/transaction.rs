use crate::crypto::proof_of_identity::ProofOfIdentity;
use crate::networking::encryption::QuantumEncryption;
use crate::p2p::gossip_protocol::GossipProtocol;
use crate::p2p::multi_hop_routing::MultiHopRouting;
use crate::p2p::dark_routing::DarkRouting;
use crate::p2p::peer_manager::PeerManager;
use crate::p2p::message_protocol::{P2PMessage, SecureMessage};
use serde::{Serialize, Deserialize};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents a Zero-Knowledge Proof (ZKP)-based transaction
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZKTransaction {
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl ZKTransaction {
    /// Creates a new ZKP transaction and signs it with quantum encryption
    pub fn new(sender: &str, receiver: &str, amount: u64, private_key: &[u8]) -> Self {
        let timestamp = Utc::now().timestamp() as u64;
        let data = format!("{}{}{}{}", sender, receiver, amount, timestamp);
        let signature = QuantumEncryption::sign_data(&data, private_key);

        Self {
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            amount,
            timestamp,
            signature,
        }
    }

    /// Verifies transaction validity using quantum-safe signatures
    pub fn verify(&self, public_key: &[u8]) -> bool {
        let data = format!("{}{}{}{}", self.sender, self.receiver, self.amount, self.timestamp);
        QuantumEncryption::verify_signature(&data, &self.signature, public_key)
    }
}

/// Manages transaction broadcasting and validation over P2P
pub struct TransactionManager {
    peer_manager: Arc<PeerManager>,
    gossip_protocol: Arc<GossipProtocol>,
    multi_hop_routing: Arc<MultiHopRouting>,
    dark_routing: Arc<DarkRouting>,
}

impl TransactionManager {
    /// Initializes a new TransactionManager with P2P modules
    pub fn new(
        peer_manager: Arc<PeerManager>,
        gossip_protocol: Arc<GossipProtocol>,
        multi_hop_routing: Arc<MultiHopRouting>,
        dark_routing: Arc<DarkRouting>,
    ) -> Self {
        Self {
            peer_manager,
            gossip_protocol,
            multi_hop_routing,
            dark_routing,
        }
    }

    /// Broadcasts a transaction to all peers using GossipProtocol
    pub async fn broadcast_transaction(&self, transaction: ZKTransaction) {
        let message = P2PMessage::Transaction(transaction);
        self.gossip_protocol.broadcast_message(message).await;
    }

    /// Routes a transaction securely over multiple hops
    pub async fn route_transaction(&self, sender: &str, receiver: &str, transaction: ZKTransaction) {
        let route = self.multi_hop_routing.select_route(sender, receiver);
        self.multi_hop_routing.forward_message(route, P2PMessage::Transaction(transaction)).await;
    }

    /// Sends a fully anonymous transaction using DarkRouting
    pub async fn send_anonymous_transaction(&self, sender: &str, transaction: ZKTransaction) {
        let route = self.dark_routing.select_anonymous_route(sender);
        self.dark_routing.forward_anonymous(route, P2PMessage::Transaction(transaction)).await;
    }

    /// Processes incoming P2P transaction messages
    pub async fn process_p2p_message(&self, message: P2PMessage) {
        match message {
            P2PMessage::Transaction(tx) => {
                if tx.verify(&QuantumEncryption::get_public_key()) {
                    self.peer_manager.add_transaction_to_pool(tx);
                    println!("✅ Valid transaction received and added to mempool.");
                } else {
                    println!("❌ Invalid transaction rejected.");
                }
            },
            _ => {}
        }
    }
}