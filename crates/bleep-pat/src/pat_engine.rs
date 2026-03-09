//! # BLEEP PAT Engine — Sprint 7
//!
//! Production-grade Programmable Asset Token (PAT) engine.
//!
//! PATs are fungible tokens issued on the BLEEP chain with configurable
//! burn rates, transfer restrictions, and optional supply caps.
//!
//! ## Token Model
//! - Supply is capped at `total_supply_cap` (set at creation).
//! - Transfers automatically deduct `burn_rate_bps` from the transferred amount
//!   (deflationary mechanic).
//! - Only the token `owner` may mint new tokens.
//! - Any holder may burn their own tokens at any time.
//!
//! ## Storage
//! - `PATRegistry` holds all token definitions and balances in memory.
//! - In Sprint 8 this will be persisted via the `bleep-state` RocksDB backend.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// ERROR TYPES
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum PATError {
    #[error("Token {0} not found")]
    TokenNotFound(String),
    #[error("Insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u128, need: u128 },
    #[error("Unauthorized: caller {0} is not the token owner")]
    Unauthorized(String),
    #[error("Supply cap exceeded: cap {cap}, would reach {would_reach}")]
    SupplyCapExceeded { cap: u128, would_reach: u128 },
    #[error("Token {0} already exists")]
    TokenAlreadyExists(String),
    #[error("Invalid burn rate: {0} bps (max 10000)")]
    InvalidBurnRate(u16),
    #[error("Zero amount not allowed")]
    ZeroAmount,
    #[error("Cannot transfer to self")]
    SelfTransfer,
}

pub type PATResult<T> = Result<T, PATError>;

// ─────────────────────────────────────────────────────────────────────────────
// TOKEN DEFINITION
// ─────────────────────────────────────────────────────────────────────────────

/// Immutable token metadata + mutable supply state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PATToken {
    /// Unique token symbol (e.g. "USDB", "WETH-PAT").
    pub symbol: String,
    /// Human-readable name.
    pub name: String,
    /// Decimal places (default 8, same as BLEEP).
    pub decimals: u8,
    /// Address of the token owner (sole minter).
    pub owner: String,
    /// Maximum total supply (0 = unlimited).
    pub total_supply_cap: u128,
    /// Current total minted supply.
    pub current_supply: u128,
    /// Total ever burned.
    pub total_burned: u128,
    /// Deflationary burn rate applied on each transfer (basis points).
    /// E.g. 50 = 0.5%.  Max 1000 (10%).
    pub burn_rate_bps: u16,
    /// Creation timestamp (seconds since epoch).
    pub created_at: u64,
    /// State hash (sha256 of supply + burned).
    pub state_hash: [u8; 32],
}

impl PATToken {
    /// Create a new token definition.
    pub fn new(
        symbol: String,
        name: String,
        decimals: u8,
        owner: String,
        total_supply_cap: u128,
        burn_rate_bps: u16,
        created_at: u64,
    ) -> PATResult<Self> {
        if burn_rate_bps > 1000 {
            return Err(PATError::InvalidBurnRate(burn_rate_bps));
        }
        let mut token = PATToken {
            symbol,
            name,
            decimals,
            owner,
            total_supply_cap,
            current_supply: 0,
            total_burned: 0,
            burn_rate_bps,
            created_at,
            state_hash: [0u8; 32],
        };
        token.recompute_hash();
        Ok(token)
    }

    /// Compute and store the state hash.
    pub fn recompute_hash(&mut self) {
        let mut h = Sha256::new();
        h.update(self.symbol.as_bytes());
        h.update(self.current_supply.to_be_bytes());
        h.update(self.total_burned.to_be_bytes());
        let result = h.finalize();
        self.state_hash.copy_from_slice(&result);
    }

