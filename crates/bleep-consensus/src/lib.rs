pub mod ai_adaptive_logic;
pub mod blockchain_state;
pub mod consensus;
pub mod networking;
pub mod tests;
pub mod epoch;
pub mod engine;
pub mod pos_engine;
pub mod pbft_engine;
pub mod pow_engine;
pub mod orchestrator;
pub mod validator_identity;
pub mod slashing_engine;
pub mod finality;

pub use consensus::{BLEEPAdaptiveConsensus, ConsensusMode, Validator};
pub use blockchain_state::BlockchainState;
pub use networking::NetworkingModule;
pub use epoch::{EpochConfig, EpochState, ConsensusMode as EpochConsensusMode};
pub use engine::{ConsensusEngine, ConsensusError};
pub use validator_identity::{ValidatorIdentity, ValidatorRegistry, ValidatorState};
pub use slashing_engine::{SlashingEngine, SlashingEvidence, SlashingEvent, SlashingPenalty};
pub use orchestrator::ConsensusOrchestrator;
pub use finality::{FinalizyCertificate, FinalityProof, FinalizityManager, ValidatorSignature};

pub fn run_consensus_engine() -> Result<(), Box<dyn std::error::Error>> {
    // Consensus engine initialization - called at node startup
    // Returns immediately; actual consensus runs in background tasks
    Ok(())
}
