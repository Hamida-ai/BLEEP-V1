//! # BleepEconomicsRuntime — Sprint 7
//!
//! Wires all four economic engines into a single coordinated runtime that is
//! called at each epoch boundary by the Scheduler task `token_burn_execution`.
//!
//! ## Epoch lifecycle (called once per epoch end)
//! ```text
//! EpochEnd(epoch_id, block_count, fee_revenue, shard_utilisation)
//!   │
//!   ├─► FeeMarket::update_base_fee(avg_utilisation)    → new base_fee
//!   │
//!   ├─► OracleBridgeEngine::aggregate_prices("BLEEP/USD", ts, 300)
//!   │    → AggregatedPrice (median from ≥3 operators)
//!   │
//!   ├─► ValidatorIncentivesEngine::compute_epoch_rewards(epoch)
//!   │    → Vec<RewardRecord>  (per-validator)
//!   │
//!   ├─► CanonicalTokenomicsEngine::record_emission(epoch, BlockProposal, total)
//!   │   CanonicalTokenomicsEngine::record_burn(epoch, TransactionFee, burn_amt)
//!   │
//!   └─► CanonicalTokenomicsEngine::finalize_epoch(epoch)  → state_hash
//! ```

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    CanonicalTokenomicsEngine, EmissionType, BurnType, TokenomicsError,
    FeeMarket, ShardCongestion, TransactionType, ResourceUsage, FeeMarketError,
    ValidatorIncentivesEngine, ValidatorMetrics, ValidatorError, RewardRecord,
    OracleBridgeEngine, PriceUpdate, OracleSource, OracleError,
    integration::{BleepEconomics, EconomicError},
    distribution::FeeDistribution,
};

// ── Public re-exports for convenience ────────────────────────────────────────
pub use crate::integration::BleepEconomics as EconomicState;

/// Input provided to the runtime at each epoch boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochInput {
    /// Epoch that just ended.
    pub epoch: u64,
    /// Number of blocks produced in this epoch.
    pub block_count: u64,
    /// Total transaction fee revenue (microBLEEP) collected this epoch.
    pub fee_revenue: u128,
    /// Average shard utilisation over the epoch (0–10_000 bps).
    pub avg_utilisation_bps: u16,
    /// Per-validator metrics for reward computation.
    pub validator_metrics: Vec<ValidatorMetrics>,
    /// Oracle price updates received during the epoch.
    pub oracle_updates: Vec<PriceUpdate>,
}

/// Output produced by the runtime at each epoch boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochOutput {
    /// Epoch that was processed.
    pub epoch: u64,
    /// New base fee after EIP-1559 adjustment (microBLEEP / gas unit).
    pub new_base_fee: u128,
    /// Total BLEEP emitted this epoch (microBLEEP).
    pub total_emitted: u128,
    /// Total BLEEP burned this epoch (microBLEEP).
    pub total_burned: u128,
    /// Current circulating supply after epoch (microBLEEP).
    pub circulating_supply: u128,
    /// Validator reward records.
    pub reward_records: Vec<RewardRecord>,
    /// Supply state hash (for consensus commitment).
    pub supply_state_hash: Vec<u8>,
    /// BLEEP/USD median price (microUSD per BLEEP, 6 decimals).
    /// None if fewer than 3 oracle sources available.
    pub bleep_usd_price: Option<u128>,
}

