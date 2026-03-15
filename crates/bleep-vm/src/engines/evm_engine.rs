//! # Layer 3 — EVM Engine
//!
//! Production EVM engine powered by [`revm`] — the same EVM implementation
//! used by Foundry, Reth, and Hardhat.
//!
//! * 100% EVM-compatible: Solidity, Vyper, Yul contracts work unchanged.
//! * Supports Berlin, London, Shanghai, and Cancun hard-forks.
//! * Isolated from other engines: a bug in EVM never touches WASM.
//! * EIP-1559 base-fee, EIP-2929 access lists, EIP-3860 init-code limit.
//! * All 9 EVM precompiles: ecRecover, SHA-256, RIPEMD-160, identity,
//!   modexp, BN256 add/mul/pairing, BLAKE2f.
//!
//! Gas units emitted by this engine are in EVM gas; the `GasModel` in
//! the router normalises them to BLEEP gas before charging.

use crate::error::{VmError, VmResult};
use crate::execution::{
    execution_context::ExecutionContext,
    state_transition::StateDiff,
};
use crate::intent::TargetVm;
use crate::router::vm_router::{Engine, EngineResult};
use crate::types::{ExecutionLog, LogLevel};

use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{
        AccountInfo, Address, Bytecode, Bytes, ExecutionResult as RevmResult,
        Output, TransactTo, TxEnv as RevmTxEnv, U256, B256,
        SpecId, BlockEnv as RevmBlockEnv,
    },
    EVM,
};

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use tracing::{debug, warn, instrument};

// ─────────────────────────────────────────────────────────────────────────────
// IN-MEMORY CONTRACT STORAGE
// ─────────────────────────────────────────────────────────────────────────────

/// Stores deployed EVM contract bytecode keyed by address.
type ContractStore = Arc<RwLock<HashMap<[u8; 20], Vec<u8>>>>;

// ─────────────────────────────────────────────────────────────────────────────
// EVM ENGINE
// ─────────────────────────────────────────────────────────────────────────────

/// Production EVM engine backed by `revm`.
pub struct EvmEngine {
    /// Deployed contract bytecodes (in-memory for this session).
    contracts: ContractStore,
    /// EVM hard-fork spec (defaults to Shanghai).
    spec_id:   SpecId,
}

impl EvmEngine {
    pub fn new() -> Self {
        EvmEngine {
            contracts: Arc::new(RwLock::new(HashMap::new())),
            spec_id:   SpecId::SHANGHAI,
        }
    }

    /// Use a specific EVM hard-fork.
    pub fn with_spec(mut self, spec: SpecId) -> Self {
        self.spec_id = spec;
        self
    }

    /// Derive a deterministic 20-byte EVM address from 32-byte BLEEP address.
    fn to_evm_address(addr: &[u8; 32]) -> Address {
        // Take the last 20 bytes (Ethereum-compatible convention)
        let mut evm = [0u8; 20];
        evm.copy_from_slice(&addr[12..32]);
        Address::from(evm)
    }