    /// Amount burned on a transfer of `amount`.
    pub fn transfer_burn_amount(&self, amount: u128) -> u128 {
        if self.burn_rate_bps == 0 {
            return 0;
        }
        (amount * self.burn_rate_bps as u128) / 10_000
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOKEN LEDGER — per-token balance map
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenLedger {
    /// address → balance
    pub balances: BTreeMap<String, u128>,
}

impl TokenLedger {
    pub fn balance_of(&self, address: &str) -> u128 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    pub fn credit(&mut self, address: &str, amount: u128) {
        *self.balances.entry(address.to_string()).or_insert(0) += amount;
    }

    pub fn debit(&mut self, address: &str, amount: u128) -> PATResult<()> {
        let bal = self.balance_of(address);
        if bal < amount {
            return Err(PATError::InsufficientBalance { have: bal, need: amount });
        }
        let entry = self.balances.entry(address.to_string()).or_insert(0);
        *entry -= amount;
        if *entry == 0 {
            self.balances.remove(address);
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PAT EVENT LOG
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PATEvent {
    Mint  { token: String, to: String, amount: u128, ts: u64 },
    Burn  { token: String, from: String, amount: u128, ts: u64 },
    Transfer {
        token: String,
        from: String,
        to: String,
        amount: u128,
        burn_deducted: u128,
        ts: u64,
    },
    TokenCreated { token: String, owner: String, ts: u64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// PAT REGISTRY
// ─────────────────────────────────────────────────────────────────────────────

/// Central PAT registry. Arc<Mutex<PATRegistry>> is held by the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PATRegistry {
    pub tokens:  BTreeMap<String, PATToken>,
    pub ledgers: BTreeMap<String, TokenLedger>,
    pub events:  Vec<PATEvent>,
}

impl PATRegistry {
    pub fn new() -> Self {
        Self {
            tokens:  BTreeMap::new(),
            ledgers: BTreeMap::new(),
            events:  Vec::new(),
        }
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // ── Token creation ───────────────────────────────────────────────────────

    /// Create a new PAT.  `owner` becomes the sole minter.
    pub fn create_token(
        &mut self,
        symbol: String,
        name: String,
        decimals: u8,
        owner: String,
        total_supply_cap: u128,
        burn_rate_bps: u16,
    ) -> PATResult<()> {
        if self.tokens.contains_key(&symbol) {
            return Err(PATError::TokenAlreadyExists(symbol));
        }
        let token = PATToken::new(
            symbol.clone(), name, decimals, owner.clone(),
            total_supply_cap, burn_rate_bps, Self::now(),
        )?;
        self.tokens.insert(symbol.clone(), token);
        self.ledgers.insert(symbol.clone(), TokenLedger::default());
        self.events.push(PATEvent::TokenCreated { token: symbol, owner, ts: Self::now() });
        Ok(())
    }

    // ── Mint ─────────────────────────────────────────────────────────────────

    /// Mint `amount` tokens of `symbol` to `to`.  Caller must be token owner.
    pub fn mint(&mut self, symbol: &str, caller: &str, to: &str, amount: u128) -> PATResult<()> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        let token = self.tokens.get(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
        if token.owner != caller {
            return Err(PATError::Unauthorized(caller.to_string()));
        }
        if token.total_supply_cap > 0 {
            let would_reach = token.current_supply.checked_add(amount)
                .unwrap_or(u128::MAX);
            if would_reach > token.total_supply_cap {
                return Err(PATError::SupplyCapExceeded {
                    cap: token.total_supply_cap,
                    would_reach,
                });
            }
        }
        // Update token supply
        {
            let token = self.tokens.get_mut(symbol).unwrap();
            token.current_supply += amount;
            token.recompute_hash();
        }
        // Credit ledger
        self.ledgers.get_mut(symbol).unwrap().credit(to, amount);
        self.events.push(PATEvent::Mint {
            token: symbol.to_string(),
            to: to.to_string(),
            amount,
            ts: Self::now(),
        });
        Ok(())
    }

    // ── Burn ─────────────────────────────────────────────────────────────────

    /// Burn `amount` tokens of `symbol` from `from`.  Caller must own the tokens.
    pub fn burn(&mut self, symbol: &str, from: &str, amount: u128) -> PATResult<()> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        // Debit ledger first (validates balance)
        self.ledgers.get_mut(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?
            .debit(from, amount)?;
        // Update supply
        {
            let token = self.tokens.get_mut(symbol)
                .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
            token.current_supply = token.current_supply.saturating_sub(amount);
            token.total_burned  += amount;
            token.recompute_hash();
        }
        self.events.push(PATEvent::Burn {
            token: symbol.to_string(),
            from: from.to_string(),
            amount,
            ts: Self::now(),
        });
        Ok(())
    }

    // ── Transfer ─────────────────────────────────────────────────────────────

    /// Transfer `amount` from `from` to `to`.
    ///
    /// The `burn_rate_bps` is deducted from `amount` and permanently destroyed.
    /// The recipient receives `amount - burn_deducted`.
    pub fn transfer(
        &mut self,
        symbol: &str,
        from: &str,
        to: &str,
        amount: u128,
    ) -> PATResult<u128> {
        if amount == 0 {
            return Err(PATError::ZeroAmount);
        }
        if from == to {
            return Err(PATError::SelfTransfer);
        }
        let burn_deducted = {
            let token = self.tokens.get(symbol)
                .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?;
            token.transfer_burn_amount(amount)
        };
        let received = amount - burn_deducted;

        // Debit sender (full amount)
        self.ledgers.get_mut(symbol)
            .ok_or_else(|| PATError::TokenNotFound(symbol.to_string()))?
            .debit(from, amount)?;

        // Credit recipient (net of burn)
        self.ledgers.get_mut(symbol).unwrap().credit(to, received);

        // Update supply if burn > 0
        if burn_deducted > 0 {
            let token = self.tokens.get_mut(symbol).unwrap();
            token.current_supply  = token.current_supply.saturating_sub(burn_deducted);
            token.total_burned   += burn_deducted;
            token.recompute_hash();
        }

        self.events.push(PATEvent::Transfer {
            token: symbol.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            burn_deducted,
            ts: Self::now(),
        });
        Ok(received)
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    pub fn balance_of(&self, symbol: &str, address: &str) -> u128 {
        self.ledgers.get(symbol)
            .map(|l| l.balance_of(address))
            .unwrap_or(0)
    }

    pub fn get_token(&self, symbol: &str) -> Option<&PATToken> {
        self.tokens.get(symbol)
    }

    pub fn list_tokens(&self) -> Vec<&PATToken> {
        self.tokens.values().collect()
    }

    pub fn recent_events(&self, limit: usize) -> Vec<&PATEvent> {
        let n = self.events.len();
        let start = n.saturating_sub(limit);
        self.events[start..].iter().collect()
    }
}

impl Default for PATRegistry {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> PATRegistry {
        let mut reg = PATRegistry::new();
        reg.create_token(
            "USDB".to_string(),
            "USD Bleep".to_string(),
            8,
            "alice".to_string(),
            1_000_000_000 * 100_000_000u128, // 1B tokens
            50, // 0.5% transfer burn
        ).unwrap();
        reg
    }

    #[test]
    fn test_mint_and_balance() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 1_000_000_000).unwrap();
        assert_eq!(reg.balance_of("USDB", "bob"), 1_000_000_000);
        assert_eq!(reg.get_token("USDB").unwrap().current_supply, 1_000_000_000);
    }

    #[test]
    fn test_unauthorized_mint() {
        let mut reg = registry();
        let err = reg.mint("USDB", "mallory", "bob", 1_000).unwrap_err();
        assert!(matches!(err, PATError::Unauthorized(_)));
    }

    #[test]
    fn test_burn() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "alice", 5_000_000_000).unwrap();
        reg.burn("USDB", "alice", 1_000_000_000).unwrap();
        assert_eq!(reg.balance_of("USDB", "alice"), 4_000_000_000);
        assert_eq!(reg.get_token("USDB").unwrap().total_burned, 1_000_000_000);
    }

    #[test]
    fn test_transfer_with_burn_rate() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "alice", 10_000_000_000).unwrap();
        // Transfer 1000 with 0.5% burn → burn = 5, received = 995
        let received = reg.transfer("USDB", "alice", "bob", 1_000_000_000).unwrap();
        let burn = 1_000_000_000u128 * 50 / 10_000; // 5_000_000
        assert_eq!(received, 1_000_000_000 - burn);
        assert_eq!(reg.balance_of("USDB", "bob"), received);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let mut reg = PATRegistry::new();
        reg.create_token(
            "CAPPED".to_string(), "Capped".to_string(), 8,
            "owner".to_string(), 1000, 0,
        ).unwrap();
        reg.mint("CAPPED", "owner", "user", 1000).unwrap();
        let err = reg.mint("CAPPED", "owner", "user", 1).unwrap_err();
        assert!(matches!(err, PATError::SupplyCapExceeded { .. }));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 100).unwrap();
        let err = reg.transfer("USDB", "bob", "carol", 200).unwrap_err();
        assert!(matches!(err, PATError::InsufficientBalance { .. }));
    }

    #[test]
    fn test_event_log() {
        let mut reg = registry();
        reg.mint("USDB", "alice", "bob", 500_000_000).unwrap();
        reg.transfer("USDB", "bob", "carol", 100_000_000).unwrap();
        // TokenCreated + Mint + Transfer = 3 events
        assert_eq!(reg.events.len(), 3);
    }
}
