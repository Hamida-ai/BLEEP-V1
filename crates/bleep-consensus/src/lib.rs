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
pub mod block_producer;

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

pub use block_producer::{BlockProducer, FinalizedBlock, ProducerConfig, start_block_producer, MAX_TXS_PER_BLOCK, BLOCK_INTERVAL_MS};

pub mod gossip_bridge;
pub use gossip_bridge::{GossipBridge, encode_finalized_block, decode_finalized_block};


// ── Hardening-phase modules ────────────────────────────────────────────────────
pub mod chaos_engine;
pub mod shard_coordinator;
pub mod performance_bench;

pub use chaos_engine::{
    ChaosEngine, ChaosScenario, ChaosOutcome, ChaosConfig, ChaosSummary,
    ContinuousChaosHarness,
};
pub use shard_coordinator::{
    ShardCoordinator, ShardId, CrossShardTx, CrossShardState,
    StressTestResult, EpochStats, NUM_SHARDS as SHARD_COUNT,
};
pub use performance_bench::{
    PerformanceBenchmark, BenchmarkResult, TpsWindow,
    TARGET_TPS, BENCHMARK_DURATION_SECS,
};

pub mod security_audit;
pub use security_audit::{AuditReport, AuditFinding, AuditSummary, Severity, FindingStatus};
