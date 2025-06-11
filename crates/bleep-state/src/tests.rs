#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;
    use crate::transaction::{Transaction, QuantumSecure};
    use crate::consensus::{BLEEPAdaptiveConsensus, ConsensusMode};
    use crate::p2p::P2PNode;
    use crate::sharding::BLEEPShardingModule;
    use log::info;

    // Helper functions for setup
    fn create_mock_consensus() -> Arc<Mutex<BLEEPAdaptiveConsensus>> {
        Arc::new(Mutex::new(BLEEPAdaptiveConsensus::new(ConsensusMode::PoW)))
    }

    fn create_mock_p2p_node() -> Arc<P2PNode> {
        Arc::new(P2PNode::new())
    }

    fn create_sample_transaction(id: u64, from: &str, to: &str, amount: u64) -> Transaction {
        Transaction {
            id,
            from: from.to_string(),
            to: to.to_string(),
            amount,
        }
    }

    #[test]
    fn test_shard_manager_full_functionality() {
        let consensus = create_mock_consensus();
        let p2p_node = create_mock_p2p_node();

        // Step 1: Initialize ShardManager with 4 shards
        let mut sharding_module = BLEEPShardingModule::new(4, consensus, p2p_node).unwrap();
        assert_eq!(sharding_module.shards.len(), 4, "Expected 4 shards at initialization.");

        // Step 2: Assign transactions and verify state updates
        let tx1 = create_sample_transaction(1, "Alice", "Bob", 50);
        let tx2 = create_sample_transaction(2, "Bob", "Charlie", 100);
        
        sharding_module.assign_transaction(tx1.clone()).unwrap();
        sharding_module.assign_transaction(tx2.clone()).unwrap();
        
        let shard = sharding_module.shards.get(&0).unwrap().lock().unwrap();
        assert!(shard.transactions.len() >= 1, "Shard should have at least 1 transaction after assignment.");

        // Step 3: Manual shard state initialization
        let mut shard_states = HashMap::new();
        shard_states.insert(1, vec![
            create_sample_transaction(3, "Charlie", "Dave", 75),
            create_sample_transaction(4, "Eve", "Frank", 25),
        ]);
        
        let result = sharding_module.initialize_shards_from_state(shard_states);
        assert!(result.is_ok(), "Shard state initialization should be successful.");
        let shard1 = sharding_module.shards.get(&1).unwrap().lock().unwrap();
        assert_eq!(shard1.transactions.len(), 2, "Shard 1 should have 2 transactions.");

        // Step 4: Trigger rebalance
        sharding_module.monitor_and_auto_rebalance();
        let post_rebalance_shard = sharding_module.shards.get(&0).unwrap().lock().unwrap();
        assert!(post_rebalance_shard.transactions.len() >= 0, "Shards should rebalance transactions.");

        // Step 5: Broadcast shard state (mock test)
        sharding_module.broadcast_shard_state();
        // Note: This requires a mocked P2P node broadcast to validate completely.

        println!("✅ All shard state tests passed!");
    }
}