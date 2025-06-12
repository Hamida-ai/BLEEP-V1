use advanced_bleep::core::blockchain::{Blockchain, Block};
use advanced_bleep::core::transaction::{Transaction, Mempool};
use advanced_bleep::core::networking::P2PNode;
use advanced_bleep::core::sharding::ShardManager;
use advanced_bleep::core::ai::DynamicLoadBalancer;
use advanced_bleep::core::config::MAX_TRANSACTIONS_PER_SECOND;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Generates a high volume of transactions for scalability testing
fn generate_high_volume_transactions(count: usize) -> Vec<Transaction> {
    let mut transactions = Vec::new();
    for i in 0..count {
        let tx = Transaction::new(
            &format!("user{}", i % 1000),
            &format!("user{}", (i + 1) % 1000),
            10,
            "valid_signature"
        );
        transactions.push(tx);
    }
    transactions
}

#[test]
fn scalability_test_bleep_blockchain() {
    println!("🚀 **Starting BLEEP Blockchain Scalability Test...**");

    // 🌎 Initialize Blockchain, Mempool, P2P Network, AI Load Balancer, and Shard Manager
    let blockchain = Arc::new(Mutex::new(Blockchain::new()));
    let mempool = Arc::new(Mutex::new(Mempool::new()));
    let p2p_node = Arc::new(P2PNode::new("127.0.0.1:9002"));
    let shard_manager = Arc::new(Mutex::new(ShardManager::new()));
    let load_balancer = DynamicLoadBalancer::new();

    // 🚀 Define Test Parameters
    let num_transactions = 1_000_000; // 1M transactions
    let transactions = generate_high_volume_transactions(num_transactions);

    // 📌 Start Scalability Test
    let start_time = Instant::now();

    // 🚀 1. Test High Transaction Throughput
    println!("🛠 **Processing {} Transactions...**", num_transactions);
    for tx in transactions {
        mempool.lock().unwrap().add_transaction(tx);
    }
    let elapsed_time = start_time.elapsed().as_secs_f64();
    let tps = num_transactions as f64 / elapsed_time;
    println!("📊 **Processed Transactions Per Second (TPS): {:.2}**", tps);
    assert!(tps >= MAX_TRANSACTIONS_PER_SECOND as f64, "🚨 TPS below expected threshold!");

    // 🚀 2. Test Sharding Efficiency
    let start_sharding = Instant::now();
    shard_manager.lock().unwrap().distribute_transactions();
    let sharding_time = start_sharding.elapsed().as_millis();
    println!("📊 **Sharding Execution Time: {} ms**", sharding_time);
    assert!(sharding_time < 500, "🚨 Sharding is too slow!");

    // 🚀 3. Test AI Load Balancing Under High Load
    let start_ai = Instant::now();
    let optimized = load_balancer.optimize_network_traffic();
    let ai_execution_time = start_ai.elapsed().as_millis();
    println!("📊 **AI Load Balancer Execution Time: {} ms**", ai_execution_time);
    assert!(optimized, "🚨 AI Load Balancer failed to optimize network!");

    // 🚀 4. Test Block Propagation Speed
    let start_propagation = Instant::now();
    let block = Block::new(mempool.lock().unwrap().get_pending_transactions());
    blockchain.lock().unwrap().add_block(block);
    let propagation_time = start_propagation.elapsed().as_millis();
    println!("📊 **Block Propagation Time: {} ms**", propagation_time);
    assert!(propagation_time < 200, "🚨 Block propagation is too slow!");

    // 🚀 5. Test Network Scalability With 10,000 Nodes
    let start_network_scaling = Instant::now();
    for i in 0..10_000 {
        let node_address = format!("127.0.0.1:{}", 9003 + i);
        p2p_node.connect_peer(&node_address);
    }
    let network_scaling_time = start_network_scaling.elapsed().as_secs_f64();
    println!("📊 **P2P Network Scaling Time: {:.2} seconds**", network_scaling_time);
    assert!(network_scaling_time < 60.0, "🚨 P2P network scaling too slow!");

    println!("✅ **BLEEP Blockchain Scalability Test Completed Successfully!**");
}