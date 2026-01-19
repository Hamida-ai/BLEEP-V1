# BLEEP Phase 4 Quick Reference

## Module Locations

| Module | File | Lines | Purpose |
|--------|------|-------|---------|
| Checkpointing | `shard_checkpoint.rs` | 500 | Deterministic shard snapshots |
| Fault Detection | `shard_fault_detection.rs` | 538 | Rule-based fault identification |
| Isolation | `shard_isolation.rs` | 396 | Shard freeze and isolation |
| Rollback | `shard_rollback.rs` | 406 | Atomic state restoration |
| Slashing | `shard_validator_slashing.rs` | 432 | Validator punishment & reassignment |
| Healing | `shard_healing.rs` | 428 | Recovery & reintegration |
| Orchestrator | `phase4_recovery_orchestrator.rs` | 511 | Master recovery coordinator |
| Safety | `phase4_safety_invariants.rs` | 500 | Protocol safety enforcement |
| Tests | `phase4_integration_tests.rs` | 700+ | Comprehensive test suite |

## Key Data Structures

### Recovery Operation
```rust
pub struct RecoveryOperation {
    pub shard_id: ShardId,
    pub start_epoch: EpochId,
    pub trigger_fault: FaultEvidence,
    pub stage: RecoveryStage,
    pub slashed_validators: Vec<Vec<u8>>,
    pub is_finalized: bool,
}
```

### Recovery Stages
```
FaultDetected → Isolated → RollingBack → ValidatorAdjustment 
    → Healing → RecoveryComplete → Reintegrated
```

### Fault Evidence
```rust
pub struct FaultEvidence {
    pub fault_type: FaultType,  // 7 types
    pub shard_id: ShardId,
    pub severity: FaultSeverity,  // Critical, High, Medium, Low
    pub proof: Vec<u8>,  // Cryptographic evidence
}
```

## Critical Operations

### 1. Detect Fault
```rust
let fault = detector.detect_state_root_mismatch(
    shard_id, epoch, height,
    reported_root, quorum_root,
    total_validators, dissenting_count
)?;
```

### 2. Isolate Shard
```rust
orchestrator.initiate_recovery(shard_id, fault)?;
// Automatically:
// - Freezes shard (cannot accept transactions)
// - Blocks cross-shard transactions
// - Creates recovery operation record
```

### 3. Rollback
```rust
orchestrator.execute_rollback(shard_id, current_height)?;
// Automatically:
// - Finds valid checkpoint
// - Aborts cross-shard transactions
// - Releases state locks
// - Restores state
// - Verifies restoration
```

### 4. Slash Validators
```rust
orchestrator.perform_validator_adjustment(
    shard_id, faulty_validators,
    all_validators, num_shards
)?;
// Automatically:
// - Removes faulty validators' stake
// - Disables validators from proposing
// - Reassigns remaining validators
```

### 5. Heal
```rust
orchestrator.begin_healing(shard_id, target_height, root)?;
// Shard:
// - Rebuilds from checkpoint
// - Syncs blocks to current height
// - Verifies state at epochs
// - Becomes ready for reintegration
```

### 6. Reintegrate
```rust
orchestrator.reintegrate_shard(shard_id)?;
// Automatically:
// - Removes from isolated set
// - Re-enables transactions
// - Restores cross-shard participation
```

## Safety Invariants

| Invariant | Enforcement | Status |
|-----------|------------|--------|
| No global rollback | Only shard-level rollback allowed | ✅ |
| No cross-shard corruption | Isolation prevents state propagation | ✅ |
| No silent recovery | All operations auditable | ✅ |
| No unverified reintegration | Full healing required | ✅ |
| Determinism | All decisions rule-based | ✅ |
| Byzantine tolerance | 2f+1 quorum maintained | ✅ |
| No forks | Deterministic state restoration | ✅ |

## Configuration

```rust
// Checkpointing
CheckpointConfig::new(
    blocks_per_checkpoint: 100,
    max_retained_checkpoints: 10,
    max_rollback_depth: 1000,
)?

// Fault Detection
FaultDetectionConfig::new(
    liveness_failure_epochs: 5,
    state_mismatch_threshold_percent: 50,
    min_quorum_size: 4,
)?

// Create Orchestrator
let orchestrator = RecoveryOrchestrator::new(
    checkpoint_config,
    fault_detection_config,
);
```

## Testing

### Run All Phase 4 Tests
```bash
cargo test phase4_ --release
```

### Run Specific Test Category
```bash
# Determinism tests
cargo test deterministic

# Isolation tests
cargo test isolation

# Rollback tests
cargo test rollback

# Validator tests
cargo test validator

# Healing tests
cargo test healing

# Safety tests
cargo test invariant
```

## Monitoring

### Get Recovery Status
```rust
if let Some(recovery) = orchestrator.get_recovery(shard_id) {
    println!("Stage: {:?}", recovery.stage);
    println!("Slashed: {}", recovery.slashed_validators.len());
    println!("Duration: {} seconds", 
        now - recovery.start_timestamp);
}
```

### Get Isolated Shards
```rust
let isolated = isolation_manager.get_isolated_shards();
println!("Isolated shards: {:?}", isolated);
```

### Get Recovery History
```rust
for op in orchestrator.get_recovery_history() {
    println!("{:?}: {:?} -> {:?}", 
        op.shard_id,
        op.start_epoch,
        op.stage);
}
```

## Error Conditions

| Error | Cause | Resolution |
|-------|-------|-----------|
| No valid checkpoint | Fault too far in past | Increase max_rollback_depth |
| Byzantine threshold exceeded | Too many slashed validators | Review slashing criteria |
| Healing stuck | State mismatch at epoch | Investigate executor |
| Fork detected | Determinism violation | Emergency halt |

## Performance

| Operation | Complexity | Time |
|-----------|-----------|------|
| Checkpoint creation | O(1) | <1ms |
| Fault detection | O(n) | <5ms (n=validators) |
| Isolation | O(1) | <1ms |
| Rollback | O(k) | <100ms (k=cross-shard txs) |
| Validator reassignment | O(n log n) | <10ms |
| Healing (per epoch) | O(b) | <100ms (b=blocks) |

## Resources

- **Full Documentation**: [phase4_shard_recovery.md](docs/phase4_shard_recovery.md)
- **Implementation Summary**: [PHASE4_IMPLEMENTATION.md](PHASE4_IMPLEMENTATION.md)
- **Source Code**: `crates/bleep-state/src/phase4_*.rs`
- **Tests**: `crates/bleep-state/src/phase4_integration_tests.rs`

## Key Takeaways

1. **Shard failures are isolated** - only that shard stops accepting transactions
2. **Network continues operating** - other shards process normally
3. **Recovery is deterministic** - all nodes compute identical recovery plan
4. **No manual intervention needed** - recovery proceeds automatically
5. **Byzantine validators are identified and removed** - provably faulty validators slashed
6. **Complete audit trail** - every operation recorded and verifiable
7. **Time-bounded** - recovery completes within N epochs
8. **Fork prevention** - impossible to create conflicting states

---

**Quick Start**: Copy-paste the key operations section above into your consensus layer integration code.
