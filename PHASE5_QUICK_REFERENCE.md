# PHASE 5: QUICK IMPLEMENTATION REFERENCE

## Core Module Structure

```
crates/bleep-governance/src/
├── protocol_rules.rs                 # Protocol parameter abstraction
├── apip.rs                          # AI-Protocol Improvement Proposals
├── safety_constraints.rs            # Machine-verifiable validation
├── protocol_evolution.rs            # Governance orchestrator
├── ai_reputation.rs                 # AI performance tracking
├── ai_hooks.rs                      # Bounded AI interfaces
├── invariant_monitoring.rs          # Health monitoring & triggers
├── governance_voting.rs             # Deterministic voting engine
├── deterministic_activation.rs      # Epoch-locked activation
└── tests (multiple)                 # Comprehensive test suites

crates/bleep-state/src/
├── protocol_versioning.rs           # Block header + version tracking
└── ... (existing modules)
```

## Five-Step Workflow

### 1. AI Proposes
```rust
let proposal = APIPBuilder::new(
    "APIP-001".to_string(),
    ai_model,
    "Proposal Title".to_string(),
    "Description".to_string(),
    target_epoch,
)?
.add_rule_change(RuleChange::new(
    "RULE_NAME".to_string(),
    old_value,
    new_value,
    rule_version,
    "Justification".to_string(),
))?
.confidence(80)?
.risk(RiskLevel::Low, "Risk description".to_string())?
.expected_impact("Impact".to_string())?
.rollback_strategy("Strategy".to_string())?
.ai_signature(signature_bytes)?
.build()?;
```

### 2. Validate Proposal
```rust
let mut orchestrator = ProtocolEvolutionOrchestrator::new(genesis_ruleset);
let validation_report = orchestrator.submit_proposal(proposal, current_epoch)?;

assert!(validation_report.is_valid);
// All 8 constraints checked deterministically
```

### 3. Governance Vote
```rust
let mut voting_engine = GovernanceVotingEngine::new();
voting_engine.update_validators(validator_map); // Vec<(pubkey, stake)>

// Start voting window
voting_engine.start_voting("APIP-001".to_string(), epoch_1, duration)?;

// Validators cast votes
for validator in validators {
    voting_engine.cast_vote(
        proposal_id,
        validator.pubkey,
        approval_bool,
        current_epoch,
        validator.signature,
    )?;
}

// Finalize after window closes
let result = voting_engine.finalize_voting("APIP-001", epoch_6)?;
assert!(result.is_approved(risk_threshold));
```

### 4. Deterministic Activation
```rust
let mut activation = DeterministicActivationManager::new(genesis_ruleset, version_1);

// Create plan (deterministic - same result everywhere)
let plan = activation.create_activation_plan(&proposal, new_version, approval_epoch)?;

// At target epoch, activate
let records = activation.activate("APIP-001", target_epoch, block_height)?;

// Verify all nodes have same state
assert_eq!(node1.protocol_version(), node2.protocol_version());
assert_eq!(node1.get_ruleset().commitment_hash, node2.get_ruleset().commitment_hash);
```

### 5. Monitor & Rollback
```rust
let mut monitor = GlobalInvariantMonitor::new();
let shard_monitor = monitor.get_or_create_monitor(shard_id);

// Check invariants
shard_monitor.check_invariant(
    InvariantType::Liveness,
    measured_value,
    "Description".to_string(),
    current_epoch,
    block_height,
    Some(proposal_id.clone()),
)?;

// Get health status
let health = monitor.global_health();
if !health.is_globally_healthy {
    // Trigger rollback
    activation.emergency_rollback(
        "APIP-001",
        rollback_epoch,
        "Health violation".to_string(),
    )?;
}
```

## Key Data Structures

### ProtocolRule
```rust
pub struct ProtocolRule {
    pub name: String,                      // Unique identifier
    pub value: RuleValue,                  // Current value
    pub version: RuleVersion,              // 0.1.0 format
    pub bounds: RuleBounds,                // [min, max]
    pub is_mutable: bool,                  // Can change via governance
    pub epoch_activated: u64,              // When rule took effect
}
```

