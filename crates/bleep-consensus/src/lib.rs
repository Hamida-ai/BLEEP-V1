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

use std::collections::HashMap;
use std::sync::Arc;
use crate::engine::ConsensusMetrics;
use crate::pos_engine::PoSConsensusEngine;

pub fn run_consensus_engine() -> Result<(), Box<dyn std::error::Error>> {
    let config = EpochConfig::new(100, 0, 2)
        .map_err(|e| format!("Consensus epoch configuration failed: {}", e))?;

    let mut engines: HashMap<EpochConsensusMode, Arc<dyn ConsensusEngine>> = HashMap::new();
    let local_engine = Arc::new(PoSConsensusEngine::new("local-validator".to_string(), 1_000_000));
    engines.insert(EpochConsensusMode::PosNormal, local_engine);

    let mut orchestrator = ConsensusOrchestrator::new(
        config,
        engines,
        10,
        0.66,
        3,
    ).map_err(|e| format!("Consensus orchestrator initialization failed: {}", e))?;

    let mode = orchestrator.select_mode(0, &ConsensusMetrics::new());
    log::info!("Consensus orchestrator initialized in mode: {}", mode.as_str());
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
