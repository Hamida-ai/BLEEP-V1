#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::runtime::Runtime;

    #[test]
    fn test_validator_recovery() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let consensus = Arc::new(Mutex::new(BLEEPAdaptiveConsensus::new()));
            let monitor = BlockchainMonitor::new(consensus.clone());

            // Simulating a failed validator
            monitor.update_validator_status(1, false);
            monitor.recover_failed_validator(1).await;

            let health_status = monitor.health_status.lock().unwrap();
            assert_eq!(health_status.get(&1), Some(&true), "Validator should be recovered");
        });
    }

    #[test]
    fn test_auto_shard_balancing() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let sharding = Arc::new(Mutex::new(BLEEPShardingModule::new()));
            let ai_engine = Arc::new(BLEEPAIDecisionModule::new());
            let shard_manager = ShardManager { sharding, ai_engine };

            shard_manager.auto_shard_balancing().await;

            let current_shard_count = shard_manager.sharding.lock().unwrap().get_shard_count();
            assert!(current_shard_count > 0, "Shards should be dynamically managed");
        });
    }

    #[test]
    fn test_recover_corrupt_state() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let ai_engine = Arc::new(BLEEPAIDecisionModule::new());
            let state_merkle = Arc::new(Mutex::new(StateMerkle::new()));
            let state_monitor = BlockchainStateMonitor { ai_engine, state_merkle };

            // Simulating blockchain anomaly detection
            state_monitor.recover_corrupt_state().await;

            let last_state = state_monitor.state_merkle.lock().unwrap().get_current_state();
            assert!(last_state.is_some(), "State should be restored to a valid checkpoint");
        });
    }

    #[test]
    fn test_smart_contract_security() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let ai_engine = Arc::new(BLEEPAIDecisionModule::new());
            let security_module = SmartContractSecurity { ai_engine };

            let contract_code = "contract Test { function test() public { expensive_op; } }";
            let optimized_code = security_module.secure_and_optimize_smart_contract(contract_code).await;

            assert!(
                !optimized_code.contains("Security flaws detected"),
                "Contract should be optimized and secure"
            );
        });
    }

    #[test]
    fn test_self_healing_automation() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let consensus = Arc::new(Mutex::new(BLEEPAdaptiveConsensus::new()));
            let monitor = BlockchainMonitor::new(consensus.clone());

            let sharding = Arc::new(Mutex::new(BLEEPShardingModule::new()));
            let ai_engine = Arc::new(BLEEPAIDecisionModule::new());
            let shard_manager = ShardManager { sharding, ai_engine.clone() };

            let state_merkle = Arc::new(Mutex::new(StateMerkle::new()));
            let state_monitor = BlockchainStateMonitor { ai_engine, state_merkle };

            let security_module = SmartContractSecurity { ai_engine: Arc::new(BLEEPAIDecisionModule::new()) };

            let self_healing = BLEEPSelfHealingAutomation::new(monitor, shard_manager, state_monitor, security_module);

            self_healing.run().await;

            assert!(
                self_healing.monitor.health_status.lock().unwrap().len() > 0,
                "Validators should have updated health statuses"
            );
        });
    }
                                               }