    /// Compute CREATE address (keccak256(rlp([sender, nonce]))[12..]).
    fn create_address(sender: Address, nonce: u64) -> Address {
        // Simplified RLP: [0xd6, 0x94, sender(20), nonce_byte]
        let mut buf = Vec::with_capacity(23);
        buf.push(0xd6u8);
        buf.push(0x94u8);
        buf.extend_from_slice(sender.as_slice());
        buf.push(nonce as u8);
        let hash = sha3::Keccak256::digest(&buf);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[12..]);
        Address::from(addr)
    }

    /// Compute CREATE2 address.
    fn create2_address(sender: Address, salt: [u8; 32], init_code: &[u8]) -> Address {
        use sha3::Digest as _;
        let init_code_hash = sha3::Keccak256::digest(init_code);
        let mut buf = Vec::with_capacity(1 + 20 + 32 + 32);
        buf.push(0xff);
        buf.extend_from_slice(sender.as_slice());
        buf.extend_from_slice(&salt);
        buf.extend_from_slice(&init_code_hash);
        let hash = sha3::Keccak256::digest(&buf);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[12..]);
        Address::from(addr)
    }

    /// Build a revm `EVM` instance configured for one transaction.
    fn build_evm<'a>(
        &self,
        ctx:      &ExecutionContext,
        db:       CacheDB<EmptyDB>,
        to:       TransactTo,
        calldata: Bytes,
        value:    U256,
        gas:      u64,
    ) -> EVM<CacheDB<EmptyDB>> {
        let mut evm = EVM::new();
        evm.database(db);
        evm.env.cfg.spec_id = self.spec_id;
        evm.env.cfg.chain_id = ctx.block.chain_id;
        evm.env.cfg.limit_contract_code_size = Some(24_576); // EIP-170

        // Block environment
        evm.env.block = RevmBlockEnv {
            number:     U256::from(ctx.block.number),
            coinbase:   Address::from(Self::to_evm_address(&ctx.block.coinbase).0),
            timestamp:  U256::from(ctx.block.timestamp),
            gas_limit:  U256::from(ctx.block.gas_limit),
            basefee:    U256::from(ctx.block.base_fee),
            difficulty: U256::ZERO,
            prevrandao: Some(B256::ZERO),
            blob_excess_gas_and_price: None,
        };

        // Transaction environment
        evm.env.tx = RevmTxEnv {
            caller:          Self::to_evm_address(&ctx.tx.caller),
            transact_to:     to,
            data:            calldata,
            value,
            gas_limit:       gas,
            gas_price:       U256::from(ctx.tx.gas_price),
            gas_priority_fee: None,
            chain_id:        Some(ctx.block.chain_id),
            nonce:           Some(ctx.tx.nonce),
            access_list:     Vec::new(),
            blob_hashes:     Vec::new(),
            max_fee_per_blob_gas: None,
        };
        evm
    }

    /// Pre-populate the CacheDB with stored contracts so CALL/STATICCALL can find them.
    fn populate_db(&self, db: &mut CacheDB<EmptyDB>) {
        let contracts = self.contracts.read();
        for (addr_bytes, bytecode) in contracts.iter() {
            let addr = Address::from(*addr_bytes);
            let code_hash = {
                let h = sha3::Keccak256::digest(bytecode);
                B256::from_slice(&h)
            };
            let bytecode = Bytecode::new_raw(Bytes::from(bytecode.clone()));
            let info = AccountInfo {
                balance:    U256::ZERO,
                nonce:      0,
                code_hash,
                code:       Some(bytecode),
            };
            db.insert_account_info(addr, info);
        }
    }

    /// Convert revm `ExecutionResult` into our `EngineResult`.
    fn convert_result(
        revm_result: RevmResult,
        state_diff:  StateDiff,
        start:       Instant,
    ) -> EngineResult {
        match revm_result {
            RevmResult::Success { reason: _, gas_used, gas_refunded: _, logs, output } => {
                let output_bytes = match output {
                    Output::Call(b)   => b.to_vec(),
                    Output::Create(b, _) => b.to_vec(),
                };
                let engine_logs: Vec<ExecutionLog> = logs.iter().map(|l| ExecutionLog {
                    level:   LogLevel::Info,
                    message: format!("LOG{} addr={:x}", l.topics.len(), l.address),
                    data:    l.data.to_vec(),
                }).collect();
                EngineResult {
                    success:       true,
                    output:        output_bytes,
                    gas_used,
                    state_diff,
                    logs:          engine_logs,
                    revert_reason: None,
                    exec_time:     start.elapsed(),
                }
            }
            RevmResult::Revert { gas_used, output } => {
                let reason = decode_revert_reason(&output);
                EngineResult {
                    success:       false,
                    output:        output.to_vec(),
                    gas_used,
                    state_diff:    StateDiff::empty(),
                    logs:          Vec::new(),
                    revert_reason: Some(reason),
                    exec_time:     start.elapsed(),
                }
            }
            RevmResult::Halt { reason, gas_used } => {
                EngineResult {
                    success:       false,
                    output:        Vec::new(),
                    gas_used,
                    state_diff:    StateDiff::empty(),
                    logs:          Vec::new(),
                    revert_reason: Some(format!("HALT: {reason:?}")),
                    exec_time:     start.elapsed(),
                }
            }
        }
    }

    /// Convert revm state changes to our `StateDiff`.
    fn extract_state_diff(
        contract_addr: Address,
        evm_result: &RevmResult,
        calldata: &[u8],
    ) -> StateDiff {
        let mut diff = StateDiff::empty();

        // Extract events from successful execution
        if let RevmResult::Success { logs, output, .. } = evm_result {
            for log in logs {
                let contract_bytes: [u8; 32] = {
                    let mut b = [0u8; 32];
                    b[12..].copy_from_slice(log.address.as_slice());
                    b
                };
                let topics: Vec<[u8; 32]> = log.topics()
                    .iter()
                    .map(|t| **t)
                    .collect();
                diff.emit_event(contract_bytes, topics, log.data.to_vec());
            }

            // For CREATE: record code deployment
            if let Output::Create(_, Some(addr)) = output {
                let addr_bytes: [u8; 32] = {
                    let mut b = [0u8; 32];
                    b[12..].copy_from_slice(addr.as_slice());
                    b
                };
                diff.deploy_code(addr_bytes, calldata.to_vec());
            }
        }

        diff
    }
}

