# PHASE 5: GOVERNANCE OPERATIONS GUIDE
## How to Operate the AI-Driven Protocol Evolution System

---

## QUICK START

### For AI Models: How to Propose a Change

```rust
// 1. Create proposal
let proposal = APIPBuilder::new(
    "APIP-2026-001",
    AIModelMetadata::new(
        "gpt-4-analyzer",
        "2.1.0",
        "GPT-4 Protocol Analyzer",
        "OpenAI",
    ),
    "Optimize Checkpoint Frequency",
    "Analysis shows current frequency is suboptimal".to_string(),
    100, // target epoch
)?
.add_rule_change(RuleChange::new(
    "CHECKPOINT_FREQUENCY",
    100,   // current value
    120,   // proposed value
    RuleVersion::new(1, 0, 0),
    "Reduce overhead while maintaining safety".to_string(),
))?
.confidence(85)?
.risk(RiskLevel::Low, "Tested extensively, low risk".to_string())?
.expected_impact("10% reduction in checkpoint overhead".to_string())?
.rollback_strategy("Revert to 100 block checkpoint frequency".to_string())?
.ai_signature(sign_proposal())?
.build()?;

// 2. Submit on-chain
let orchestrator = ProtocolEvolutionOrchestrator::new(ruleset);
let validation = orchestrator.submit_proposal(proposal, current_epoch)?;

// 3. Check if valid
if validation.is_valid {
    println!("Proposal accepted for governance voting!");
} else {
    println!("Proposal rejected: {:?}", validation.failed_checks);
}
```

---

### For Validators: How to Vote

```rust
// 1. Setup voting engine
let mut voting_engine = GovernanceVotingEngine::new();
voting_engine.update_validators(get_validator_set());

// 2. Vote on proposal
voting_engine.start_voting("APIP-2026-001".to_string(), epoch, duration)?;

// During voting window:
for validator in my_validators {
    voting_engine.cast_vote(
        "APIP-2026-001".to_string(),
        validator.public_key,
        true,  // or false to reject
        current_epoch,
        sign_vote(),
    )?;
}

// 3. Finalize voting
let result = voting_engine.finalize_voting("APIP-2026-001", epoch + duration)?;

// 4. Check result
if result.is_approved(67) {
    println!("Proposal approved: {:.1}%", result.approval_percentage);
} else {
    println!("Proposal rejected: {:.1}%", result.approval_percentage);
}
```

---

### For Operators: How to Activate Changes

```rust
// 1. Setup activation manager
let genesis_ruleset = ProtocolRuleSetFactory::create_genesis()?;
let mut activation = DeterministicActivationManager::new(genesis_ruleset, 1);

// 2. Create activation plan
let plan = activation.create_activation_plan(
    &approved_proposal,
    next_protocol_version,
    approval_epoch,
)?;

// 3. At target epoch, activate
if activation.check_activation_ready("APIP-2026-001", current_epoch) {
    let records = activation.activate(
        "APIP-2026-001",
        current_epoch,
        current_block_height,
    )?;
    
    println!("Activated {} rule changes", records.len());
}

// 4. Monitor health
let health = activation.check_invariants(shard_id, "APIP-2026-001")?;
if health {
    // Rollback was triggered
    println!("Proposal rolled back due to invariant violation");
}
```

---

## DETAILED WORKFLOWS

### Workflow 1: Complete Proposal Lifecycle

**Epochs 0-4: Preparation**
```
Epoch 0: AI analyzes chain metrics
         - Identifies optimization opportunity
         - Designs proposal
         - Configures safety bounds

Epoch 1: Proposal submitted on-chain
         - Content hash computed
         - Validation engine runs 8 checks
         - All validators receive proposal

Epoch 2: Validators review proposal
         - Check historical data
         - Simulate impact
         - Prepare positions
```

**Epochs 5-14: Governance**
```
Epoch 5: Voting window opens
         - Duration: 10 epochs
         - All validators can vote
         - Votes weighted by stake

Epochs 6-14: Voting occurs
            - Validators cast weighted votes
            - Double voting prevented
            - Participation tracked

Epoch 15: Voting finalizes
          - Result: approval_percentage = X%
          - Thresoldcompare: X% >= risk_threshold?
          - Status: APPROVED or REJECTED
```

**Epochs 20-39: Activation & Monitoring**
```
Epoch 20: All rules activated
          - All nodes apply changes
          - Protocol version incremented
          - New rules in effect

Epochs 21-39: Continuous monitoring
             - Invariants checked every block
             - Violations logged
             - Health tracked
             - AI reputation updated
```

**Epoch 40+: Completion**
```
If healthy:
  - Status: ACTIVATED
  - AI reputation: +points
  - System continues

If violation detected:
  - Emergency rollback triggered
  - All changes reverted
  - Status: ROLLED_BACK
  - AI reputation: -points
```

