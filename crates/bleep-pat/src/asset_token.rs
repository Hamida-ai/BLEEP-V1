//! Protocol Asset Token (PAT) Module
//! 
//! This module implements the Protocol Asset Token system for BLEEP.
//! It handles tokenomics, minting, burning, and token governance.

use serde::{Deserialize, Serialize};

/// Protocol Asset Token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PATConfig {
    /// Total supply cap
    pub total_supply_cap: u128,
    /// Current circulating supply
    pub current_supply: u128,
}

impl Default for PATConfig {
    fn default() -> Self {
        Self {
            total_supply_cap: 1_000_000_000 * 10_u128.pow(18),
            current_supply: 0,
        }
    }
}
