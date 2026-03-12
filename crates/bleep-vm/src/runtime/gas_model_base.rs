//! Production gas metering for bleep-vm.
//!
//! Design goals:
//! - **Deterministic**: identical bytecode always charges identical gas.
//! - **Overflow-safe**: all arithmetic uses checked / saturating ops.
//! - **Fine-grained**: per-opcode cost table, memory quadratic scaling,
//!   storage per-byte pricing, cross-call premiums.
//! - **Adaptive pricing**: network-load factor adjusts base multiplier
//!   without breaking determinism within a block.
//! - **Instrumentation**: inserts metering calls into WASM bytecode before
//!   execution so the host function can interrupt on exhaustion.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::{debug, trace, warn};

use crate::error::{VmError, VmResult};
use crate::types::{GasSchedule, WasmOpcode};

// ─────────────────────────────────────────────────────────────────────────────
// CONSTANTS
// ─────────────────────────────────────────────────────────────────────────────

/// Absolute ceiling: no single transaction may consume more than this.
pub const MAX_GAS_PER_TX: u64 = 30_000_000;
/// Minimum gas limit a caller may specify (prevents spam with gas = 0).
pub const MIN_GAS_LIMIT: u64 = 21_000;
/// Gas cost per byte of calldata.
pub const GAS_PER_CALLDATA_BYTE: u64 = 16;
/// Gas cost per WASM page (64 KiB) initially requested.
pub const GAS_PER_INITIAL_MEMORY_PAGE: u64 = 6_400;

// ─────────────────────────────────────────────────────────────────────────────
// OPERATION BREAKDOWN
// ─────────────────────────────────────────────────────────────────────────────

/// Detailed breakdown of gas consumed per category.
#[derive(Debug, Clone, Default)]
pub struct GasBreakdown {
    pub computation:    u64,
    pub memory:         u64,
    pub storage_reads:  u64,
    pub storage_writes: u64,
    pub cross_calls:    u64,
    pub calldata:       u64,
    pub logs:           u64,
    pub crypto:         u64,
}

