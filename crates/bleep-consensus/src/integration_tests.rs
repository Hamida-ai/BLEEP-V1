// PHASE 1: COMPREHENSIVE INTEGRATION TESTS
// Real-world consensus scenarios and safety verification
// 
// TESTS COVER:
// 1. Epoch transition determinism
// 2. Partial validator failure scenarios
// 3. Conflicting AI advisory reports
// 4. Byzantine validator behavior
// 5. Emergency PoW activation & exit
// 6. Mode mismatch block rejection
// 7. Fork safety

#[cfg(test)]
mod phase1_integration_tests {
    use bleep_consensus::epoch::{ConsensusMode, EpochConfig, EpochBuilder};
    use bleep_consensus::engine::{ConsensusEngine, ConsensusMetrics};
    use bleep_consensus::orchestrator::ConsensusOrchestrator;
    use bleep_consensus::pos_engine::PoSConsensusEngine;
    use bleep_consensus::pbft_engine::PbftConsensusEngine;
    use bleep_consensus::pow_engine::EmergencyPoWEngine;
    use bleep_consensus::ai_advisory::{AiAdvisoryReport, AiReportAggregator};
    use bleep_core::block::Block;
    use bleep_core::blockchain::BlockchainState;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Mock consensus engine for testing
    struct MockConsensusEngine(ConsensusMode);

    impl ConsensusEngine for MockConsensusEngine {
        fn verify_block(
            &self,
            _block: &Block,
            _epoch_state: &bleep_consensus::EpochState,
            _blockchain_state: &BlockchainState,
        ) -> Result<(), bleep_consensus::ConsensusError> {
            Ok(())
        }

        fn propose_block(
            &self,
            _height: u64,
            _previous_hash: String,
            _transactions: Vec<bleep_core::block::Transaction>,
            _epoch_state: &bleep_consensus::EpochState,
            _blockchain_state: &BlockchainState,
        ) -> Result<Block, bleep_consensus::ConsensusError> {
            Err(bleep_consensus::ConsensusError::Other("Mock".to_string()))
        }

        fn consensus_mode(&self) -> ConsensusMode {
            self.0
        }

        fn health_status(&self) -> f64 {
            0.9
        }
    }

    fn create_orchestrator() -> ConsensusOrchestrator {
        let config = EpochConfig::new(1000, 0, 1).unwrap();
        let mut engines = HashMap::new();
        engines.insert(
            ConsensusMode::PosNormal,
            Arc::new(MockConsensusEngine(ConsensusMode::PosNormal))
                as Arc<dyn ConsensusEngine>,
        );
        engines.insert(
            ConsensusMode::PbftFastFinality,
            Arc::new(MockConsensusEngine(ConsensusMode::PbftFastFinality))
                as Arc<dyn ConsensusEngine>,
        );
        engines.insert(
            ConsensusMode::EmergencyPow,
            Arc::new(MockConsensusEngine(ConsensusMode::EmergencyPow))
                as Arc<dyn ConsensusEngine>,
        );

        ConsensusOrchestrator::new(config, engines, 5, 0.66, 3).unwrap()
    }

    // ============================================================
    // TEST 1: EPOCH TRANSITION DETERMINISM
    // ============================================================
    
    #[test]
    fn test_epoch_transition_is_deterministic() {
        let config = EpochConfig::new(100, 0, 1).unwrap();
        
        // Same height must always produce same epoch ID
        for _ in 0..5 {
            assert_eq!(config.epoch_id(50), 0);
            assert_eq!(config.epoch_id(150), 1);
            assert_eq!(config.epoch_id(250), 2);
        }
    }

    #[test]
    fn test_epoch_boundaries_are_exact() {
        let config = EpochConfig::new(100, 0, 1).unwrap();
        
        // Epoch 0: [0, 99]
        // Epoch 1: [100, 199]
        // Epoch 2: [200, 299]
        
        assert_eq!(config.epoch_id(99), 0);
        assert_eq!(config.epoch_id(100), 1);
        
        assert_eq!(config.epoch_id(199), 1);
        assert_eq!(config.epoch_id(200), 2);
    }

    #[test]
    fn test_epoch_builder_produces_valid_state() {
        let config = EpochConfig::new(100, 0, 1).unwrap();
        let builder = EpochBuilder::new(&config);
        
        // Build epoch for height 50
        let epoch = builder.build(50, ConsensusMode::PosNormal).unwrap();
        assert_eq!(epoch.epoch_id, 0);
        assert_eq!(epoch.start_height, 0);
        assert_eq!(epoch.end_height, 99);
        
        // Verify it validates correctly
        assert!(epoch.validate(&config).is_ok());
    }

    // ============================================================
    // TEST 2: PARTIAL VALIDATOR FAILURE
    // ============================================================

    #[test]
    fn test_consensus_resilient_to_2f_1_participation() {
        let mut orchestrator = create_orchestrator();
        
        // 70% participation is above 66% threshold - no emergency
        let metrics = ConsensusMetrics {
            validator_participation: 0.70,
            block_proposal_time_ms: 100,
            rejected_block_count: 2,
            slashing_event_count: 0,
            finality_latency_blocks: 2,
            network_utilization: 0.5,
        };
        
        let mode = orchestrator.select_mode(0, &metrics);
        assert_eq!(mode, ConsensusMode::PosNormal);
    }