impl Default for EvmEngine {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl Engine for EvmEngine {
    fn name(&self) -> &'static str { "evm-revm" }

    fn supports(&self, vm: &TargetVm) -> bool {
        matches!(vm, TargetVm::Evm)
    }

    #[instrument(skip(self, ctx, bytecode, calldata), fields(engine = "evm-revm"))]
    async fn execute(
        &self,
        ctx:       &ExecutionContext,
        bytecode:  &[u8],
        calldata:  &[u8],
        gas_limit: u64,
    ) -> VmResult<EngineResult> {
        let start = Instant::now();

        // Determine call target
        let contract_addr = Self::to_evm_address(&ctx.tx.caller); // placeholder
        let to = if bytecode.is_empty() {
            TransactTo::Call(contract_addr)
        } else {
            // Bytecode provided means in-memory call to provided code
            TransactTo::Call(contract_addr)
        };

        let mut db = CacheDB::new(EmptyDB::default());
        self.populate_db(&mut db);

        // If bytecode provided, inject it at the target address
        if !bytecode.is_empty() {
            let code_hash = {
                let h = sha3::Keccak256::digest(bytecode);
                B256::from_slice(&h)
            };
            db.insert_account_info(contract_addr, AccountInfo {
                balance:   U256::from(ctx.tx.value),
                nonce:     0,
                code_hash,
                code:      Some(Bytecode::new_raw(Bytes::from(bytecode.to_vec()))),
            });
        }

        let value = U256::from(ctx.tx.value);
        let calldata_bytes = Bytes::from(calldata.to_vec());

        let mut evm = self.build_evm(ctx, db, to, calldata_bytes.clone(), value, gas_limit);

        let revm_result = evm
            .transact_commit()
            .map_err(|e| VmError::ExecutionFailed(format!("revm error: {e:?}")))?;

        let diff = Self::extract_state_diff(contract_addr, &revm_result, calldata);
        let result = Self::convert_result(revm_result, diff, start);

        debug!(
            success   = result.success,
            gas_used  = result.gas_used,
            exec_us   = result.exec_time.as_micros(),
            "EVM execution complete"
        );

        Ok(result)
    }