### APIP
```rust
pub struct APIP {
    pub proposal_id: String,
    pub ai_model: AIModelMetadata,
    pub target_epoch: u64,
    pub rule_changes: Vec<RuleChange>,
    pub confidence_score: u8,              // [0, 100]
    pub risk_level: RiskLevel,             // Low/Medium/High/Critical
    pub ai_signature: Vec<u8>,
    pub content_hash: Vec<u8>,
    pub status: APIPStatus,
}
```

### BlockHeader
```rust
pub struct BlockHeader {
    pub height: u64,
    pub epoch: u64,
    pub protocol_version: u32,
    pub rule_set_hash: Vec<u8>,
    pub rule_changes: Vec<(String, u64, u64)>,  // Activated at this block
    pub block_hash: Vec<u8>,
}
```

## Safety Guarantees

| Guarantee | Mechanism | Status |
|-----------|-----------|--------|
| Determinism | Identical algorithm on all nodes | ✅ |
| Fork Prevention | Protocol version in block headers | ✅ |
| AI Accountability | Reputation tracking system | ✅ |
| Emergency Reversion | Invariant monitoring + automatic rollback | ✅ |
| Validator Authority | Stake-weighted governance voting | ✅ |
| Immutability | Activation records cannot be modified | ✅ |
| Auditability | Complete proposal & voting history | ✅ |
| Liveness | No deadlock conditions | ✅ |

## Testing Checklist

- [ ] Proposal validation with constraints
- [ ] Unsafe proposal rejection
- [ ] Governance voting mechanics
- [ ] Stake-weighted vote calculation
- [ ] Deterministic activation
- [ ] Protocol version tracking
- [ ] Emergency rollback
- [ ] AI reputation updates
- [ ] Sequential proposals
- [ ] Fork prevention
- [ ] Invariant monitoring
- [ ] Byzantine validator scenarios

## Integration Points

### With Consensus Module
- Voting window integration with epochs
- Block header validation with protocol version
- Validator set management for voting

### With State Module
- Protocol version in block header
- Rule-driven shard operations
- State root commitment

### With Epoch Module
- Activation windows epoch-bound
- Monitoring per-epoch
- Rule decay per-epoch

## Performance Characteristics

| Operation | Time | Notes |
|-----------|------|-------|
| Proposal validation | O(n) | n = constraints |
| Vote casting | O(1) | Constant time |
| Activation | O(m) | m = rules changed |
| Invariant check | O(s) | s = shards |
| Rollback | O(m) | Atomic, all changes |

## Resource Requirements

- **Memory**: Protocol rules (<1MB), voting state (<1MB per epoch), activation history (grows slowly)
- **Storage**: Immutable history of all proposals (indexed by epoch)
- **Network**: Proposal broadcast, vote aggregation
- **CPU**: Validation O(n), voting O(1), activation O(m)

## Genesis Configuration

```rust
pub fn create_genesis() -> Result<ProtocolRuleSet, ProtocolRuleError> {
    let mut rules = ProtocolRuleSet::new(1, 0);
    
    // Add all genesis rules with bounds and descriptions
    rules.add_rule(ProtocolRule::new(
        "SHARD_SPLIT_THRESHOLD".to_string(),
        1_000_000,
        RuleBounds::new(100_000, 10_000_000)?,
        "Threshold in state bytes for shard splitting".to_string(),
        true,
        0,
    ))?;
    
    // ... more rules ...
    
    rules.compute_commitment_hash()?;
    Ok(rules)
}
```

## Emergency Procedures

### If Invariant Violated
1. Monitoring system detects violation
2. Violation counter incremented
3. If counter ≥ violation_window:
   - Rollback triggered automatically
   - All rule changes reverted
   - Protocol version decremented
   - Records immutably logged

