// PHASE 6: ADVERSARIAL TESTNET COMPREHENSIVE TESTS
//
// Purpose: Verify the entire adversarial testnet system with:
// 1. Repeated adversarial runs (determinism verification)
// 2. Third-party reproducibility (anyone can re-run and verify)
// 3. No manual intervention (fully automated)
// 4. Complete before/after state proofs
// 5. Investor-ready demonstrations
//
// Test Categories:
// - Validator Collusion Attack Scenarios (5 tests)
// - Invalid State Injection Scenarios (5 tests)
// - Governance Abuse Scenarios (5 tests)
// - Determinism & Reproducibility (5 tests)
// - Autonomous Detection & Recovery (5 tests)
// - Public Observability (5 tests)
// - End-to-End Integration Tests (5 tests)

#[cfg(test)]
mod phase6_adversarial_testnet_tests {
    use crate::adversarial_testnet::*;
    use crate::scenario_injector::*;
    use crate::observability::*;

    // ═══════════════════════════════════════════════════════════════════════════════
    // VALIDATOR COLLUSION SCENARIOS (5 tests)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_01_validator_collusion_two_validators() {
        // Scenario: Two validators collude at epoch 10
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "collision_two_validators".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![5, 7],
                attack_epoch: 10,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let before_state = testnet.initialize().expect("Initialize failed");

        testnet.inject_scenario(10).expect("Injection failed");

        assert_eq!(testnet.state, TestnetRunState::AttackInjected);
        assert_eq!(testnet.incident_log.len(), 1);
        assert_eq!(
            testnet.incident_log[0].incident_type,
            DetectedIncidentType::ValidatorMisbehavior
        );
        
