use advanced_bleep::networking::{P2PNetwork, PeerNode, GossipProtocol, NoiseProtocol, QUICTransport};
use advanced_bleep::core::crypto::ProofOfIdentity;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn networking_p2p_test() {
    println!("🚀 **Starting BLEEP Blockchain Networking & P2P Test...**");

    // 🌎 Initialize P2P Network
    let p2p_network = Arc::new(Mutex::new(P2PNetwork::new()));

    // 📌 Start Networking & P2P Test
    let start_time = Instant::now();

    // 🚀 1. Test Peer Discovery
    println!("⚠️ **Testing peer discovery mechanism...**");
    let node1 = PeerNode::new("192.168.1.1:30303");
    let node2 = PeerNode::new("192.168.1.2:30303");
    let node3 = PeerNode::new("192.168.1.3:30303");

    p2p_network.lock().unwrap().add_peer(node1.clone());
    p2p_network.lock().unwrap().add_peer(node2.clone());
    p2p_network.lock().unwrap().add_peer(node3.clone());

    assert!(p2p_network.lock().unwrap().peer_count() == 3, "🚨 Peer discovery failed!");

    // 🚀 2. Test Secure Communication (Noise Protocol + QUIC)
    println!("⚠️ **Testing secure peer communication using Noise + QUIC...**");
    let message = "Hello, BLEEP!";
    let encrypted_message = NoiseProtocol::encrypt(&message, node2.public_key());
    let decrypted_message = NoiseProtocol::decrypt(&encrypted_message, node2.private_key());

    assert!(decrypted_message == message, "🚨 Secure communication failed!");

    // 🚀 3. Test Gossip Protocol Efficiency
    println!("⚠️ **Testing gossip protocol efficiency...**");
    let transaction = "TX_123456";
    GossipProtocol::broadcast(transaction);

    let received_tx = p2p_network.lock().unwrap().listen_for_gossip();
    assert!(received_tx == Some(transaction.to_string()), "🚨 Gossip protocol failed!");

    // 🚀 4. Test Network Stability Under Load
    println!("⚠️ **Testing network stability under high load (1M messages)...**");
    for i in 0..1_000_000 {
        p2p_network.lock().unwrap().send_message(&format!("Message {}", i));
    }
    assert!(p2p_network.lock().unwrap().message_queue_size() == 1_000_000, "🚨 Network instability detected!");

    // 🚀 5. Test Resistance to Sybil Attacks (Proof-of-Identity)
    println!("⚠️ **Testing resistance to Sybil attacks...**");
    let malicious_node = PeerNode::new("192.168.1.100:30303");
    let forged_identity = ProofOfIdentity::forge_identity();
    let is_legit = p2p_network.lock().unwrap().validate_peer_identity(malicious_node.clone(), forged_identity);

    assert!(!is_legit, "🚨 Sybil attack not detected!");

    // 🚀 6. Test AI-Powered Anomaly Detection
    println!("⚠️ **Testing AI-powered anomaly detection...**");
    let anomaly_detected = p2p_network.lock().unwrap().detect_anomalies();
    assert!(anomaly_detected, "🚨 Anomaly detection failed!");

    println!("✅ **BLEEP Blockchain Networking & P2P Test Completed Successfully!**");
}