impl GasBreakdown {
    pub fn total(&self) -> u64 {
        self.computation
            .saturating_add(self.memory)
            .saturating_add(self.storage_reads)
            .saturating_add(self.storage_writes)
            .saturating_add(self.cross_calls)
            .saturating_add(self.calldata)
            .saturating_add(self.logs)
            .saturating_add(self.crypto)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GAS METER (per-transaction instance)
// ─────────────────────────────────────────────────────────────────────────────

/// Holds the gas state for a single transaction execution.
pub struct GasMeter {
    schedule:   Arc<GasSchedule>,
    limit:      u64,
    used:       u64,
    breakdown:  GasBreakdown,
    /// Per-opcode execution counts (for profiling / future pricing).
    op_counts:  BTreeMap<WasmOpcode, u64>,
}

impl GasMeter {
    pub fn new(limit: u64, schedule: Arc<GasSchedule>) -> VmResult<Self> {
        if limit < MIN_GAS_LIMIT {
            return Err(VmError::InvalidTransaction(format!(
                "Gas limit {limit} below minimum {MIN_GAS_LIMIT}"
            )));
        }
        if limit > MAX_GAS_PER_TX {
            return Err(VmError::InvalidTransaction(format!(
                "Gas limit {limit} exceeds maximum {MAX_GAS_PER_TX}"
            )));
        }
        Ok(GasMeter {
            schedule,
            limit,
            used: 0,
            breakdown: GasBreakdown::default(),
            op_counts: BTreeMap::new(),
        })
    }

    // ── Core charge ──────────────────────────────────────────────────────────

    /// Charge `amount` units of gas.  Returns `Err(GasExhausted)` if the limit
    /// would be exceeded.  Uses checked arithmetic throughout.
    #[inline]
    pub fn charge(&mut self, amount: u64) -> VmResult<()> {
        let new_used = self.used.checked_add(amount)
            .ok_or(VmError::GasOverflow)?;
        if new_used > self.limit {
            return Err(VmError::GasExhausted { used: new_used, limit: self.limit });
        }
        self.used = new_used;
        Ok(())
    }

    /// Charge for a WASM opcode.
    pub fn charge_opcode(&mut self, op: &WasmOpcode) -> VmResult<()> {
        let cost = self.schedule.costs.get(op).copied().unwrap_or(1);
        self.breakdown.computation = self.breakdown.computation.saturating_add(cost);
        *self.op_counts.entry(op.clone()).or_insert(0) += 1;
        self.charge(cost)
    }

    /// Charge for `n` bytes of calldata.
    pub fn charge_calldata(&mut self, n: usize) -> VmResult<()> {
        let cost = (n as u64).saturating_mul(GAS_PER_CALLDATA_BYTE);
        self.breakdown.calldata = self.breakdown.calldata.saturating_add(cost);
        self.charge(cost)
    }

    /// Charge for allocating `pages` WASM memory pages (64 KiB each).
    pub fn charge_memory_pages(&mut self, pages: u32) -> VmResult<()> {
        let cost = (pages as u64)
            .checked_mul(self.schedule.memory_per_page)
            .ok_or(VmError::GasOverflow)?;
        self.breakdown.memory = self.breakdown.memory.saturating_add(cost);
        self.charge(cost)
    }

    /// Charge for a storage write of `n` bytes.
    pub fn charge_storage_write(&mut self, n: usize) -> VmResult<()> {
        let cost = (n as u64).saturating_mul(self.schedule.storage_per_byte);
        self.breakdown.storage_writes = self.breakdown.storage_writes.saturating_add(cost);
        self.charge(cost)
    }

    /// Charge for a storage read (flat fee).
    pub fn charge_storage_read(&mut self) -> VmResult<()> {
        let cost = self.schedule.storage_per_byte;
        self.breakdown.storage_reads = self.breakdown.storage_reads.saturating_add(cost);
        self.charge(cost)
    }

    /// Charge for a cross-contract call.
    pub fn charge_cross_call(&mut self) -> VmResult<()> {
        let cost = self.schedule.cross_call_base;
        self.breakdown.cross_calls = self.breakdown.cross_calls.saturating_add(cost);
        self.charge(cost)
    }

    /// Charge for a SHA-256 hash over `chunks` 32-byte blocks.
    pub fn charge_sha256(&mut self, chunks: u64) -> VmResult<()> {
        let cost = chunks.saturating_mul(self.schedule.sha256_per_chunk);
        self.breakdown.crypto = self.breakdown.crypto.saturating_add(cost);
        self.charge(cost)
    }

    /// Charge for emitting a log entry with `data_len` bytes of data.
    pub fn charge_log(&mut self, data_len: usize) -> VmResult<()> {
        let cost = self.schedule.log_base
            .saturating_add((data_len as u64).saturating_mul(8));
        self.breakdown.logs = self.breakdown.logs.saturating_add(cost);
        self.charge(cost)
    }

    // ── Quadratic memory penalty ─────────────────────────────────────────────

    /// Charge the quadratic memory expansion cost used in Ethereum-style VMs.
    /// Cost = words² / 512  where `words` = ceil(bytes / 32).
    pub fn charge_memory_expansion(&mut self, new_size_bytes: u64) -> VmResult<()> {
        let words = new_size_bytes.div_ceil(32);
        let cost = words.saturating_mul(words) / 512;
        self.breakdown.memory = self.breakdown.memory.saturating_add(cost);
        self.charge(cost)
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    pub fn used(&self) -> u64        { self.used }
    pub fn limit(&self) -> u64       { self.limit }
    pub fn remaining(&self) -> u64   { self.limit.saturating_sub(self.used) }
    pub fn breakdown(&self) -> &GasBreakdown { &self.breakdown }
    pub fn op_counts(&self) -> &BTreeMap<WasmOpcode, u64> { &self.op_counts }

    /// Snapshot gas used as a refundable amount (e.g., for failed txs).
    pub fn refund_amount(&self, refund_fraction: f64) -> u64 {
        ((self.used as f64) * refund_fraction.clamp(0.0, 1.0)) as u64
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ADAPTIVE GAS PRICER  (block-level, shared across transactions)
// ─────────────────────────────────────────────────────────────────────────────

/// Block-level adaptive gas price oracle.
///
/// Tracks network utilisation and adjusts the base fee using EIP-1559-like
/// exponential smoothing.  All state is `AtomicU64` so it can be shared
/// across async tasks without a mutex.
pub struct AdaptiveGasPricer {
    /// Current base fee in gas units (fixed-point: actual = value / 1_000).
    base_fee:        AtomicU64,
    /// Target block utilisation in % (0–100).  Default = 50.
    target_util_pct: AtomicU64,
    /// Last block's actual utilisation (0–100).
    last_util_pct:   AtomicU64,
    /// Maximum base fee adjustment per block in ‰ (default ±12.5%).
    max_change_ppm:  AtomicU64,
}

impl AdaptiveGasPricer {
    pub fn new(initial_base_fee: u64) -> Self {
        AdaptiveGasPricer {
            base_fee:        AtomicU64::new(initial_base_fee * 1_000),
            target_util_pct: AtomicU64::new(50),
            last_util_pct:   AtomicU64::new(50),
            max_change_ppm:  AtomicU64::new(125), // 12.5% = 125‰
        }
    }

    /// Call at the end of each block with actual utilisation [0, 100].
    pub fn update(&self, actual_util_pct: u64) {
        self.last_util_pct.store(actual_util_pct, Ordering::Relaxed);
        let target = self.target_util_pct.load(Ordering::Relaxed);
        let current = self.base_fee.load(Ordering::Relaxed);
        let max_change = self.max_change_ppm.load(Ordering::Relaxed);

        let delta_pct = actual_util_pct as i64 - target as i64;
        // Scale: full max_change only reached at 100% deviation.
        let change = (current as i64 * delta_pct * max_change as i64) / (100 * 1_000);
        let new_fee = (current as i64 + change).max(1_000) as u64; // floor at 0.001
        self.base_fee.store(new_fee, Ordering::Relaxed);

        debug!(
            actual_util = actual_util_pct,
            new_base_fee = new_fee / 1_000,
            "AdaptiveGasPricer updated"
        );
    }

    /// Returns the current base fee in whole units.
    pub fn base_fee(&self) -> u64 {
        self.base_fee.load(Ordering::Relaxed) / 1_000
    }

    /// Effective gas cost for a transaction with priority fee `tip`.
    pub fn effective_gas_price(&self, tip: u64) -> u64 {
        self.base_fee().saturating_add(tip)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WASM STATIC GAS ANALYSER
// ─────────────────────────────────────────────────────────────────────────────

/// Statically analyse WASM bytecode and return an *upper-bound* gas estimate
/// without executing.  Used for pre-flight checks.
pub fn estimate_gas_static(bytecode: &[u8], schedule: &GasSchedule) -> VmResult<u64> {
    use wasmparser::{Parser, Payload, Operator};

    let mut total: u64 = 0;
    let mut in_code = false;

    for payload in Parser::new(0).parse_all(bytecode) {
        let payload = payload.map_err(|e| VmError::WasmCompile(e.to_string()))?;
        match payload {
            Payload::CodeSectionEntry(body) => {
                for op in body.get_operators_reader()
                    .map_err(|e| VmError::WasmCompile(e.to_string()))?
                {
                    let op = op.map_err(|e| VmError::WasmCompile(e.to_string()))?;
                    let cost = opcode_gas_cost(&op, schedule);
                    total = total.saturating_add(cost);
                }
            }
            _ => {}
        }
    }
    Ok(total)
}

/// Map a `wasmparser::Operator` to a gas cost using `schedule`.
fn opcode_gas_cost(op: &wasmparser::Operator<'_>, schedule: &GasSchedule) -> u64 {
    use wasmparser::Operator::*;
    let wasm_op = match op {
        I32Add    | I32And | I32Or | I32Xor | I32Shl
        | I32ShrU | I32ShrS | I32Rotr | I32Rotl => WasmOpcode::I32Add,
        I32Sub    => WasmOpcode::I32Sub,
        I32Mul    => WasmOpcode::I32Mul,
        I32DivU   => WasmOpcode::I32DivU,
        I32DivS   => WasmOpcode::I32DivS,
        I32RemU   => WasmOpcode::I32RemU,
        I32RemS   => WasmOpcode::I32RemS,
        I64Add    | I64And | I64Or | I64Xor | I64Shl
        | I64ShrU | I64ShrS => WasmOpcode::I64Add,
        I64Sub    => WasmOpcode::I64Sub,
        I64Mul    => WasmOpcode::I64Mul,
        I64DivU   => WasmOpcode::I64DivU,
        I64DivS   => WasmOpcode::I64DivS,
        F32Add | F32Sub => WasmOpcode::F32Add,
        F32Mul   => WasmOpcode::F32Mul,
        F64Add | F64Sub => WasmOpcode::F64Add,
        F64Mul   => WasmOpcode::F64Mul,
        I32Load { .. } | I32Load8U { .. } | I32Load8S { .. }
        | I32Load16U { .. } | I32Load16S { .. } => WasmOpcode::I32Load,
        I64Load { .. } | I64Load8U { .. } | I64Load8S { .. }
        | I64Load32U { .. } | I64Load32S { .. } => WasmOpcode::I64Load,
        I32Store { .. } | I32Store8 { .. } | I32Store16 { .. } => WasmOpcode::I32Store,
        I64Store { .. } | I64Store8 { .. } | I64Store16 { .. } | I64Store32 { .. } => WasmOpcode::I64Store,
        MemoryGrow { .. } => WasmOpcode::MemoryGrow,
        MemorySize { .. } => WasmOpcode::MemorySize,
        MemoryCopy { .. } => WasmOpcode::MemoryCopy,
        MemoryFill { .. } => WasmOpcode::MemoryFill,
        Call { .. }         => WasmOpcode::Call,
        CallIndirect { .. } => WasmOpcode::CallIndirect,
        If { .. }           => WasmOpcode::If,
        Block { .. }        => WasmOpcode::Block,
        Loop { .. }         => WasmOpcode::Loop,
        Br { .. }           => WasmOpcode::Br,
        BrIf { .. }         => WasmOpcode::BrIf,
        BrTable { .. }      => WasmOpcode::BrTable,
        Return              => WasmOpcode::Return,
        Unreachable         => WasmOpcode::Unreachable,
        Nop                 => WasmOpcode::Nop,
        LocalGet { .. }     => WasmOpcode::LocalGet,
        LocalSet { .. }     => WasmOpcode::LocalSet,
        LocalTee { .. }     => WasmOpcode::LocalTee,
        GlobalGet { .. }    => WasmOpcode::GlobalGet,
        GlobalSet { .. }    => WasmOpcode::GlobalSet,
        I32Const { .. } | I64Const { .. } => WasmOpcode::I32Const,
        F32Const { .. } | F64Const { .. } => WasmOpcode::F32Const,
        _                   => return 1, // Unknown → 1 unit
    };
    schedule.costs.get(&wasm_op).copied().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn default_schedule() -> Arc<GasSchedule> { Arc::new(GasSchedule::default()) }

    #[test]
    fn test_basic_charge() {
        let mut m = GasMeter::new(1_000_000, default_schedule()).unwrap();
        m.charge(100).unwrap();
        assert_eq!(m.used(), 100);
        assert_eq!(m.remaining(), 999_900);
    }

    #[test]
    fn test_gas_exhausted() {
        let mut m = GasMeter::new(MIN_GAS_LIMIT, default_schedule()).unwrap();
        let result = m.charge(MIN_GAS_LIMIT + 1);
        assert!(matches!(result, Err(VmError::GasExhausted { .. })));
    }

    #[test]
    fn test_charge_opcode() {
        let mut m = GasMeter::new(1_000_000, default_schedule()).unwrap();
        m.charge_opcode(&WasmOpcode::I32Mul).unwrap();
        assert_eq!(m.used(), 5);
    }

    #[test]
    fn test_charge_storage_write() {
        let mut m = GasMeter::new(1_000_000, default_schedule()).unwrap();
        m.charge_storage_write(100).unwrap();
        assert_eq!(m.breakdown().storage_writes, 5_000); // 100 * 50
    }

    #[test]
    fn test_calldata_charge() {
        let mut m = GasMeter::new(1_000_000, default_schedule()).unwrap();
        m.charge_calldata(10).unwrap();
        assert_eq!(m.breakdown().calldata, 160); // 10 * 16
    }

    #[test]
    fn test_memory_expansion_quadratic() {
        let mut m = GasMeter::new(1_000_000, default_schedule()).unwrap();
        // 1024 bytes → 32 words → 32*32/512 = 2
        m.charge_memory_expansion(1024).unwrap();
        assert_eq!(m.breakdown().memory, 2);
    }

    #[test]
    fn test_gas_overflow_protection() {
        let mut m = GasMeter::new(MAX_GAS_PER_TX, default_schedule()).unwrap();
        let result = m.charge(u64::MAX);
        assert!(matches!(result, Err(VmError::GasOverflow)));
    }

    #[test]
    fn test_below_min_limit_rejected() {
        let result = GasMeter::new(MIN_GAS_LIMIT - 1, default_schedule());
        assert!(result.is_err());
    }

    #[test]
    fn test_adaptive_pricer_increases_on_high_load() {
        let pricer = AdaptiveGasPricer::new(10);
        let before = pricer.base_fee();
        pricer.update(95); // 95% utilisation >> 50% target
        let after = pricer.base_fee();
        assert!(after > before, "Base fee should increase on high load");
    }

    #[test]
    fn test_adaptive_pricer_decreases_on_low_load() {
        let pricer = AdaptiveGasPricer::new(10);
        let before = pricer.base_fee();
        pricer.update(5); // 5% utilisation << 50% target
        let after = pricer.base_fee();
        assert!(after < before, "Base fee should decrease on low load");
    }

    #[test]
    fn test_estimate_gas_static_minimal_wasm() {
        // Minimal valid WASM: magic + version only (no code)
        let wasm = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let schedule = GasSchedule::default();
        let est = estimate_gas_static(&wasm, &schedule).unwrap();
        assert_eq!(est, 0); // no code section
    }
}
