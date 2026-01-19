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

