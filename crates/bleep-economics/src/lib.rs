/// BLEEP PHASE 5: ECONOMIC NERVOUS SYSTEM
///
/// This crate implements the complete economic layer for BLEEP, ensuring:
/// - Tokenomics are provable and constitutional
/// - Fees reflect real resource usage
/// - Validators are incentivized to heal and participate
/// - Slashing is automatic and fair
/// - External data is trust-minimized
/// - The protocol survives rational adversaries
///
/// After Phase 5, BLEEP survives not just bugs — but greed.

pub mod tokenomics;
pub mod fee_market;
pub mod validator_incentives;
pub mod oracle_bridge;
pub mod game_theory;

// Re-export key types for easy access
pub use tokenomics::{
    CanonicalTokenomicsEngine, EmissionSchedule, BurnConfig, SupplyState,
    EmissionType, BurnType, TokenomicsError,
};

pub use fee_market::{
    FeeMarket, BaseFeeParams, ShardCongestion, TransactionType, ResourceUsage,
    FeeMarketError,
};

pub use validator_incentives::{
    ValidatorIncentivesEngine, ValidatorAccount, RewardRecord, SlashingEvidence,
    SlashingViolationType, ValidatorStatus, RewardType, ValidatorError,
};

pub use oracle_bridge::{
    OracleBridgeEngine, PriceUpdate, AggregatedPrice, OracleOperator,
    BridgeConfig, BridgeTransaction, OracleError, OracleSource,
};

pub use game_theory::{
    SafetyVerifier, SafetyAnalysis, AttackType, SafetyError,
};

/// Economic system integrator (combines all modules)
pub mod integration {
    use crate::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BleepEconomics {
        /// Tokenomics engine
        pub tokenomics: tokenomics::CanonicalTokenomicsEngine,
        /// Fee market
        pub fee_market: fee_market::FeeMarket,
        /// Validator incentives
        pub validators: validator_incentives::ValidatorIncentivesEngine,
        /// Oracle & bridge integration
        pub oracle_bridge: oracle_bridge::OracleBridgeEngine,
    }

    impl BleepEconomics {
        /// Initialize at genesis
        pub fn genesis() -> Self {
            BleepEconomics {
                tokenomics: tokenomics::CanonicalTokenomicsEngine::genesis(),
                fee_market: fee_market::FeeMarket::genesis(),
                validators: validator_incentives::ValidatorIncentivesEngine::genesis(),
                oracle_bridge: oracle_bridge::OracleBridgeEngine::genesis(),
            }
        }

        /// Verify all economic invariants for an epoch
        pub fn verify_epoch_invariants(&self) -> Result<(), EconomicError> {
            // Verify tokenomics
            self.tokenomics.supply_state.verify()?;
            self.tokenomics.supply_state.verify_hash()?;

            // Verify no negative balances
            if self.tokenomics.supply_state.total_burned > self.tokenomics.supply_state.total_minted {
                return Err(EconomicError::SupplyInvariantViolation);
            }

            // Verify circulating supply is correct
            let expected_circulation = self.tokenomics.supply_state.total_minted
                .saturating_sub(self.tokenomics.supply_state.total_burned);
            if self.tokenomics.supply_state.circulating_supply != expected_circulation {
                return Err(EconomicError::CirculationMismatch);
            }

            // Verify fee market is valid
            self.fee_market.base_fee_params.validate()?;

            Ok(())
        }
    }

    #[derive(Debug, thiserror::Error, Clone, PartialEq)]
    pub enum EconomicError {
        #[error("Tokenomics error: {0}")]
        Tokenomics(#[from] tokenomics::TokenomicsError),
        #[error("Fee market error: {0}")]
        FeeMarket(#[from] fee_market::FeeMarketError),
        #[error("Validator error: {0}")]
        Validator(#[from] validator_incentives::ValidatorError),
        #[error("Oracle error: {0}")]
        Oracle(#[from] oracle_bridge::OracleError),
        #[error("Supply invariant violated")]
        SupplyInvariantViolation,
        #[error("Circulation supply mismatch")]
        CirculationMismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_creation() {
        let econ = integration::BleepEconomics::genesis();
        assert_eq!(econ.tokenomics.supply_state.circulating_supply, 0);
        assert_eq!(econ.tokenomics.supply_state.total_minted, 0);
    }

    #[test]
    fn test_epoch_invariants() {
        let econ = integration::BleepEconomics::genesis();
        assert!(econ.verify_epoch_invariants().is_ok());
    }
}