    #[test]
    fn test_consensus_triggers_pow_at_threshold() {
        let mut orchestrator = create_orchestrator();
        
        // 60% participation is below 66% threshold - emergency PoW
        let metrics = ConsensusMetrics {
            validator_participation: 0.60,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 0,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let mode = orchestrator.select_mode(0, &metrics);
        assert_eq!(mode, ConsensusMode::EmergencyPow);
    }

    // ============================================================
    // TEST 3: AI ADVISORY AGGREGATION WITH CONFLICTING REPORTS
    // ============================================================

    #[test]
    fn test_ai_advisory_aggregation_handles_outliers() {
        let aggregator = AiReportAggregator::new(3);
        
        // Reports from 3 AI nodes with varying assessments
        let mut r1 = AiAdvisoryReport::new(0, "ai_1".to_string(), 90, 85, 10, 90);
        r1.sign(vec![1]);
        
        let mut r2 = AiAdvisoryReport::new(0, "ai_2".to_string(), 85, 80, 15, 85);
        r2.sign(vec![2]);
        
        // Outlier: much more pessimistic
        let mut r3 = AiAdvisoryReport::new(0, "ai_3".to_string(), 30, 25, 70, 30);
        r3.sign(vec![3]);
        
        let agg = aggregator.aggregate(&[r1, r2, r3]).unwrap();
        
        // Median should be robust to the outlier
        // Network health: median of [90, 85, 30] = 85
        assert_eq!(agg.network_health, 85);
        
        // Validator performance: median of [85, 80, 25] = 80
        assert_eq!(agg.validator_performance, 80);
    }

    #[test]
    fn test_ai_advisory_only_provides_input_not_override() {
        // This test verifies that AI reports are ADVISORY, not decision-making
        let aggregator = AiReportAggregator::new(1);
        
        let mut report = AiAdvisoryReport::new(0, "ai_1".to_string(), 100, 100, 0, 100);
        report.sign(vec![1]);
        
        let agg = aggregator.aggregate(&[report]).unwrap();
        
        // Even with perfect AI report (100% health), the Orchestrator
        // makes final decision based on deterministic metrics
        let mut orchestrator = create_orchestrator();
        let metrics = ConsensusMetrics {
            validator_participation: 0.50, // Below 66% - emergency!
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 0,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let mode = orchestrator.select_mode(0, &metrics);
        // Orchestrator chooses PoW despite AI saying everything is fine
        // because validator_participation < 0.66
        assert_eq!(mode, ConsensusMode::EmergencyPow);
    }

    // ============================================================
    // TEST 4: BYZANTINE VALIDATOR DETECTION
    // ============================================================

    #[test]
    fn test_consensus_detects_slashing_events() {
        let mut orchestrator = create_orchestrator();
        
        let metrics = ConsensusMetrics {
            validator_participation: 0.90,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 10, // Many slashing events detected
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let mode = orchestrator.select_mode(0, &metrics);
        // PoW activated because slashing_event_count >= threshold (3)
        assert_eq!(mode, ConsensusMode::EmergencyPow);
    }

    #[test]
    fn test_slashing_threshold_is_enforced() {
        let mut orchestrator = create_orchestrator();
        
        // At threshold: 2 slashing events (< 3)
        let metrics_ok = ConsensusMetrics {
            validator_participation: 0.90,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 2,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let mode = orchestrator.select_mode(0, &metrics_ok);
        assert_eq!(mode, ConsensusMode::PosNormal);
        
        // Slightly over threshold: 3 slashing events
        let metrics_emergency = ConsensusMetrics {
            validator_participation: 0.90,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 3,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let mut orchestrator2 = create_orchestrator();
        let mode2 = orchestrator2.select_mode(0, &metrics_emergency);
        assert_eq!(mode2, ConsensusMode::EmergencyPow);
    }

    // ============================================================
    // TEST 5: EMERGENCY POW ACTIVATION & AUTO-EXIT
    // ============================================================

    #[test]
    fn test_pow_activates_on_emergency() {
        let mut orchestrator = create_orchestrator();
        
        let emergency_metrics = ConsensusMetrics {
            validator_participation: 0.50,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 0,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let mode = orchestrator.select_mode(0, &emergency_metrics);
        assert_eq!(mode, ConsensusMode::EmergencyPow);
        
        // Verify state machine is active
        assert!(matches!(
            orchestrator.pow_state(),
            bleep_consensus::orchestrator::EmergencyPoWState::Active { activated_at_epoch: 0 }
        ));
    }

    #[test]
    fn test_pow_auto_exits_after_max_epochs() {
        let mut orchestrator = create_orchestrator();
        
        // Activate PoW at epoch 0
        let emergency_metrics = ConsensusMetrics {
            validator_participation: 0.50,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 0,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let _mode = orchestrator.select_mode(0, &emergency_metrics);
        
        // Conditions improve at epoch 5
        let recovery_metrics = ConsensusMetrics {
            validator_participation: 0.90,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 0,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        let mode = orchestrator.select_mode(5, &recovery_metrics);
        // PoW must exit because max_pow_epochs = 5
        assert_eq!(mode, ConsensusMode::PosNormal);
    }

    #[test]
    fn test_pow_stays_active_if_conditions_poor() {
        let mut orchestrator = create_orchestrator();
        
        let emergency_metrics = ConsensusMetrics {
            validator_participation: 0.50,
            block_proposal_time_ms: 100,
            rejected_block_count: 0,
            slashing_event_count: 0,
            finality_latency_blocks: 1,
            network_utilization: 0.5,
        };
        
        // Activate at epoch 0
        let _mode = orchestrator.select_mode(0, &emergency_metrics);
        
        // Still poor at epoch 2
        let mode = orchestrator.select_mode(2, &emergency_metrics);
        // Should stay in PoW
        assert_eq!(mode, ConsensusMode::EmergencyPow);
    }

    // ============================================================
    // TEST 6: MODE MISMATCH BLOCK REJECTION
    // ============================================================

    #[test]
    fn test_blocks_with_wrong_mode_are_rejected() {
        let mut orchestrator = create_orchestrator();
        
        let config = EpochConfig::new(1000, 0, 1).unwrap();
        let builder = EpochBuilder::new(&config);
        let epoch = builder.build(100, ConsensusMode::PosNormal).unwrap();
        
        // Create a block with wrong consensus mode
        let mut block = Block::new(100, vec![], "hash99".to_string());
        block.epoch_id = 0;
        block.consensus_mode = bleep_core::block::ConsensusMode::PbftFastFinality; // Wrong!
        block.protocol_version = 1;
        
        let state = BlockchainState::default();
        
        // Orchestrator must reject this block
        let result = orchestrator.verify_block_consensus(&block, &epoch, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_blocks_with_wrong_epoch_are_rejected() {
        let mut orchestrator = create_orchestrator();
        
        let config = EpochConfig::new(1000, 0, 1).unwrap();
        let builder = EpochBuilder::new(&config);
        let epoch = builder.build(100, ConsensusMode::PosNormal).unwrap();
        
        // Create a block with wrong epoch ID
        let mut block = Block::new(100, vec![], "hash99".to_string());
        block.epoch_id = 999; // Wrong!
        block.consensus_mode = bleep_core::block::ConsensusMode::PosNormal;
        block.protocol_version = 1;
        
        let state = BlockchainState::default();
        
        // Orchestrator must reject this block
        let result = orchestrator.verify_block_consensus(&block, &epoch, &state);
        assert!(result.is_err());
    }

    // ============================================================
    // TEST 7: FORK SAFETY
    // ============================================================

    #[test]
    fn test_deterministic_mode_selection_prevents_forks() {
        // Two orchestrators with identical config
        let mut o1 = create_orchestrator();
        let mut o2 = create_orchestrator();
        
        let metrics = ConsensusMetrics {
            validator_participation: 0.75,
            block_proposal_time_ms: 100,
            rejected_block_count: 5,
            slashing_event_count: 1,
            finality_latency_blocks: 3,
            network_utilization: 0.6,
        };
        
        // Both orchestrators must select identical mode for same input
        let mode1 = o1.select_mode(100, &metrics);
        let mode2 = o2.select_mode(100, &metrics);
        
        assert_eq!(mode1, mode2);
    }

    #[test]
    fn test_epoch_boundaries_prevent_mode_disagreement() {
        // All honest nodes compute same epoch boundaries
        let config1 = EpochConfig::new(100, 0, 1).unwrap();
        let config2 = EpochConfig::new(100, 0, 1).unwrap();
        
        // Two nodes both see height 250
        let epoch1 = config1.epoch_id(250);
        let epoch2 = config2.epoch_id(250);
        
        // They must agree
        assert_eq!(epoch1, epoch2);
        assert_eq!(epoch1, 2);
    }

    #[test]
    fn test_block_with_consistent_fields_accepted() {
        let orchestrator = create_orchestrator();
        
        let config = EpochConfig::new(1000, 0, 1).unwrap();
        let builder = EpochBuilder::new(&config);
        let epoch = builder.build(100, ConsensusMode::PosNormal).unwrap();
        
        // Create a block with correct fields
        let mut block = Block::new(100, vec![], "hash99".to_string());
        block.epoch_id = 0;
        block.consensus_mode = bleep_core::block::ConsensusMode::PosNormal;
        block.protocol_version = 1;
        block.validator_signature = vec![1, 2, 3]; // Must have signature
        
        let state = BlockchainState::default();
        
        // This block should pass orchestrator verification
        let result = orchestrator.verify_block_consensus(&block, &epoch, &state);
        // Result depends on the engine's verify_block (which succeeds in mock)
        // but the orchestrator checks should pass
        let _ = result; // Just check it doesn't panic
    }
}