---

### Workflow 2: Emergency Rollback

**Trigger: Invariant Violation**
```
Block N-1: Liveness = 95% ✅
Block N: Liveness = 40% ❌ (< 80% threshold)

InvariantMonitor.check():
  - violation_count = 1
  - violation_window = 2
  - Continue monitoring

Block N+1: Liveness = 50% ❌
InvariantMonitor.check():
  - violation_count = 2
  - violation_count >= violation_window
  - TRIGGER ROLLBACK

Activation.emergency_rollback():
  - Revert all rule changes
  - Restore previous values
  - Decrement protocol version
  - Create immutable record
  - Status: ROLLED_BACK
  - AI reputation: Heavy penalty
```

---

### Workflow 3: Multi-Proposal Sequencing

**Scenario: Two proposals in different target epochs**

```
Proposal A:
  - target_epoch: 50
  - Status at epoch 50: ACTIVATE
  - Status after: ACTIVATED

Proposal B:
  - target_epoch: 100
  - Depends on Proposal A's rules
  - Status at epoch 50: PENDING
  - Status at epoch 100: ACTIVATE
  - Status after: ACTIVATED
```

**Safety**: All rules validated against current state at submission time.

---

## OPERATIONAL PARAMETERS

### Voting Configuration
```rust
pub struct VotingConfiguration {
    pub min_voting_duration: u64,      // epochs
    pub max_voting_duration: u64,      // epochs
    pub max_concurrent_proposals: u64, // limit
    pub min_proposal_interval: u64,    // epochs
}

// Recommended Genesis Values:
// min_voting_duration: 5
// max_voting_duration: 20
// max_concurrent_proposals: 3
// min_proposal_interval: 1
```

### Invariant Thresholds
```rust
pub struct InvariantConfiguration {
    // Liveness
    liveness_threshold: 80.0,
    liveness_violation_window: 2,
    
    // Finality
    finality_threshold: 66.0,
    finality_violation_window: 1,
    
    // Validator Health
    health_threshold: 75.0,
    health_violation_window: 3,
    
    // AI Performance
    ai_threshold: 50.0,
    ai_violation_window: 5,
}

// These are governance rules - can be changed via A-PIPs
```

### Risk Thresholds
```rust
pub enum RiskLevel {
    Low:      51,      // Simple majority
    Medium:   67,      // 2/3 majority
    High:     80,      // Supermajority
    Critical: 90,      // Consensus needed
}

// Risk classification should match actual impact:
// - Parameter tuning = Low
// - Algorithm changes = Medium
// - Validator mechanisms = High
// - Consensus core = Critical
```

---

## MONITORING DASHBOARD (RECOMMENDED)

### Key Metrics

**Proposal Activity**
```
- Total proposals submitted: N
- Proposals approved: N/M (X%)
- Proposals rejected: M/N (Y%)
- Average time to voting: Z epochs
- Average time to activation: W epochs
```

**Governance Participation**
```
- Average validator participation: P%
- Average stake participation: Q%
- Voting power concentration (Herfindahl): H
- Double voting attempts: 0
```

**AI Performance**
```
- AI models: count
- Avg proposal acceptance rate: X%
- Best performing model: name (Y% acceptance)
- Worst performing model: name (Z% acceptance)
- Model reputation scores: [list]
```

**Protocol Health**
```
- Current protocol version: N
- Rule changes per epoch: avg X
- Rollbacks triggered: count
- Invariant violations: count
- Emergency activations: count
```

**System Stability**
```
- Finality rate: X%
- Liveness: X%
- State coherence: X%
- Validator health: X%
- Cross-shard consistency: X%
```

---

## EMERGENCY PROCEDURES

### Procedure 1: Detect Critical Invariant Violation

**When to Activate**
- Any FATAL invariant violated
- Multiple invariants simultaneously violated
- Persistent violation over violation_window

**Steps**
1. Invariant monitoring system detects violation
2. Automatic alert triggered
3. Rollback initiated by orchestrator
4. All rule changes reverted
5. Protocol version decremented
6. Immutable record created
7. System continues operation

**No manual intervention required** - system is designed to heal automatically.

---

### Procedure 2: Governance Emergency Vote

**When to Activate**
- Rollback failed (should never happen)
- Protocol in critical state
- Consensus agreement needed on emergency action

**Steps**
1. Governance committee initiates emergency proposal
2. Voting window reduced to 1 epoch
3. Supermajority (90%+) required
4. Special emergency status tracked
5. Action taken immediately
6. Full history recorded

---

### Procedure 3: Network Resync After Divergence

**If Minority Fork Detected**

