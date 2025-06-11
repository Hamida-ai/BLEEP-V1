#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::{
        peer_manager::PeerManager,
        gossip_protocol::GossipProtocol,
        multi_hop_routing::MultiHopRouting,
        dark_routing::DarkRouting,
        message_protocol::{MessageProtocol, SecureMessage},
    };
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_peer_discovery() {
        let peer_manager = Arc::new(PeerManager::new());
        peer_manager.add_peer("node_1".to_string(), "192.168.1.1:3000".to_string());
        peer_manager.add_peer("node_2".to_string(), "192.168.1.2:3000".to_string());

        let peers = peer_manager.get_peers();
        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&"node_1".to_string()));
        assert!(peers.contains(&"node_2".to_string()));
    }

    #[tokio::test]
    async fn test_gossip_propagation() {
        let peer_manager = Arc::new(PeerManager::new());
        let gossip_protocol = GossipProtocol::new(peer_manager.clone());

        peer_manager.add_peer("node_1".to_string(), "192.168.1.1:3000".to_string());
        peer_manager.add_peer("node_2".to_string(), "192.168.1.2:3000".to_string());

        let message = "Test Gossip Message".to_string();
        gossip_protocol.spread_message("node_1".to_string(), message.clone()).await;

        let received = gossip_protocol.get_received_messages();
        assert!(received.contains(&message));
    }

    #[tokio::test]
    async fn test_multi_hop_routing_with_failures() {
        let peer_manager = Arc::new(PeerManager::new());
        let message_protocol = MessageProtocol::new();
        let multi_hop_routing = MultiHopRouting::new(peer_manager.clone(), message_protocol.clone());

        peer_manager.add_peer("A".to_string(), "192.168.1.1:3000".to_string());
        peer_manager.add_peer("B".to_string(), "192.168.1.2:3000".to_string());
        peer_manager.add_peer("C".to_string(), "192.168.1.3:3000".to_string());

        // Simulating a failure (Removing node B)
        peer_manager.remove_peer("B");

        let message = SecureMessage {
            sender_id: "A".to_string(),
            payload: "Test Multi-Hop".to_string(),
            hop_count: 0,
        };

        let route = multi_hop_routing.select_route("A", "C");
        
        // Ensure rerouting happens when a node fails
        assert!(!route.contains(&"B".to_string()));
        assert!(route.contains(&"C".to_string()));
    }

    #[tokio::test]
    async fn test_dark_routing_anonymity_with_trust_scores() {
        let peer_manager = Arc::new(PeerManager::new());
        let message_protocol = MessageProtocol::new();
        let dark_routing = DarkRouting::new(peer_manager.clone(), message_protocol.clone());

        peer_manager.add_peer("X".to_string(), "192.168.1.4:3000".to_string());
        peer_manager.add_peer("Y".to_string(), "192.168.1.5:3000".to_string());
        peer_manager.add_peer("Z".to_string(), "192.168.1.6:3000".to_string());

        // Assign AI trust scores
        {
            let mut trust_scores = dark_routing.ai_security.lock().unwrap();
            trust_scores.insert("X".to_string(), 90);
            trust_scores.insert("Y".to_string(), 20); // Low-trust node
            trust_scores.insert("Z".to_string(), 80);
        }

        let message = SecureMessage {
            sender_id: "X".to_string(),
            payload: "Dark Route Test".to_string(),
            hop_count: 0,
        };

        dark_routing.send_anonymous_message(message.clone()).await;
        let route = dark_routing.select_anonymous_route("X");

        assert!(route.len() > 1);
        assert_ne!(route.first().unwrap(), &"X".to_string());

        // Ensure low-trust node "Y" is rarely selected
        let low_trust_count = route.iter().filter(|&&ref node| node == "Y").count();
        assert!(low_trust_count <= 1);
    }

    #[tokio::test]
    async fn test_message_timing_analysis() {
        let peer_manager = Arc::new(PeerManager::new());
        let message_protocol = MessageProtocol::new();
        let multi_hop_routing = MultiHopRouting::new(peer_manager.clone(), message_protocol.clone());

        peer_manager.add_peer("A".to_string(), "192.168.1.1:3000".to_string());
        peer_manager.add_peer("B".to_string(), "192.168.1.2:3000".to_string());
        peer_manager.add_peer("C".to_string(), "192.168.1.3:3000".to_string());

        let message = SecureMessage {
            sender_id: "A".to_string(),
            payload: "Timing Test".to_string(),
            hop_count: 0,
        };

        let start_time = Instant::now();
        multi_hop_routing.relay_message(message).await;
        let elapsed_time = start_time.elapsed();

        // Ensure messages arrive within a reasonable timeframe (e.g., 500ms)
        assert!(elapsed_time.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_onion_encryption_decryption() {
        let peer_manager = Arc::new(PeerManager::new());
        let message_protocol = MessageProtocol::new();
        let dark_routing = DarkRouting::new(peer_manager.clone(), message_protocol.clone());

        peer_manager.add_peer("X".to_string(), "192.168.1.4:3000".to_string());
        peer_manager.add_peer("Y".to_string(), "192.168.1.5:3000".to_string());
        peer_manager.add_peer("Z".to_string(), "192.168.1.6:3000".to_string());

        let message = SecureMessage {
            sender_id: "X".to_string(),
            payload: "Onion Test".to_string(),
            hop_count: 0,
        };

        let route = dark_routing.select_anonymous_route("X");
        let encrypted_layers = dark_routing.onion_encrypt(message.clone(), &route);

        // Decrypt step-by-step to verify
        let mut decrypted_payload = encrypted_layers.last().unwrap().payload.clone();
        for node in route.iter() {
            decrypted_payload = decrypt_message(&decrypted_payload, node);
        }

        assert_eq!(decrypted_payload, "Onion Test".to_string());
    }
}