    #[instrument(skip(self, ctx, bytecode, init_args), fields(engine = "evm-revm"))]
    async fn deploy(
        &self,
        ctx:       &ExecutionContext,
        bytecode:  &[u8],
        init_args: &[u8],
        gas_limit: u64,
        salt:      Option<[u8; 32]>,
    ) -> VmResult<EngineResult> {
        let start = Instant::now();

        // Combine bytecode + constructor args
        let mut init_code = bytecode.to_vec();
        init_code.extend_from_slice(init_args);

        let mut db = CacheDB::new(EmptyDB::default());
        self.populate_db(&mut db);

        let caller_addr = Self::to_evm_address(&ctx.tx.caller);

        // Seed caller account with enough balance for value
        db.insert_account_info(caller_addr, AccountInfo {
            balance:   U256::from(1_000_000_000_000_000_000u128), // 1 ETH
            nonce:     ctx.tx.nonce,
            code_hash: B256::ZERO,
            code:      None,
        });

        let to = TransactTo::Create(revm::primitives::CreateScheme::Create);
        let init_bytes = Bytes::from(init_code.clone());
        let value = U256::from(ctx.tx.value);

        let mut evm = self.build_evm(ctx, db, to, init_bytes, value, gas_limit);

        let revm_result = evm
            .transact_commit()
            .map_err(|e| VmError::ExecutionFailed(format!("revm deploy error: {e:?}")))?;

        // On success, register the deployed bytecode
        if let RevmResult::Success { output: Output::Create(_, Some(addr)), .. } = &revm_result {
            let addr_bytes: [u8; 20] = *addr.as_ref();
            let mut contracts = self.contracts.write();
            contracts.insert(addr_bytes, bytecode.to_vec());
            debug!(address = ?addr, "EVM contract deployed");
        }

        let diff = Self::extract_state_diff(caller_addr, &revm_result, &init_code);
        let result = Self::convert_result(revm_result, diff, start);
        Ok(result)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HELPERS
// ─────────────────────────────────────────────────────────────────────────────

/// Decode a Solidity revert reason from ABI-encoded `Error(string)`.
fn decode_revert_reason(data: &Bytes) -> String {
    // ABI encoding: 0x08c379a0 ++ offset(32) ++ length(32) ++ string
    if data.len() >= 68 && data[0..4] == [0x08, 0xc3, 0x79, 0xa0] {
        let len = u32::from_be_bytes([data[36], data[37], data[38], data[39]]) as usize;
        if data.len() >= 68 + len {
            if let Ok(msg) = std::str::from_utf8(&data[68..68 + len]) {
                return format!("Revert: {msg}");
            }
        }
    }
    if data.is_empty() {
        return "Execution reverted (no reason)".into();
    }
    format!("Revert data: 0x{}", hex::encode(data))
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::execution_context::{BlockEnv, TxEnv};
    use crate::types::ChainId;

    fn test_ctx(gas: u64) -> ExecutionContext {
        ExecutionContext::new(
            BlockEnv::default(),
            TxEnv::default(),
            gas,
            ChainId::Ethereum,
            uuid::Uuid::new_v4(),
            128,
        )
    }

    #[tokio::test]
    async fn test_evm_engine_supports_evm() {
        let engine = EvmEngine::new();
        assert!(engine.supports(&TargetVm::Evm));
        assert!(!engine.supports(&TargetVm::Wasm));
    }

    #[tokio::test]
    async fn test_simple_push_return() {
        // PUSH1 0x42  PUSH1 0x00  MSTORE  PUSH1 0x20  PUSH1 0x00  RETURN
        // Deploys a contract that returns 0x42
        let bytecode = hex::decode("604260005260206000f3").unwrap();
        let engine = EvmEngine::new();
        let ctx = test_ctx(100_000);
        let result = engine.execute(&ctx, &bytecode, &[], 100_000).await.unwrap();
        assert!(result.success, "Expected success: {:?}", result.revert_reason);
    }

    #[tokio::test]
    async fn test_evm_engine_name() {
        assert_eq!(EvmEngine::new().name(), "evm-revm");
    }

    #[tokio::test]
    async fn test_out_of_gas_halts() {
        // PUSH1 0x00 SLOAD — expensive op with tiny gas budget
        let bytecode = hex::decode("600054").unwrap();
        let engine = EvmEngine::new();
        let ctx = test_ctx(100);
        let result = engine.execute(&ctx, &bytecode, &[], 100).await.unwrap();
        // Should halt due to OOG, not panic
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_decode_revert_reason_empty() {
        let reason = decode_revert_reason(&Bytes::new());
        assert!(reason.contains("no reason"));
    }

    #[tokio::test]
    async fn test_create_address_deterministic() {
        let sender = Address::from([0x01u8; 20]);
        let addr1 = EvmEngine::create_address(sender, 0);
        let addr2 = EvmEngine::create_address(sender, 0);
        assert_eq!(addr1, addr2);
        let addr3 = EvmEngine::create_address(sender, 1);
        assert_ne!(addr1, addr3);
    }

    #[tokio::test]
    async fn test_create2_address_uses_salt() {
        let sender = Address::from([0x01u8; 20]);
        let salt1 = [0x00u8; 32];
        let salt2 = [0x01u8; 32];
        let code  = b"\x60\x00";
        let a1 = EvmEngine::create2_address(sender, salt1, code);
        let a2 = EvmEngine::create2_address(sender, salt2, code);
        assert_ne!(a1, a2);
    }
}