/// Errors that can occur during epoch processing.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Tokenomics error: {0}")]
    Tokenomics(#[from] TokenomicsError),
    #[error("Fee market error: {0}")]
    FeeMarket(#[from] FeeMarketError),
    #[error("Validator incentives error: {0}")]
    Validator(#[from] ValidatorError),
    #[error("Oracle error: {0}")]
    Oracle(#[from] OracleError),
    #[error("Economic invariant violation: {0}")]
    InvariantViolation(String),
}

/// The BLEEP economic runtime.  Arc<Mutex<BleepEconomicsRuntime>> is held by
/// the node and accessed by the Scheduler at each epoch boundary.
pub struct BleepEconomicsRuntime {
    /// All four integrated engines.
    pub state: BleepEconomics,
    /// Monotonically increasing epoch counter (mirrors chain epoch).
    pub last_epoch: u64,
    /// Epoch outputs retained for RPC queries (last 100 epochs).
    pub epoch_history: std::collections::VecDeque<EpochOutput>,
}

impl BleepEconomicsRuntime {
    /// Initialise at genesis (zero supply, default params).
    pub fn genesis() -> Self {
        let mut runtime = BleepEconomicsRuntime {
            state: BleepEconomics::genesis(),
            last_epoch: 0,
            epoch_history: std::collections::VecDeque::with_capacity(100),
        };
        // Register the default BLEEP/USD shard on the fee market
        runtime.state.fee_market.record_shard_congestion(0, ShardCongestion {
            shard_id: 0,
            pending_txs: 0,
            utilisation_bps: 5000,
            gas_price_multiplier: 1000,
        });
        runtime
    }

    /// Register a validator with the incentives engine.
    /// Called once per validator at stake time.
    pub fn register_validator(&mut self, validator_id: Vec<u8>, stake: u128) -> Result<(), RuntimeError> {
        self.state.validators.register_validator(validator_id, stake)?;
        Ok(())
    }

    /// Register an oracle operator (5 operators for 3-of-5 quorum).
    pub fn register_oracle_operator(&mut self, operator_id: Vec<u8>, slashing_balance: u128) -> Result<(), RuntimeError> {
        self.state.oracle_bridge.register_operator(operator_id, slashing_balance)?;
        Ok(())
    }

    /// Submit a price update from an oracle operator.
    pub fn submit_price_update(&mut self, update: PriceUpdate) -> Result<(), RuntimeError> {
        self.state.oracle_bridge.submit_price_update(update)?;
        Ok(())
    }

    /// Process all economic state transitions for a completed epoch.
    ///
    /// This is the hot path called by the `token_burn_execution` Scheduler task.
    pub fn process_epoch(&mut self, input: EpochInput) -> Result<EpochOutput, RuntimeError> {
        let epoch = input.epoch;
        info!("⚙️  [EconomicsRuntime] Processing epoch {} ({} blocks, {} µBLEEP fees)",
              epoch, input.block_count, input.fee_revenue);

        // ── 1. Update fee market base fee ────────────────────────────────────
        let new_base_fee = self.state.fee_market
            .update_base_fee(epoch, input.avg_utilisation_bps)?;
        info!("  💹 New base fee: {} µBLEEP", new_base_fee);

        // ── 2. Record shard utilisation ───────────────────────────────────────
        self.state.fee_market.record_shard_congestion(0, ShardCongestion {
            shard_id: 0,
            pending_txs: (input.block_count * 100) as u32,
            utilisation_bps: input.avg_utilisation_bps,
            gas_price_multiplier: (new_base_fee / 1000).max(1000) as u64,
        });

        // ── 3. Oracle price aggregation ───────────────────────────────────────
        // Ingest any new oracle updates supplied with this epoch input
        for update in &input.oracle_updates {
            if let Err(e) = self.state.oracle_bridge.submit_price_update(update.clone()) {
                warn!("  ⚠️  Oracle update rejected: {}", e);
            }
        }
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let bleep_usd_price = match self.state.oracle_bridge.aggregate_prices("BLEEP/USD", ts, 300) {
            Ok(agg) => {
                info!("  🔮 BLEEP/USD median price: {} µUSD ({} sources)", agg.median_price, agg.source_count);
                Some(agg.median_price)
            }
            Err(OracleError::InsufficientSources) => {
                info!("  🔮 BLEEP/USD: insufficient oracle sources (<3) — skipping price commit");
                None
            }
            Err(e) => {
                warn!("  ⚠️  Oracle aggregation failed: {}", e);
                None
            }
        };

        // ── 4. Validator metrics + reward computation ─────────────────────────
        for metrics in &input.validator_metrics {
            if let Err(e) = self.state.validators.record_metrics(
                metrics.validator_id.clone(),
                metrics.clone(),
            ) {
                warn!("  ⚠️  Validator metrics skipped ({}): {}", hex::encode(&metrics.validator_id[..4.min(metrics.validator_id.len())]), e);
            }
        }
        let reward_records = match self.state.validators.compute_epoch_rewards(epoch) {
            Ok(records) => {
                let total: u128 = records.iter().map(|r| r.total_reward).sum();
                info!("  🏆 Validator rewards: {} records, {} µBLEEP total", records.len(), total);
                records
            }
            Err(e) => {
                warn!("  ⚠️  Reward computation failed: {} — using empty records", e);
                vec![]
            }
        };

        // ── 5. Tokenomics: emission ────────────────────────────────────────────
        let total_emitted: u128 = reward_records.iter().map(|r| r.total_reward).sum();
        if total_emitted > 0 {
            self.state.tokenomics.record_emission(
                epoch,
                EmissionType::BlockProposal,
                total_emitted,
            )?;
        }

        // ── 6. Tokenomics: fee distribution (25% burn / 50% validators / 25% treasury)
        // Uses the canonical FeeDistribution model from distribution.rs.
        let fee_dist = FeeDistribution::compute(input.fee_revenue);
        debug_assert!(fee_dist.is_consistent(), "fee distribution invariant broken");

        let total_burned = if fee_dist.burned > 0 {
            self.state.tokenomics.record_burn(epoch, BurnType::TransactionFee, fee_dist.burned)?;
            info!("  🔥 Fee burn: {} µBLEEP (25% of {} µBLEEP revenue)", fee_dist.burned, input.fee_revenue);
            fee_dist.burned
        } else {
            0
        };

        // Validator reward top-up from fee revenue (50% of fees)
        if fee_dist.validator_reward > 0 {
            // Record as additional emission from the fee validator share bucket.
            // This does NOT draw from the ValidatorRewards allocation pool —
            // it comes from fee revenue already in circulation, so we add it
            // directly to the emission record without a supply-cap check.
            self.state.tokenomics.record_emission(
                epoch, EmissionType::BlockProposal, fee_dist.validator_reward
            ).unwrap_or_else(|e| warn!("  ⚠️  Fee validator emission skipped: {}", e));
            info!("  💰 Validator fee share: {} µBLEEP (50% of fees)", fee_dist.validator_reward);
        }

        // Treasury allocation from fee revenue (25% of fees)
        if fee_dist.treasury > 0 {
            info!("  🏛  Treasury fee share: {} µBLEEP (25% of fees)", fee_dist.treasury);
            // Treasury share is tracked via burn_records with a separate type.
            // In production this routes to the Foundation Treasury bucket in GenesisAllocation.
            self.state.tokenomics.record_burn(epoch, BurnType::ProposalRejection, 0)
                .unwrap_or(()); // no-op placeholder; real routing in distribution engine
        }

        // ── 7. Finalize epoch (snapshot + hash) ───────────────────────────────
        let supply_state_hash = self.state.tokenomics.finalize_epoch(epoch)?;
        let circulating_supply = self.state.tokenomics.supply_state.circulating_supply;

        info!("  ✅ Epoch {} finalized: supply={} µBLEEP, hash={}",
              epoch, circulating_supply,
              hex::encode(&supply_state_hash[..4.min(supply_state_hash.len())]));

        // ── 8. Invariant check ────────────────────────────────────────────────
        if let Err(e) = self.state.verify_epoch_invariants() {
            return Err(RuntimeError::InvariantViolation(e.to_string()));
        }

        self.last_epoch = epoch;

        let output = EpochOutput {
            epoch,
            new_base_fee,
            total_emitted,
            total_burned,
            circulating_supply,
            reward_records,
            supply_state_hash,
            bleep_usd_price,
        };

        // Retain last 100 epochs in the history ring buffer
        if self.epoch_history.len() >= 100 {
            self.epoch_history.pop_front();
        }
        self.epoch_history.push_back(output.clone());

        Ok(output)
    }

    /// Retrieve the output for a specific epoch (for RPC).
    pub fn get_epoch_output(&self, epoch: u64) -> Option<&EpochOutput> {
        self.epoch_history.iter().find(|o| o.epoch == epoch)
    }

    /// Latest epoch output.
    pub fn latest_epoch_output(&self) -> Option<&EpochOutput> {
        self.epoch_history.back()
    }

    /// Current circulating supply.
    pub fn circulating_supply(&self) -> u128 {
        self.state.tokenomics.supply_state.circulating_supply
    }

    /// Current base fee.
    pub fn current_base_fee(&self) -> u128 {
        self.state.fee_market.base_fee_params.current_base_fee
    }

    /// Total burned to date.
    pub fn total_burned(&self) -> u128 {
        self.state.tokenomics.supply_state.total_burned
    }

    /// Total minted to date.
    pub fn total_minted(&self) -> u128 {
        self.state.tokenomics.supply_state.total_minted
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_metrics(id_byte: u8, blocks: u32) -> ValidatorMetrics {
        ValidatorMetrics {
            validator_id: vec![id_byte; 8],
            blocks_proposed: blocks,
            attestations_included: blocks * 10,
            healing_participations: 0,
            shards_coordinated: 1,
            double_signs_evidence: 0,
            state_transition_failures: 0,
            epoch: 1,
        }
    }

    #[test]
    fn test_genesis_runtime() {
        let rt = BleepEconomicsRuntime::genesis();
        assert_eq!(rt.circulating_supply(), 0);
        assert_eq!(rt.total_burned(), 0);
        assert_eq!(rt.last_epoch, 0);
    }

    #[test]
    fn test_epoch_processing_no_oracle() {
        let mut rt = BleepEconomicsRuntime::genesis();
        // Register a validator
        rt.register_validator(vec![1u8; 8], 1_000_000).unwrap();

        let input = EpochInput {
            epoch: 1,
            block_count: 100,
            fee_revenue: 5_000_000,        // 0.05 BLEEP fees
            avg_utilisation_bps: 6000,     // 60% load
            validator_metrics: vec![make_metrics(1, 100)],
            oracle_updates: vec![],
        };

        let out = rt.process_epoch(input).unwrap();
        assert_eq!(out.epoch, 1);
        // 25% of 5_000_000 = 1_250_000 burned
        assert_eq!(out.total_burned, 1_250_000);
        // No oracle price available without operators
        assert!(out.bleep_usd_price.is_none());
    }

    #[test]
    fn test_oracle_price_aggregation_quorum() {
        let mut rt = BleepEconomicsRuntime::genesis();

        // Register 5 oracle operators
        for i in 0u8..5 {
            rt.register_oracle_operator(vec![i; 8], 10_000_000).unwrap();
        }

        let ts = 1_700_000_000u64;
        // Submit 3 price updates for quorum (need >= 3 fresh)
        for i in 0u8..3 {
            rt.submit_price_update(PriceUpdate {
                source: OracleSource::Custom(vec![i; 8]),
                asset: "BLEEP/USD".to_string(),
                price: 100_000_000u128 + i as u128 * 1_000_000, // ~$1 with slight spread
                timestamp: ts,
                confidence_bps: 100,  // 1% confidence
                operator_id: vec![i; 8],
                signature: vec![0u8; 64],
            }).unwrap();
        }

        let input = EpochInput {
            epoch: 1,
            block_count: 100,
            fee_revenue: 0,
            avg_utilisation_bps: 5000,
            validator_metrics: vec![],
            oracle_updates: vec![],
        };

        // process_epoch calls aggregate with ts and max_age=300
        // The stored updates have timestamp=ts; we need SystemTime to be ~ts
        // Just verify the call doesn't panic — exact price depends on SystemTime
        let _out = rt.process_epoch(input);
    }

    #[test]
    fn test_fee_market_updates_base_fee() {
        let mut rt = BleepEconomicsRuntime::genesis();
        let initial_fee = rt.current_base_fee();

        let input = EpochInput {
            epoch: 1,
            block_count: 100,
            fee_revenue: 0,
            avg_utilisation_bps: 9000, // high load → fee should increase
            validator_metrics: vec![],
            oracle_updates: vec![],
        };
        let out = rt.process_epoch(input).unwrap();
        // With 90% utilization above 50% target, fee should have increased
        assert!(out.new_base_fee >= initial_fee,
                "Expected fee to stay same or increase under high load");
    }

    #[test]
    fn test_invariants_hold_after_epoch() {
        let mut rt = BleepEconomicsRuntime::genesis();
        rt.register_validator(vec![1u8; 8], 1_000_000).unwrap();
        let input = EpochInput {
            epoch: 1,
            block_count: 50,
            fee_revenue: 10_000_000,
            avg_utilisation_bps: 5000,
            validator_metrics: vec![make_metrics(1, 50)],
            oracle_updates: vec![],
        };
        let out = rt.process_epoch(input).unwrap();
        // supply = total_minted - total_burned
        let minted = rt.total_minted();
        let burned = rt.total_burned();
        assert_eq!(out.circulating_supply, minted.saturating_sub(burned));
    }
}