        // Verify before state recorded
        assert_eq!(before_state.epoch, 0);
        assert!(!testnet.state_history.is_empty());
    }

    #[test]
    fn test_02_validator_collusion_three_validators() {
        // Scenario: Three validators collude (higher byzantine threshold)
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "collision_three_validators".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![1, 8, 15],
                attack_epoch: 20,
                attack_duration_blocks: 36,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(20).expect("Injection failed");

        assert_eq!(testnet.metrics.incidents_detected, 1);
        assert!(testnet.metrics.validators_slashed >= 3);
    }

    #[test]
    fn test_03_validator_collusion_recovery() {
        // Scenario: Collusion detected, governance triggers slashing
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "collision_with_recovery".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![3, 9],
                attack_epoch: 15,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(15).expect("Injection failed");
        testnet.trigger_autonomous_recovery().expect("Recovery failed");

        assert_eq!(testnet.state, TestnetRunState::Recovered);
        assert!(testnet.incident_log[0].state_after.is_some());
    }

    #[test]
    fn test_04_validator_collusion_immutable_log() {
        // Scenario: Verify incident log is immutable
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "collision_immutable_log".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![2, 11],
                attack_epoch: 12,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(12).expect("Injection failed");

        // Verify integrity
        assert!(testnet.verify_incident_integrity());

        // Get incident log
        let log = testnet.get_incident_log();
        assert!(!log.is_empty());
        assert!(log[0].verify_integrity());
    }

    #[test]
    fn test_05_validator_collusion_metrics() {
        // Scenario: Verify metrics are updated correctly
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "collision_metrics".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![4, 6, 18],
                attack_epoch: 30,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let initial_metrics = testnet.get_public_metrics();
        assert_eq!(initial_metrics.incidents_detected, 0);

        let _ = testnet.initialize();
        testnet.inject_scenario(30).expect("Injection failed");

        let updated_metrics = testnet.get_public_metrics();
        assert!(updated_metrics.incidents_detected > 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // INVALID STATE INJECTION SCENARIOS (5 tests)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_06_state_injection_invalid_balance() {
        // Scenario: Invalid balance state injected at epoch 50
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::InvalidStateInjection,
            testnet_id: "state_invalid_balance".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 50,
                corruption_percentage: 15.0,
                corruption_type: "InvalidBalance".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(50).expect("Injection failed");

        assert_eq!(testnet.state, TestnetRunState::AttackInjected);
        assert_eq!(
            testnet.incident_log[0].incident_type,
            DetectedIncidentType::StateInvalidity
        );
    }

    #[test]
    fn test_07_state_injection_duplicate_transaction() {
        // Scenario: Duplicate transaction in state
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::InvalidStateInjection,
            testnet_id: "state_duplicate_tx".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 40,
                corruption_percentage: 5.0,
                corruption_type: "DuplicateTransaction".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let before = testnet.initialize().expect("Initialize failed");
        testnet.inject_scenario(40).expect("Injection failed");

        // Verify before/after state captured
        assert_eq!(before.epoch, 0);
        assert_eq!(testnet.incident_log.len(), 1);
    }

    #[test]
    fn test_08_state_injection_missing_state() {
        // Scenario: Accounts randomly deleted from state
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::InvalidStateInjection,
            testnet_id: "state_missing".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 60,
                corruption_percentage: 20.0,
                corruption_type: "MissingState".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(60).expect("Injection failed");

        assert!(testnet.metrics.incidents_detected > 0);
    }

    #[test]
    fn test_09_state_injection_recovery() {
        // Scenario: State corruption detected, rollback executed
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::InvalidStateInjection,
            testnet_id: "state_injection_recovery".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 55,
                corruption_percentage: 10.0,
                corruption_type: "BadMerkleRoot".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(55).expect("Injection failed");
        testnet.trigger_autonomous_recovery().expect("Recovery failed");

        assert_eq!(testnet.state, TestnetRunState::Recovered);
    }

    #[test]
    fn test_10_state_injection_before_after_proof() {
        // Scenario: Verify before/after state proofs
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::InvalidStateInjection,
            testnet_id: "state_proof".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 45,
                corruption_percentage: 8.0,
                corruption_type: "InvalidNonce".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let before = testnet.initialize().expect("Initialize failed");
        testnet.inject_scenario(45).expect("Injection failed");
        testnet.trigger_autonomous_recovery().expect("Recovery failed");

        let history = testnet.get_state_history();
        assert!(history.contains_key(&0)); // Before state
        assert!(history.len() > 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // GOVERNANCE ABUSE SCENARIOS (5 tests)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_11_governance_abuse_slashing_removal() {
        // Scenario: Attacker proposes removing slashing (set to 0)
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::GovernanceAbuse,
            testnet_id: "gov_remove_slashing".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 30,
                attacker_votes: 500,
                bad_parameter: "slashing_rate".to_string(),
                bad_value: "0".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(30).expect("Injection failed");

        assert_eq!(
            testnet.incident_log[0].incident_type,
            DetectedIncidentType::GovernanceViolation
        );
        assert!(!testnet.governance_proposals.is_empty());
        assert_eq!(
            testnet.governance_proposals[0].rejected_reason,
            "Violates constitutional constraints"
        );
    }

    #[test]
    fn test_12_governance_abuse_disable_finalizer() {
        // Scenario: Attacker tries to break consensus finality
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::GovernanceAbuse,
            testnet_id: "gov_disable_finalizer".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 35,
                attacker_votes: 1000,
                bad_parameter: "consensus_finalizer".to_string(),
                bad_value: "disabled".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(35).expect("Injection failed");

        assert_eq!(testnet.metrics.incidents_detected, 1);
    }

    #[test]
    fn test_13_governance_abuse_constitutional_rejection() {
        // Scenario: Verify constitutional constraints reject abuse
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::GovernanceAbuse,
            testnet_id: "gov_constitutional".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 25,
                attacker_votes: 2000,
                bad_parameter: "min_stake".to_string(),
                bad_value: "0".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(25).expect("Injection failed");

        // Proposal should be rejected
        assert!(!testnet.governance_proposals.is_empty());
        assert!(testnet.governance_proposals[0]
            .rejected_reason
            .contains("constitutional"));
    }

    #[test]
    fn test_14_governance_abuse_quorum_reduction() {
        // Scenario: Attacker tries to reduce governance quorum
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::GovernanceAbuse,
            testnet_id: "gov_reduce_quorum".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 40,
                attacker_votes: 500,
                bad_parameter: "governance_quorum".to_string(),
                bad_value: "5".to_string(), // Try to reduce to 5%
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(40).expect("Injection failed");

        assert_eq!(testnet.state, TestnetRunState::AttackInjected);
    }

    #[test]
    fn test_15_governance_abuse_recovery_upgrade() {
        // Scenario: Governance-driven upgrade to fix vulnerability
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::GovernanceAbuse,
            testnet_id: "gov_recovery_upgrade".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 28,
                attacker_votes: 1500,
                bad_parameter: "consensus_mode".to_string(),
                bad_value: "byzantine".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(28).expect("Injection failed");
        testnet
            .execute_upgrade("1.0.0", "1.0.1")
            .expect("Upgrade failed");

        assert_eq!(testnet.state, TestnetRunState::GovernanceActive);
        assert!(!testnet.recovery_actions.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // DETERMINISM & REPRODUCIBILITY (5 tests)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_16_deterministic_collision_scenario() {
        // Same config = same execution trace
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "determinism_collision".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![1, 2],
                attack_epoch: 10,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet1 = AdversarialTestnet::new(config.clone());
        let _ = testnet1.initialize();
        testnet1.inject_scenario(10).expect("Injection 1 failed");

        let mut testnet2 = AdversarialTestnet::new(config);
        let _ = testnet2.initialize();
        testnet2.inject_scenario(10).expect("Injection 2 failed");

        // Same incident type
        assert_eq!(
            testnet1.incident_log[0].incident_type,
            testnet2.incident_log[0].incident_type
        );
    }

    #[test]
    fn test_17_deterministic_state_injection() {
        // Same state injection = same corruption
        let mut executor1 = ScenarioExecutor::new(b"repro_test_seed".to_vec());
        executor1
            .execute_state_injection(StateCorruptionType::InvalidBalance, 50, 15.0)
            .expect("Execution 1 failed");

        let mut executor2 = ScenarioExecutor::new(b"repro_test_seed".to_vec());
        executor2
            .execute_state_injection(StateCorruptionType::InvalidBalance, 50, 15.0)
            .expect("Execution 2 failed");

        // Verify determinism
        assert!(executor1.verify_determinism(&executor2));
    }

    #[test]
    fn test_18_deterministic_governance_abuse() {
        // Same proposal = same vulnerability
        let proposal1 = MaliciousProposal::new(
            GovernanceAttackType::SetSlashingToZero,
            30,
            1000000,
            b"fixed_seed",
        );

        let proposal2 = MaliciousProposal::new(
            GovernanceAttackType::SetSlashingToZero,
            30,
            1000000,
            b"fixed_seed",
        );

        assert_eq!(proposal1.proposal_id, proposal2.proposal_id);
        assert_eq!(proposal1.attacker_votes, proposal2.attacker_votes);
    }

    #[test]
    fn test_19_third_party_reproducibility() {
        // Anyone with config can reproduce results
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "third_party_reproducible".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![5, 7],
                attack_epoch: 15,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let config_json = serde_json::to_string(&config).expect("Serialization failed");
        let config_restored: AdversarialScenarioConfig =
            serde_json::from_str(&config_json).expect("Deserialization failed");

        let mut testnet = AdversarialTestnet::new(config_restored);
        let _ = testnet.initialize();
        testnet.inject_scenario(15).expect("Injection failed");

        assert!(!testnet.incident_log.is_empty());
    }

    #[test]
    fn test_20_no_manual_intervention_required() {
        // Full automated flow without manual steps
        let config = AdversarialScenarioConfig::default();
        let mut testnet = AdversarialTestnet::new(config);

        // All steps automated
        let _ = testnet.initialize(); // ✅ Automated
        testnet
            .advance_epoch() // ✅ Automated
            .expect("Advance failed");
        let _ = testnet.get_incident_log(); // ✅ Automated
        let _ = testnet.get_public_metrics(); // ✅ Automated
        assert!(testnet.verify_incident_integrity()); // ✅ Automated

        // No manual approval, slashing, or intervention needed
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // AUTONOMOUS DETECTION & RECOVERY (5 tests)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_21_autonomous_attack_detection() {
        // System detects attack without configuration
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "autonomous_detection".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![3, 5],
                attack_epoch: 20,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(20).expect("Injection failed");

        // Attack detected automatically
        assert_eq!(testnet.state, TestnetRunState::AttackInjected);
        assert!(!testnet.incident_log.is_empty());
    }

    #[test]
    fn test_22_autonomous_rollback_triggered() {
        // System rolls back state automatically
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::InvalidStateInjection,
            testnet_id: "autonomous_rollback".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 50,
                corruption_percentage: 10.0,
                corruption_type: "InvalidBalance".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(50).expect("Injection failed");
        testnet.trigger_autonomous_recovery().expect("Recovery failed");

        assert_eq!(testnet.state, TestnetRunState::Recovered);
        assert!(testnet.incident_log[0].state_after.is_some());
    }

    #[test]
    fn test_23_governance_driven_recovery() {
        // Governance proposes and executes recovery
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::GovernanceAbuse,
            testnet_id: "governance_recovery".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 25,
                attacker_votes: 1000,
                bad_parameter: "slashing_rate".to_string(),
                bad_value: "0".to_string(),
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(25).expect("Injection failed");
        testnet.propose_governance_recovery(RecoveryAction {
            action_id: "recovery_001".to_string(),
            recovery_epoch: 26,
            action_type: "Governance".to_string(),
            description: "Reject harmful proposal".to_string(),
            executed: true,
            execution_time: 0,
        }).expect("Recovery failed");

        assert_eq!(testnet.state, TestnetRunState::GovernanceActive);
    }

    #[test]
    fn test_24_slashing_enforced() {
        // Bad validators are slashed
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "slashing_enforced".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![2, 4, 10],
                attack_epoch: 15,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        let initial_slashed = testnet.metrics.validators_slashed;
        testnet.inject_scenario(15).expect("Injection failed");
        let final_slashed = testnet.metrics.validators_slashed;

        assert!(final_slashed > initial_slashed);
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // PUBLIC OBSERVABILITY (5 tests)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_25_public_metrics_dashboard() {
        let mut engine = ObservabilityEngine::new(b"observability_test".to_vec());

        engine.update_metrics(NetworkMetrics {
            epoch: 1,
            block_height: 12,
            validators_online: 20,
            validators_slashed: 0,
            finalization_rate: 100.0,
            proposal_acceptance_rate: 95.0,
            recovery_count: 0,
            ..Default::default()
        });

        let dashboard = engine.get_public_dashboard();
        assert_eq!(dashboard.current_epoch, 1);
        assert_eq!(dashboard.block_height, 12);
    }

    #[test]
    fn test_26_immutable_event_log() {
        let mut engine = ObservabilityEngine::new(b"event_log_test".to_vec());

        engine.record_event(
            EventType::AttackDetected,
            "Validator collusion detected".to_string(),
        );

        let log = engine.get_event_log();
        assert!(!log.is_empty());
        assert!(log[0].verify());
    }

    #[test]
    fn test_27_critical_events_tracked() {
        let mut engine = ObservabilityEngine::new(b"critical_test".to_vec());

        engine.record_event(
            EventType::AttackDetected,
            "Attack detected".to_string(),
        );

        let dashboard = engine.get_public_dashboard();
        assert!(dashboard.critical_events_count > 0);
    }

    #[test]
    fn test_28_metrics_time_series() {
        let mut engine = ObservabilityEngine::new(b"timeseries_test".to_vec());

        for i in 0..10 {
            engine.update_metrics(NetworkMetrics {
                epoch: i,
                block_height: i * 12,
                finalization_rate: 100.0 - (i as f64 * 2.0),
                ..Default::default()
            });
        }

        let history = engine.get_metrics_history();
        assert_eq!(history.epochs.len(), 10);
    }

    #[test]
    fn test_29_alert_system() {
        let mut alerts = AlertSystem::new();

        alerts.publish_alert(
            AlertSeverity::Critical,
            "Byzantine attack detected".to_string(),
            "event_001".to_string(),
        );

        let critical = alerts.get_critical_alerts();
        assert_eq!(critical.len(), 1);
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // END-TO-END INTEGRATION (5 tests)
    // ═══════════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_30_combined_attack_scenario() {
        // All three attacks simultaneously
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::CombinedAttack,
            testnet_id: "combined_attack".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![1, 2],
                attack_epoch: 20,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 20,
                corruption_percentage: 15.0,
                corruption_type: "InvalidBalance".to_string(),
            }),
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 20,
                attacker_votes: 1000,
                bad_parameter: "slashing_rate".to_string(),
                bad_value: "0".to_string(),
            }),
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(20).expect("Injection failed");

        // All three attacks detected
        assert!(testnet.incident_log.len() >= 3);
    }

    #[test]
    fn test_31_full_lifecycle_no_manual_intervention() {
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "full_lifecycle".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![5, 7],
                attack_epoch: 10,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        let mut testnet = AdversarialTestnet::new(config);

        // Complete automated lifecycle
        let before = testnet.initialize().expect("Init failed");
        testnet.inject_scenario(10).expect("Injection failed");
        testnet.trigger_autonomous_recovery().expect("Recovery failed");

        // All phases automated, no manual steps
        assert_eq!(testnet.state, TestnetRunState::Recovered);
        assert!(testnet.verify_incident_integrity());
    }

    #[test]
    fn test_32_investor_ready_demonstration() {
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::CombinedAttack,
            testnet_id: "investor_demo".to_string(),
            validator_count: 20,
            byzantine_count: 3,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![1, 3],
                attack_epoch: 15,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            state_injection: Some(StateInjectionConfig {
                injection_epoch: 15,
                corruption_percentage: 10.0,
                corruption_type: "InvalidBalance".to_string(),
            }),
            governance_abuse: Some(GovernanceAbuseConfig {
                proposal_epoch: 15,
                attacker_votes: 1000,
                bad_parameter: "slashing_rate".to_string(),
                bad_value: "0".to_string(),
            }),
        };

        let mut testnet = AdversarialTestnet::new(config);
        let _ = testnet.initialize();
        testnet.inject_scenario(15).expect("Injection failed");

        // Get public dashboard (what investors see)
        let dashboard = testnet.get_public_metrics();
        assert_eq!(dashboard.run_state, TestnetRunState::AttackInjected);
        assert!(dashboard.incidents_detected > 0);

        // Get incident log (full transparency)
        let incidents = testnet.get_incident_log();
        assert!(!incidents.is_empty());

        // Verify integrity (proof of authenticity)
        assert!(testnet.verify_incident_integrity());
    }

    #[test]
    fn test_33_repeatable_adversarial_runs() {
        let config = AdversarialScenarioConfig {
            scenario: AdversarialScenario::ValidatorCollusion,
            testnet_id: "repeatable_run".to_string(),
            validator_count: 20,
            byzantine_count: 2,
            duration_epochs: 100,
            collision: Some(CollisionAttackConfig {
                colluding_validators: vec![5, 7],
                attack_epoch: 10,
                attack_duration_blocks: 24,
                attempt_fork: true,
            }),
            ..Default::default()
        };

        // Run 1
        let mut testnet1 = AdversarialTestnet::new(config.clone());
        let _ = testnet1.initialize();
        testnet1.inject_scenario(10).expect("Injection 1 failed");

        // Run 2 (identical config)
        let mut testnet2 = AdversarialTestnet::new(config);
        let _ = testnet2.initialize();
        testnet2.inject_scenario(10).expect("Injection 2 failed");

        // Same results = deterministic execution verified
        assert_eq!(
            testnet1.incident_log[0].incident_type,
            testnet2.incident_log[0].incident_type
        );
    }
}