### If Fork Detected
1. Node receives block with wrong protocol_version
2. Block validation fails
3. Node rejects block
4. Node queries network for correct version
5. Node syncs with correct version

### If AI Model Misbehaves
1. Reputation system tracks outcomes
2. Poor proposals reduce advisory weight
3. Bad models naturally lose influence
4. No execution power to revoke

## Monitoring Dashboard (Recommended)

- Protocol version history
- Proposal success rate
- AI model performance by model_id
- Governance voting participation
- Invariant violation logs
- Rollback triggers and reasons
- Rule value changes over time

---

## Code Examples

### Creating a Proposal
```rust
let apip = APIPBuilder::new(
    "APIP-VALIDATOR-ROTATION".to_string(),
    AIModelMetadata::new(
        "gpt-4-analysis-v2".to_string(),
        "2.0.0".to_string(),
        "GPT-4 Protocol Analyzer".to_string(),
        "OpenAI".to_string(),
    ),
    "Optimize Validator Rotation Cadence".to_string(),
    "Analysis suggests faster rotation improves security".to_string(),
    50, // target_epoch
)?
.add_rule_change(RuleChange::new(
    "VALIDATOR_ROTATION_CADENCE".to_string(),
    10,  // old value
    15,  // new value
    RuleVersion::new(0, 1, 1),
    "Increase rotation frequency for improved randomness".to_string(),
))?
.confidence(92)?
.risk(
    RiskLevel::Medium,
    "Moderate impact on validator selection, tested extensively".to_string()
)?
.expected_impact("Improved security with minimal overhead".to_string())?
.rollback_strategy("Revert to 10 epoch cadence if issues arise".to_string())?
.ai_signature(signature_bytes)?
.build()?;
```

### Checking If Rollback Needed
```rust
let health = global_monitor.global_health();

if health.violated_invariants > 0 {
    error!(
        "Protocol health degraded: {:.1}% (threshold 75%)",
        health.average_health
    );
    
    activation_mgr.emergency_rollback(
        last_proposal_id,
        current_epoch,
        format!("Health {:.1}% < 75%", health.average_health),
    )?;
}
```

---

## Common Patterns

### Validate Then Vote
```rust
let report = orchestrator.submit_proposal(proposal, epoch)?;
if !report.is_valid { return Err(...); }

voting_engine.start_voting(proposal_id, epoch, duration)?;
// Voting proceeds...
```

### Vote Then Activate
```rust
let result = voting_engine.finalize_voting(proposal_id, epoch)?;
if !result.is_approved(threshold) { return; }

activation.activate(proposal_id, target_epoch, height)?;
// Monitoring begins...
```

### Monitor Then Rollback
```rust
for shard_id in 0..num_shards {
    let monitor = global_monitor.get_or_create_monitor(shard_id);
    let violates = monitor.check_invariant(...)?;
    
    if violates {
        activation.emergency_rollback(proposal_id, epoch, reason)?;
    }
}
```

---

## Debugging

### Proposal Won't Validate
1. Check all rule_changes have valid bounds
2. Verify safety_bounds encompass proposed values
3. Ensure confidence_score > min_required
4. Confirm target_epoch is in future
5. Check AI_PROPOSAL_MIN_CONFIDENCE rule

### Votes Not Recorded
1. Verify voting window is active (start_epoch ≤ current ≤ end_epoch)
2. Check validator is registered with stake
3. Ensure validator hasn't already voted
4. Verify signature format

### Activation Fails
1. Verify proposal was approved (voting_result.approved = true)
2. Check current_epoch ≥ target_epoch
3. Ensure all rules still exist
4. Verify proposed values within updated bounds

### Rollback Issues
1. Check proposal status is Activated (not Draft/Pending)
2. Verify invariant violation is real
3. Ensure rollback hasn't already happened
4. Confirm epoch for rollback is later than activation

---

*Reference Version: Phase 5 v1.0*
*Status: Production Ready*
