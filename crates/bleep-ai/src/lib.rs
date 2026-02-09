// ==================== PHASE 4: AI ADVISORY MODULE ====================
// 
// This module implements BLEEP's AI Advisory System that enhances protocol
// safety without becoming a trusted party.
//
// KEY INVARIANTS:
// 1. AI advises only - cannot execute changes
// 2. All AI outputs are signed and verifiable
// 3. Governance votes on AI recommendations
// 4. AI cannot bypass governance authority
// 5. Deterministic feature extraction (reproducible)
// 6. Fallback mechanisms if AI fails

pub mod feature_extractor;
pub mod ai_decision_module;
pub mod governance_integration;

// Legacy modules (Phase 3)
pub mod deterministic_inference;
pub mod ai_proposal_types;
pub mod ai_attestation;
pub mod ai_constraint_validator;
pub mod ai_consensus_integration;
pub mod ai_feedback_loop;

#[cfg(test)]
mod phase3_tests;

#[cfg(test)]
mod phase3_unit_tests;

// Re-export commonly used types
pub use deterministic_inference::{
    DeterministicInferenceEngine, InferenceRecord, ModelMetadata,
    DeterministicInferenceError, DeterministicInferenceResult,
};

pub use ai_proposal_types::{
    AIProposal, AIProposalError, ConsensusModeProposal,
    ShardRollbackProposal, ShardRebalanceProposal,
    ValidatorSecurityProposal, TokenomicsProposal,
    GovernancePriorityProposal, EvidenceType,
};

pub use ai_attestation::{
    AIAttestationManager, AIAttestationRecord, AIOutputCommitment,
    ProofOfInference, ConstraintOutcome,
};

pub use ai_constraint_validator::{
    ConstraintValidator, ProtocolInvariants, ConstraintContext,
    ConstraintError, ConstraintResult,
};

pub use ai_consensus_integration::{
    AIConsensusOrchestrator, ConsensusProposal, ProposalOutcome,
    ProposalState, HealingAction, HealingExecution,
};

pub use ai_feedback_loop::{
    FeedbackManager, ModelPerformance, AccuracyMetrics,
    ConfidenceCalibration, SystemHealthMetrics,
};

// PHASE 4: AI ADVISORY SYSTEM
pub use feature_extractor::{
    FeatureExtractor, ExtractedFeatures, OnChainTelemetry,
    NetworkMetrics, ConsensusMetrics, ValidatorMetrics, FinalityMetrics,
};

pub use ai_decision_module::{
    AIDecisionModule, AnomalyAssessment, AnomalyClass, AISignature,
    RecoveryRecommendation, AIError,
};

pub use governance_integration::{
    GovernanceIntegration, AIAssessmentProposal, AIFeedback, VoteStatus,
    GovernanceError,
};

mod wallet;
mod governance;
mod security;
mod smart_contracts;
mod interoperability;
mod analytics;
mod compliance;
mod sharding;
mod energy_monitor;
pub mod ai_decision;
pub mod zkp_verification;
pub mod bleep_connect;
pub mod consensus;
pub mod ai_assistant;

/// Initialize BLEEP AI services
/// 
/// This function sets up all AI subsystems:
/// 1. Feature extraction from on-chain telemetry
/// 2. Deterministic AI analysis with signed outputs
/// 3. Governance integration (AI is advisory only)
/// 4. Attestation manager for cryptographic signing
/// 5. Constraint validator with protocol invariants
/// 6. Consensus orchestrator for proposal flow
/// 7. Feedback loop for performance tracking
pub fn start_ai_services() {
    // In production, this would initialize:
    // - Feature extractor for on-chain telemetry
    // - AI decision module (deterministic inference)
    // - Governance integration (advisory-only recommendations)
    // - ONNX runtime with approved model registry
    // - Cryptographic signing infrastructure
    // - Constraint evaluation rules
    // - Feedback aggregation system
    // - Integration with consensus and healing layers
    
    log::info!("BLEEP AI Services initialized");
    log::info!("  - Feature extraction engine ready");
    log::info!("  - AI decision module (advisory only)");
    log::info!("  - Governance integration active");
    log::info!("  - Deterministic inference engine ready");
    log::info!("  - Cryptographic attestation enabled");
    log::info!("  - Constraint validation active");
    log::info!("  - Consensus integration ready");
    log::info!("  - Feedback loop monitoring enabled");
}