```
Minority Node (protocol_version = 1):
  Receives block: protocol_version = 2
  ↓
  Validation fails
  ↓
  Requests version history
  ↓
  Downloads ruleset with protocol_version = 2
  ↓
  Validates all rule changes
  ↓
  Updates local state
  ↓
  Re-joins consensus
```

---

## GOVERNANCE BEST PRACTICES

### For AI Models
1. **Analyze thoroughly before proposing**
   - Use deterministic simulation
   - Test against historical data
   - Calculate worst-case impact

2. **Be conservative initially**
   - Start with Low-risk proposals
   - Build reputation with good outcomes
   - Increase impact as trust grows

3. **Be transparent**
   - Explain reasoning clearly
   - Provide evidence
   - Be honest about confidence

4. **Monitor your proposals**
   - Track invariants post-activation
   - Be prepared to recommend rollback
   - Learn from outcomes

### For Validators
1. **Vote informed**
   - Review safety validation results
   - Check AI reputation
   - Consider ecosystem impact

2. **Participate regularly**
   - Voting participation affects health
   - Missing votes has consequences
   - Supermajority required for critical changes

3. **Monitor activated changes**
   - Watch for invariant violations
   - Report suspicious behavior
   - Help detect edge cases

4. **Maintain protocol integrity**
   - Reject invalid proposals
   - Support emergency rollbacks
   - Prioritize safety over speed

### For Operators
1. **Monitor continuously**
   - Track all invariants
   - Watch proposal lifecycle
   - Alert on anomalies

2. **Maintain infrastructure**
   - Keep versioning accurate
   - Log all changes
   - Enable reproducibility

3. **Prepare for emergencies**
   - Have rollback procedures ready
   - Test recovery scenarios
   - Document decisions

4. **Communicate transparently**
   - Public proposal discussions
   - Shared voting results
   - Published rollback reasons

---

## TROUBLESHOOTING

### Issue: Proposal Won't Submit
**Cause**: Safety constraint violation
**Solution**: Check validation report for specific failures
- Verify values within rule bounds
- Check confidence score > minimum
- Ensure target epoch in future
- Review safety bounds

### Issue: Voting Not Recording
**Cause**: Validator not registered or voting window closed
**Solution**:
- Verify validator registered with stake
- Check voting window open: start ≤ epoch < end
- Confirm validator hasn't already voted
- Verify signature format

### Issue: Activation Failed
**Cause**: Proposal not approved or target epoch not reached
**Solution**:
- Check voting result (is_approved)
- Verify current_epoch >= target_epoch
- Ensure all rules still exist
- Check proposed values within bounds

### Issue: Invariant Keeps Violating
**Cause**: Proposal caused unintended consequences
**Solution**:
- Let automatic rollback trigger
- System will revert changes
- Analyze root cause
- Design better proposal

### Issue: Rollback Didn't Complete
**Cause**: System error or corruption
**Solution**:
- **This should never happen** - design prevents it
- Check system logs
- Verify all rule values restored
- Contact protocol developers

---

## PERFORMANCE OPTIMIZATION

### For High-Volume Governance
```rust
// Batch multiple proposals
// Process votes efficiently
// Minimize storage overhead

// Recommended:
// - Max 3 concurrent proposals
// - Min 1 epoch between proposals
// - Max voting duration 20 epochs
```

### For Large Validator Sets
```rust
// Stake-weighted voting reduces votes needed
// Only store aggregate results
// Don't require individual vote signatures

// For 1000 validators:
// - Typical votes needed: 200-300 (quorum)
// - Storage: ~1MB per proposal
// - Computation: <1 second finalization
```

---

## COMPLIANCE & AUDIT

### Records Kept Immutably
- All proposals (with content hashes)
- All votes (with signatures)
- All activation records
- All rollback records
- All invariant violations
- All reputation updates

### Audit Capabilities
```
// Replay any proposal
history = orchestrator.activation_history();
for record in history {
    // Re-verify all constraints
    // Check all votes
    // Validate rule changes
    // Confirm determinism
}

// This proves:
// - Protocol followed rules
// - Governance was legitimate
// - No hidden changes
// - All decisions traceable
```

---

## FUTURE ENHANCEMENTS

### Planned (Phase 6)
- [ ] Time-locked voting
- [ ] Delegation mechanisms
- [ ] Multi-signature proposals
- [ ] Conditional rule changes
- [ ] Gradual parameter transitions

### Under Consideration
- [ ] Quorum requirements
- [ ] Voting power decay
- [ ] Proposal deposits
- [ ] Ranked choice voting
- [ ] Prediction markets

---

*Last Updated: January 16, 2026*  
*Status: OPERATIONAL PROCEDURES DEFINED*
