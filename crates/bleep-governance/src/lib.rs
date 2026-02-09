// PHASE 2: ON-CHAIN GOVERNANCE CORE
pub mod governance_core;
pub mod deterministic_executor;

// PHASE 4: CONSTITUTIONAL GOVERNANCE LAYER
pub mod constitution;
pub mod zk_voting;
pub mod proposal_lifecycle;
pub mod forkless_upgrades;
pub mod governance_binding;

// PHASE 5: AI-Driven Protocol Evolution Layer
pub mod governance_engine;
pub mod protocol_rules;
pub mod apip;
pub mod safety_constraints;
pub mod ai_reputation;
pub mod protocol_evolution;
pub mod ai_hooks;
pub mod invariant_monitoring;
pub mod governance_voting;
pub mod deterministic_activation;

#[cfg(test)]
mod phase5_integration_tests;

#[cfg(test)]
mod phase5_comprehensive_tests;

#[cfg(test)]
mod phase4_governance_tests;

#[cfg(test)]
mod phase2_governance_tests;

pub use governance_core::{
    ProposalType, ProposalState, VotingWindow, Vote, VoteTally,
    Proposal, GovernancePayload, SanctionAction, GovernanceEngine, GovernanceError,
};

pub use deterministic_executor::{
    DeterministicExecutor, ExecutionLogEntry, ExecutionStatus, ExecutionRecord, ExecutionError,
};

pub use constitution::{
    BLEEPConstitution, ConstitutionalConstraint, ConstitutionalScope,
    GovernanceAction, ValidationResult, ConstraintRule, RuleType,
};

pub use zk_voting::{
    ZKVotingEngine, VotingBallot, EncryptedBallot, VoteCommitment,
    EligibilityProof, VoterRole, VoteTally, TallyProof, ZKVotingError,
};

pub use proposal_lifecycle::{
    ProposalLifecycleManager, ProposalRecord, ProposalState, ProposalArchive,
    ProposalStateTransition, ProposalError,
};

pub use forkless_upgrades::{
    ProtocolUpgradeManager, ApprovedUpgrade, UpgradePayload, Version,
    UpgradeStatus, UpgradeCheckpoint, StateMigration, MigrationType,
    UpgradePreconditions, UpgradeError,
};

pub use governance_binding::{
    GovernanceConsensusBinding, ProposalOutcome, ActivationRecord,
};

pub use protocol_rules::{
    ProtocolRule, ProtocolRuleSet, ProtocolRuleSetFactory,
    RuleBounds, RuleValue, RuleVersion,
};

pub use apip::{
    APIP, APIPBuilder, APIPStatus, RiskLevel,
    RuleChange, SafetyBounds, AIModelMetadata,
};

pub use safety_constraints::{
    SafetyConstraintsEngine, ValidationReport, ConstraintCheckResult, CheckSeverity,
};

pub use ai_reputation::{
    AIReputationTracker, AIReputation, ProposalOutcome, ReputationRecord,
};

pub use protocol_evolution::{
    ProtocolEvolutionOrchestrator, VotingResult, ActivationRecord,
};

pub use ai_hooks::{
    AIHooks, AIHooksValidator, AdvisoryScore, HistoricalAnalysis,
    SimulationResult, OptimizationSuggestion,
};

pub use invariant_monitoring::{
    InvariantMonitor, GlobalInvariantMonitor, InvariantType, InvariantThreshold,
    InvariantSeverity, ViolationRecord, HealthStatus, GlobalHealth,
};

pub use governance_voting::{
    GovernanceVotingEngine, ValidatorVote, VotingWindow, ProposalVotingState,
    VotingResult, VotingError,
};

pub use deterministic_activation::{
    DeterministicActivationManager, ActivationPlan, ActivationState, ActivationError,
};

/// Initialize BLEEP governance with Phase 5 protocol evolution layer
/// 
/// SAFETY: Creates deterministic protocol state with genesis rules,
/// configures AI-driven evolution system, and initializes AI reputation tracking.
pub fn init_governance() -> Result<ProtocolEvolutionOrchestrator, Box<dyn std::error::Error>> {
    let genesis_ruleset = ProtocolRuleSetFactory::create_genesis()?;
    let orchestrator = ProtocolEvolutionOrchestrator::new(genesis_ruleset);
    Ok(orchestrator)
}

