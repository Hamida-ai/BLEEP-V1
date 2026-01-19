// consensus/mod.rs

// PHASE 1: CONSENSUS LAYER CORRECTION MODULES
pub mod epoch;
pub mod engine;
pub mod orchestrator;
pub mod pos_engine;
pub mod pbft_engine;
pub mod pow_engine;
pub mod ai_advisory;

// Safety invariants and documentation
pub mod safety_invariants;

// Integration tests (comprehensive real-world scenarios)
#[cfg(test)]
mod integration_tests;

// Legacy modules (maintained for compatibility)
pub mod consensus;
pub mod ai_adaptive_logic;
pub mod tests;

// Re-export critical types for convenience
pub use epoch::{ConsensusMode, EpochConfig, EpochState, EpochBuilder};
pub use engine::{ConsensusEngine, ConsensusError, ConsensusMetrics};
pub use orchestrator::{ConsensusOrchestrator, EmergencyPoWState};
pub use pos_engine::PoSConsensusEngine;
pub use pbft_engine::PbftConsensusEngine;
pub use pow_engine::EmergencyPoWEngine;
pub use ai_advisory::{AiAdvisoryReport, AiReportAggregator, AggregatedAdvisory, BoundedScore};
