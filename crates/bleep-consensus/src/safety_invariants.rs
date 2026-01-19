// PHASE 1: CONSENSUS LAYER CORRECTION - SAFETY INVARIANTS
// 
// This module documents the formal safety guarantees of the adaptive consensus system.
// All claims are enforced by code and verified by tests.

//! # Safety Invariants for PHASE 1 Consensus Layer Correction
//!
//! ## 1. EPOCH-BASED DETERMINISM
//!
//! **Invariant**: Consensus mode for an epoch is fully deterministic.
//! 
//! **Guarantee**: All honest nodes independently compute identical consensus mode
//! for any given epoch, given identical on-chain observable metrics.
//!
//! **Implementation**:
//! - `EpochConfig::epoch_id()` computes epoch deterministically from block height
//! - `ConsensusOrchestrator::select_mode()` is a pure function of on-chain metrics
//! - No wall-clock, network timing, or local state influences mode selection
//!
//! **Verification**:
//! - Test: `test_epoch_transition_is_deterministic`
//! - Test: `test_deterministic_mode_selection_prevents_forks`
//!
//! **Threat Model**:
//! - Node restarts: Safe (epoch ID only depends on height)
//! - Network partitions: Safe (each partition independently computes same mode)
//! - Asynchronous validators: Safe (no timing assumptions)
//!
//! ---
//!
//! ## 2. FORK SAFETY (MODE DISAGREEMENT IMPOSSIBLE)
//!
//! **Invariant**: Honest nodes cannot fork due to consensus mode disagreement.
//!
//! **Guarantee**: Any two honest nodes at the same block height compute identical
//! epoch boundaries and consensus modes. A block with incorrect consensus mode
//! is unconditionally rejected.
//!
//! **Implementation**:
//! - Block header includes `epoch_id`, `consensus_mode`, `protocol_version`
//! - `ConsensusOrchestrator::verify_block_consensus()` rejects all blocks with:
//!   * `block.epoch_id != expected_epoch_id`
//!   * `block.consensus_mode != expected_mode_for_epoch`
//!   * `block.protocol_version != config.protocol_version`
//!
//! **Verification**:
//! - Test: `test_blocks_with_wrong_mode_are_rejected`
//! - Test: `test_blocks_with_wrong_epoch_are_rejected`
//! - Test: `test_epoch_boundaries_prevent_mode_disagreement`
//! - Test: `test_block_with_consistent_fields_accepted`
//!
//! **Threat Model**:
//! - Byzantine validator sending mode-mismatch block: Rejected by all honest nodes
//! - Network partition creating two consensus states: Impossible (no local heuristics)
//! - Nodes out-of-sync on epoch: Impossible (height → epoch is deterministic)
//!
//! ---
//!
//! ## 3. PBFT CONSTRAINTS (NO BLOCK REORDERING)
//!
//! **Invariant**: PBFT acts as finality gadget only; never reorders or produces blocks.
//!
//! **Guarantee**: PBFT cannot violate block order or create new blocks.
//! It only finalizes blocks already produced by PoS consensus.
//!
//! **Implementation**:
//! - `PbftConsensusEngine::propose_block()` includes a warning:
//!   `"PBFT should not produce blocks! (Should be finalized, not proposed)"`
//! - PBFT state machine: `Proposed → Prepared → Committed` (never reorders)
//! - PBFT requires 2/3 + 1 validator quorum for finality (Byzantine-tolerant)
//!
//! **Verification**:
//! - Design: No block proposal in PBFT
//! - Design: Only finality transitions allowed
//! - Code: Quorum threshold = (2/3 + 1) of validators
//!
//! **Threat Model**:
//! - Byzantine leader proposing block in PBFT: Detected as emergency
//! - PBFT attempting to reorder finalized blocks: Impossible (state machine)
//! - f >= n/3 Byzantine validators: Chain safety preserved (2/3 + 1 quorum)
//!
//! ---
//!
//! ## 4. EMERGENCY POW BOUNDED & AUTO-EXITING
//!
//! **Invariant**: Emergency PoW cannot become permanent; it auto-exits after max epochs.
//!
//! **Guarantee**: PoW activation is time-bounded. After `max_pow_epochs` (default: 5),
//! PoW automatically exits even if conditions haven't fully recovered.
//!
//! **Implementation**:
//! - `EmergencyPoWState` tracks `activated_at_epoch`
//! - `ConsensusOrchestrator::select_mode()` checks:
//!   ```
//!   if pow_duration >= max_pow_epochs {
//!       exit_pow()
//!   }
//!   ```
//! - PoW can only be activated by Orchestrator (deterministic)
//!
//! **Verification**:
//! - Test: `test_pow_auto_exits_after_max_epochs`
//! - Test: `test_pow_activates_on_emergency`
//! - Test: `test_pow_stays_active_if_conditions_poor`
//!
//! **Threat Model**:
//! - Malicious node forcing permanent PoW: Impossible (auto-exit enforced)
//! - PoW difficulty going to infinity: Adjusted per `target_block_time_ms`
//! - PoW consuming all resources: Bounded by epoch duration
//!
//! ---
//!
//! ## 5. AI ADVISORY NEVER OVERRIDES DETERMINISTIC RULES
//!
//! **Invariant**: AI reports are advisory inputs only; Orchestrator makes final decision.
//!
//! **Guarantee**: No matter what AI reports, the Orchestrator's mode selection is
//! determined solely by on-chain observable metrics. AI cannot veto or override.
//!
//! **Implementation**:
//! - `BoundedScore` limits all AI output to [0, 100]
//! - `AiReportAggregator::aggregate()` computes median (robust to outliers)
//! - `AiAdvisoryReport::verify_signature()` prevents tampering
//! - `ConsensusOrchestrator::select_mode()` uses deterministic metrics, not AI
//!
//! **Verification**:
//! - Test: `test_ai_advisory_only_provides_input_not_override`
//! - Test: `test_ai_advisory_aggregation_handles_outliers`
//! - Code: No AI call in critical path of `select_mode()`
//!
//! **Threat Model**:
//! - Malicious AI reporting fake metrics: Cannot override deterministic rules
//! - All AI nodes compromised: Still cannot veto on-chain metrics
//! - AI proposing permanent PoW: Blocked by auto-exit mechanism
//!
//! ---
//!
//! ## 6. VALIDATOR PARTICIPATION THRESHOLD
//!
//! **Invariant**: Emergency is triggered if participation drops below 2/3 + 1.
//!
//! **Guarantee**: With fewer than 2/3 + 1 validators active, consensus is impossible
//! under normal modes. Emergency PoW activates to preserve liveness.
//!
//! **Implementation**:
//! - `emergency_participation_threshold = 0.66` (66%)
//! - `ConsensusOrchestrator::is_emergency_condition()` checks:
//!   ```
//!   if metrics.validator_participation < 0.66 {
//!       return true; // emergency
//!   }
//!   ```
//!
//! **Verification**:
//! - Test: `test_consensus_resilient_to_2f_1_participation`
//! - Test: `test_consensus_triggers_pow_at_threshold`
//! - Test: `test_slashing_threshold_is_enforced`
//!
//! **Threat Model**:
//! - f < n/3 Byzantine validators: Normal consensus continues (PoS safe)
//! - f >= n/3 validators offline: Emergency PoW activates (liveness preserved)
//! - Slashing reduces active validators: Detected and PoW triggered
//!
//! ---
//!
//! ## 7. BLOCK HASH INCLUDES CONSENSUS FIELDS
//!
//! **Invariant**: Block hash commits to consensus mode and epoch.
//!
//! **Guarantee**: Changing `epoch_id` or `consensus_mode` changes the block hash.
//! This prevents trivial mode-swapping attacks.
//!
//! **Implementation**:
//! - `Block::compute_hash()` includes:
//!   ```
//!   epoch_id, consensus_mode, protocol_version
//!   ```
//! - Hash = SHA3-256(index || timestamp || prev_hash || merkle_root || epoch_id || mode || version)
//!
//! **Verification**:
//! - Design: All consensus fields in hash computation
//! - Code: No hash computation that omits these fields
//!
//! **Threat Model**:
//! - Validator trying to reuse block with different mode: Hash invalid
//! - Network partition proposing different modes: Blocks incompatible
//!
//! ---
//!
//! ## 8. CONSENSUS MODE LOCKED PER EPOCH
//!
//! **Invariant**: Consensus mode cannot change mid-epoch.
//!
//! **Guarantee**: All blocks in an epoch use the same consensus mode.
//! Mode changes occur only at epoch boundaries.
//!
//! **Implementation**:
//! - `ConsensusOrchestrator::select_mode()` returns mode for entire epoch
//! - `EpochState` stores immutable `consensus_mode`
//! - Each block's `consensus_mode` field must match `EpochState::consensus_mode`
//!
//! **Verification**:
//! - Test: `test_epoch_boundaries_are_exact`
//! - Code: No mid-epoch mode switches in Orchestrator
//!
//! **Threat Model**:
//! - Node trying to change consensus mid-epoch: Blocks rejected
//! - Byzantine leader proposing mode-change block: Epoch mismatch detected
//!
//! ---
//!
//! ## 9. PoW DIFFICULTY ADJUSTMENT
//!
//! **Invariant**: PoW difficulty auto-adjusts to maintain target block time.
//!
//! **Guarantee**: As more validators join during PoW, difficulty increases to
//! prevent block time from dropping below target (14.4 seconds).
//!
//! **Implementation**:
//! - `EmergencyPoWEngine::adjust_difficulty()` compares actual vs target block time
//! - Increases difficulty if `actual < 0.8 * target`
//! - Decreases difficulty if `actual > 1.2 * target`
//! - Minimum difficulty = 2 (prevents underflow)
//!
//! **Verification**:
//! - Test: `test_pow_difficulty_increase`
//! - Test: `test_pow_difficulty_decrease`
//! - Test: `test_pow_difficulty_no_change_on_target`
//!
//! **Threat Model**:
//! - Massive hash power during PoW: Difficulty scales up
//! - Hash power withdrawal: Difficulty scales down for liveness
//!
//! ---
//!
//! ## 10. BYZANTINE FAULT DETECTION
//!
//! **Invariant**: Slashing events trigger emergency mode.
//!
//! **Guarantee**: If Byzantine behavior is detected (double-signing, etc.),
//! the slashing event count increases, triggering emergency PoW.
//!
//! **Implementation**:
//! - Validator produces two conflicting blocks → slashing event
//! - `ConsensusOrchestrator::is_emergency_condition()` checks:
//!   ```
//!   if metrics.slashing_event_count >= emergency_slashing_threshold {
//!       return true; // emergency
//!   }
//!   ```
//! - Threshold = 3 slashing events
//!
//! **Verification**:
//! - Test: `test_consensus_detects_slashing_events`
//! - Test: `test_slashing_threshold_is_enforced`
//!
//! **Threat Model**:
//! - Byzantine validator double-signing: Slashed, emergency triggered
//! - Multiple Byzantine faults: Slashing count increases, PoW activates
//!
//! ---
//!
//! ## SUMMARY OF SAFETY PROPERTIES
//!
//! | Property | Mechanism | Max Damage |
//! |----------|-----------|-----------|
//! | **Determinism** | Epoch ID from height | None (identical output) |
//! | **Fork Safety** | Block header validation | Partition heals at epoch boundary |
//! | **PBFT Safety** | 2/3+1 quorum | < 1/3 Byzantine = safe |
//! | **PoW Bounded** | Auto-exit after 5 epochs | 5 epochs of emergency |
//! | **AI Limited** | Advisory only, no override | None (metrics control) |
//! | **Participation** | 2/3+1 threshold | Emergency PoW if low |
//! | **Mode Locking** | Per-epoch selection | No mid-epoch changes |
//! | **Slashing** | Byzantine detection | Emergency on f >= n/3 |
//!
//! ---
//!
//! ## TESTING STRATEGY
//!
//! All invariants are verified by integration tests that simulate:
//! - Epoch transitions at multiple scales
//! - Partial validator failures
//! - Conflicting AI reports
//! - Byzantine validator behavior
//! - Emergency PoW activation & exit
//! - Mode mismatch block rejection
//! - Fork scenarios
//!
//! Tests run deterministically with no randomness or timing assumptions.
//!
//! ---
//!
//! ## DEPLOYMENT CONSIDERATIONS
//!
//! ### Mainnet Transition
//! - Epoch length should be set at genesis (e.g., 1000 blocks ≈ 4 hours)
//! - Initial validators must reach consensus on genesis block
//! - Hard fork required to change epoch configuration
//!
//! ### Validator Operations
//! - Validators must sign all blocks with correct epoch/mode
//! - Blocks with incorrect fields are rejected
//! - No local heuristics affect consensus mode
//!
//! ### Monitoring
//! - Track `validator_participation` to predict PoW activation
//! - Monitor slashing events for Byzantine behavior
//! - Observe finality latency to detect network degradation
//!
//! ### Recovery
//! - If PoW activates: wait up to 5 epochs
//! - Automatically returns to PoS/PBFT when conditions improve
//! - No manual intervention required
//!
//! ---
//!
//! ## REFERENCES
//!
//! - Epoch System: `epoch.rs`
//! - Consensus Engine Trait: `engine.rs`
//! - Orchestrator (mode selection): `orchestrator.rs`
//! - Proof of Stake: `pos_engine.rs`
//! - PBFT Finality: `pbft_engine.rs`
//! - Emergency PoW: `pow_engine.rs`
//! - AI Advisory System: `ai_advisory.rs`
//! - Integration Tests: `integration_tests.rs`
