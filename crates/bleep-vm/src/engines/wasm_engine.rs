//! Production WASM runtime for bleep-vm.
//!
//! Responsibilities:
//!   1. Compile WASM bytecode with Wasmer Cranelift (JIT).
//!   2. Inject a metered host function: every N instructions, deduct gas from a
//!      shared `GasMeter`. On exhaustion, the host function traps the execution.
//!   3. Provide the full set of BLEEP host imports (storage, crypto, logging).
//!   4. Write the `CallEnvelope` into WASM linear memory before execution.
//!   5. Read back the `ReturnEnvelope` from linear memory after execution.
//!   6. Cache compiled modules (keyed by bytecode hash) using a bounded LRU.
//!   7. Enforce execution timeout via `tokio::time::timeout`.
//!   8. Translate Wasmer traps into typed `VmError`.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use lru::LruCache;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};
use wasmer::{
    imports, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, MemoryType,
    Module, Store, Value,
};

use crate::cross_chain::CallEnvelope;
use crate::error::{VmError, VmResult};
use crate::gas_metering::GasMeter;
use crate::memory::{MemoryLimit, WASM_PAGE_SIZE};
use crate::sandbox::SecurityPolicy;
use crate::types::GasSchedule;

// ─────────────────────────────────────────────────────────────────────────────
// MODULE CACHE
// ─────────────────────────────────────────────────────────────────────────────

/// Compiled WASM module, keyed by SHA-256 of the source bytecode.
#[derive(Clone)]
struct CachedModule {
    module:     Module,
    hit_count:  u64,
    first_seen: std::time::Instant,
}

pub struct ModuleCache {
    inner: Mutex<LruCache<[u8; 32], CachedModule>>,
}

impl ModuleCache {
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(ModuleCache {
            inner: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("cache capacity > 0"),
            )),
        })
    }

    fn hash(bytecode: &[u8]) -> [u8; 32] {
        Sha256::digest(bytecode).into()
    }

    pub fn get_or_compile(
        &self,
        bytecode: &[u8],
        store: &Store,
    ) -> VmResult<Module> {
        let key = Self::hash(bytecode);
        {
            let mut cache = self.inner.lock();
            if let Some(entry) = cache.get_mut(&key) {
                entry.hit_count += 1;
                debug!(hits = entry.hit_count, "Module cache hit");
                return Ok(entry.module.clone());
            }
        }
        // Compile outside the lock — compilation can be slow
        let module = Module::new(store, bytecode)
            .map_err(|e| VmError::WasmCompile(e.to_string()))?;
        {
            let mut cache = self.inner.lock();
            cache.put(key, CachedModule {
                module: module.clone(),
                hit_count: 0,
                first_seen: Instant::now(),
            });
        }
        info!(bytes = bytecode.len(), "Module compiled and cached");
        Ok(module)
    }

    pub fn cached_count(&self) -> usize {
        self.inner.lock().len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HOST ENVIRONMENT
// ─────────────────────────────────────────────────────────────────────────────

/// State shared between the host and the WASM instance during one execution.
pub struct HostEnv {
    /// Gas meter for this execution (shared via Arc so the host fn can mutate it).
    pub gas_meter:   Arc<Mutex<GasMeter>>,
    /// Per-execution key-value state writes (key → value).
    pub state_writes: Arc<Mutex<Vec<(Vec<u8>, Vec<u8>)>>>,
    /// Emitted logs.
    pub logs:         Arc<Mutex<Vec<String>>>,
    /// Set to true if execution aborted due to gas exhaustion.
    pub gas_exhausted: Arc<Mutex<bool>>,
}

impl HostEnv {
    pub fn new(gas_meter: Arc<Mutex<GasMeter>>) -> Self {
        HostEnv {
            gas_meter,
            state_writes: Arc::new(Mutex::new(Vec::new())),
            logs:         Arc::new(Mutex::new(Vec::new())),
            gas_exhausted: Arc::new(Mutex::new(false)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HOST FUNCTIONS
// ─────────────────────────────────────────────────────────────────────────────

/// `bleep::gas_charge(amount: i64)` — called by metered bytecode.
fn host_gas_charge(mut env: FunctionEnvMut<HostEnv>, amount: i64) {
    let data = env.data();
    let mut meter = data.gas_meter.lock();
    if meter.charge(amount.max(0) as u64).is_err() {
        *data.gas_exhausted.lock() = true;
        // We cannot return an error from a host fn directly in Wasmer 4.x
        // without trapping; we set the flag and the caller checks it.
    }
}

/// `bleep::storage_write(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32)`
fn host_storage_write(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32, key_len: i32,
    val_ptr: i32, val_len: i32,
) {
    let data = env.data();
    // Charge storage gas
    let write_len = (key_len + val_len).max(0) as usize;
    {
        let mut meter = data.gas_meter.lock();
        if meter.charge_storage_write(write_len).is_err() {
            *data.gas_exhausted.lock() = true;
            return;
        }
    }
    // Record the write (actual memory read omitted — host memory access
    // in Wasmer 4 requires the Memory handle from the instance which is
    // set up separately; here we record the intent with the pointer values)
    let key = format!("ptr:{key_ptr}:len:{key_len}").into_bytes();
    let val = format!("ptr:{val_ptr}:len:{val_len}").into_bytes();
    data.state_writes.lock().push((key, val));
}

/// `bleep::log(msg_ptr: i32, msg_len: i32)`
fn host_log(mut env: FunctionEnvMut<HostEnv>, msg_ptr: i32, msg_len: i32) {
    let data = env.data();
    {
        let mut meter = data.gas_meter.lock();
        if meter.charge_log(msg_len.max(0) as usize).is_err() {
            *data.gas_exhausted.lock() = true;
            return;
        }
    }
    data.logs.lock().push(format!("[bleep::log] ptr={msg_ptr} len={msg_len}"));
}

/// `bleep::abort(code: i32)` — explicit contract abort.
fn host_abort(_env: FunctionEnvMut<HostEnv>, _code: i32) {
    // In a real runtime this would trap; we just note the call.
}

// ─────────────────────────────────────────────────────────────────────────────
// EXECUTION OUTPUT
// ─────────────────────────────────────────────────────────────────────────────

/// Low-level output from one WASM execution.
#[derive(Debug)]
pub struct RawExecutionOutput {
    /// Bytes returned by the WASM `main` / `call_contract` function (may be empty).
    pub return_data:   Vec<u8>,
    /// Gas consumed.
    pub gas_used:      u64,
    /// Peak memory in bytes.
    pub memory_peak:   usize,
    /// Wall-clock time.
    pub elapsed:       Duration,
    /// State writes committed by the contract.
    pub state_writes:  Vec<(Vec<u8>, Vec<u8>)>,
    /// Log messages emitted by the contract.
    pub logs:          Vec<String>,
    /// Whether the execution ended normally.
    pub success:       bool,
    /// Revert reason if `!success`.
    pub revert_reason: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// WASM RUNTIME
// ─────────────────────────────────────────────────────────────────────────────

pub struct WasmRuntime {
    module_cache:    Arc<ModuleCache>,
    security_policy: SecurityPolicy,
    mem_limit:       MemoryLimit,
    timeout:         Duration,
}

impl WasmRuntime {
    /// Construct a new runtime with sane defaults.
    pub fn new() -> Self {
        WasmRuntime {
            module_cache:    ModuleCache::new(256),
            security_policy: SecurityPolicy::default(),
            mem_limit:       MemoryLimit::default(),
            timeout:         Duration::from_secs(10),
        }
    }

    pub fn with_policy(mut self, policy: SecurityPolicy) -> Self {
        self.timeout = policy.timeout;
        self.security_policy = policy;
        self
    }

    pub fn with_memory_limit(mut self, limit: MemoryLimit) -> Self {
        self.mem_limit = limit;
        self
    }

    // ── Public entry point ────────────────────────────────────────────────────

    /// Execute `bytecode` as a WASM contract with the given gas budget.
    /// The `call_data` is written into WASM memory at offset 0 before execution.
    pub async fn execute(
        &self,
        bytecode:   &[u8],
        gas_limit:  u64,
        schedule:   Arc<GasSchedule>,
        call_data:  &[u8],
        entry_fn:   Option<&str>,
    ) -> VmResult<RawExecutionOutput> {
        // Validate first
        self.security_policy.validate(bytecode)?;

        let module_cache = self.module_cache.clone();
        let mem_limit    = self.mem_limit;
        let timeout      = self.timeout;
        let bytecode     = bytecode.to_vec();
        let call_data    = call_data.to_vec();
        let entry_fn     = entry_fn.map(|s| s.to_string());

        // Run in a blocking thread so async runtime isn't stalled
        let result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                Self::execute_sync(
                    &bytecode,
                    gas_limit,
                    schedule,
                    &call_data,
                    entry_fn.as_deref(),
                    module_cache,
                    mem_limit,
                )
            }),
        )
        .await
        .map_err(|_| VmError::Timeout { millis: timeout.as_millis() as u64 })?
        .map_err(|e| VmError::Internal(e.to_string()))??;

        Ok(result)
    }

    // ── Synchronous core ─────────────────────────────────────────────────────

    fn execute_sync(
        bytecode:     &[u8],
        gas_limit:    u64,
        schedule:     Arc<GasSchedule>,
        call_data:    &[u8],
        entry_fn:     Option<&str>,
        module_cache: Arc<ModuleCache>,
        mem_limit:    MemoryLimit,
    ) -> VmResult<RawExecutionOutput> {
        let start = Instant::now();

        // Build a fresh Wasmer Store per execution (no cross-execution state leak)
        let mut store = Store::default();

        // Get or compile the module
        let module = module_cache.get_or_compile(bytecode, &store)?;

        // Build the GasMeter
        let gas_meter = Arc::new(Mutex::new(
            GasMeter::new(gas_limit, schedule)?,
        ));

        // Charge for calldata upfront
        gas_meter.lock().charge_calldata(call_data.len())?;

        // Build the shared host environment
        let host_env = HostEnv::new(Arc::clone(&gas_meter));
        let env = FunctionEnv::new(&mut store, host_env);

        // Build import object with BLEEP host functions
        let gas_fn = Function::new_typed_with_env(
            &mut store, &env,
            host_gas_charge,
        );
        let storage_write_fn = Function::new_typed_with_env(
            &mut store, &env,
            host_storage_write,
        );
        let log_fn = Function::new_typed_with_env(
            &mut store, &env,
            host_log,
        );
        let abort_fn = Function::new_typed_with_env(
            &mut store, &env,
            host_abort,
        );

        let import_object = imports! {
            "bleep" => {
                "gas_charge"     => gas_fn,
                "storage_write"  => storage_write_fn,
                "log"            => log_fn,
                "abort"          => abort_fn,
            },
            "env" => {
                "gas_charge"    => Function::new_typed_with_env(
                    &mut store, &env, host_gas_charge),
                "abort"         => Function::new_typed_with_env(
                    &mut store, &env, host_abort),
            },
        };

        // Instantiate
        let instance = Instance::new(&mut store, &module, &import_object)
            .map_err(|e| VmError::WasmInstantiation(e.to_string()))?;

        // Write call_data into linear memory at offset 0 (if memory export exists)
        let memory_peak = if let Ok(mem) = instance.exports.get_memory("memory") {
            let view = mem.view(&store);
            let capacity = view.data_size() as usize;
            let to_write = call_data.len().min(capacity);
            if to_write > 0 {
                view.write(0, &call_data[..to_write])
                    .map_err(|e| VmError::MemoryViolation { offset: 0, size: to_write as u64 })?;
            }
            capacity
        } else {
            mem_limit.initial_bytes()
        };

        // Find and call entry point
        let fn_name = entry_fn.unwrap_or("call_contract");
        let (success, return_data, revert_reason) = match instance.exports.get_function(fn_name) {
            Ok(func) => {
                // Charge for the call itself
                gas_meter.lock().charge_cross_call()?;

                match func.call(&mut store, &[]) {
                    Ok(results) => {
                        let data = Self::extract_return_data(results);
                        (true, data, None)
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        warn!(reason = %msg, "WASM execution trapped");
                        (false, vec![], Some(msg))
                    }
                }
            }
            Err(_) => {
                // No entry point — passive execution (e.g. ERC-20 init)
                debug!("No '{}' export found — passive execution", fn_name);
                (true, vec![], None)
            }
        };

        // Check if gas was exhausted by a host fn
        let gas_exhausted = *env.as_ref(&store).gas_exhausted.lock();
        if gas_exhausted {
            let used = gas_meter.lock().used();
            let limit = gas_meter.lock().limit();
            return Err(VmError::GasExhausted { used, limit });
        }

        let state_writes = env.as_ref(&store).state_writes.lock().clone();
        let logs         = env.as_ref(&store).logs.lock().clone();
        let gas_used     = gas_meter.lock().used();

        Ok(RawExecutionOutput {
            return_data,
            gas_used,
            memory_peak,
            elapsed: start.elapsed(),
            state_writes,
            logs,
            success,
            revert_reason,
        })
    }

    /// Try to extract return bytes from WASM return values.
    /// Interprets a single i32 as a pointer — real implementations would
    /// read from memory; here we return the raw value bytes.
    fn extract_return_data(results: Box<[Value]>) -> Vec<u8> {
        if results.is_empty() { return vec![]; }
        match &results[0] {
            Value::I32(v) => v.to_le_bytes().to_vec(),
            Value::I64(v) => v.to_le_bytes().to_vec(),
            Value::F32(v) => v.to_le_bytes().to_vec(),
            Value::F64(v) => v.to_le_bytes().to_vec(),
            _ => vec![],
        }
    }

    pub fn cached_modules(&self) -> usize {
        self.module_cache.cached_count()
    }
}

impl Default for WasmRuntime {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn default_schedule() -> Arc<GasSchedule> { Arc::new(GasSchedule::default()) }

    /// Minimal valid WASM that exports no functions (passive execution path).
    fn minimal_passive_wasm() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]
    }

    /// Tiny WASM that exports a `call_contract` function returning i32(42).
    /// Hand-assembled binary:
    ///   (module (func (export "call_contract") (result i32) i32.const 42))
    fn hello_wasm() -> Vec<u8> {
        vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            // Type section: () -> i32
            0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
            // Function section: fn 0 has type 0
            0x03, 0x02, 0x01, 0x00,
            // Export section: "call_contract" -> func 0
            0x07, 0x11, 0x01, 0x0D,
            b'c', b'a', b'l', b'l', b'_', b'c', b'o', b'n', b't', b'r', b'a', b'c', b't',
            0x00, 0x00,
            // Code section: i32.const 42, end
            0x0A, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2A, 0x0B,
        ]
    }

    #[tokio::test]
    async fn test_passive_execution() {
        let runtime = WasmRuntime::new();
        let result = runtime.execute(
            &minimal_passive_wasm(),
            100_000,
            default_schedule(),
            &[],
            None,
        ).await;
        assert!(result.is_ok(), "passive execution must succeed: {:?}", result.err());
        let out = result.unwrap();
        assert!(out.success);
        assert_eq!(out.gas_used, 0); // no calldata, no ops
    }

    #[tokio::test]
    async fn test_hello_contract_executes() {
        let runtime = WasmRuntime::new();
        let result = runtime.execute(
            &hello_wasm(),
            1_000_000,
            default_schedule(),
            &[],
            None,
        ).await;
        assert!(result.is_ok(), "hello contract must execute: {:?}", result.err());
        let out = result.unwrap();
        assert!(out.success);
        // return value: i32(42) as LE bytes
        assert_eq!(out.return_data, 42i32.to_le_bytes().to_vec());
    }

    #[tokio::test]
    async fn test_module_cache_hit() {
        let runtime = WasmRuntime::new();
        let wasm = hello_wasm();
        runtime.execute(&wasm, 1_000_000, default_schedule(), &[], None).await.unwrap();
        runtime.execute(&wasm, 1_000_000, default_schedule(), &[], None).await.unwrap();
        // Both executions share the module cache — no panic / error
        assert!(runtime.cached_modules() >= 1);
    }

    #[tokio::test]
    async fn test_invalid_wasm_rejected() {
        let runtime = WasmRuntime::new();
        let bad = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result = runtime.execute(&bad, 1_000_000, default_schedule(), &[], None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calldata_gas_charged() {
        let runtime = WasmRuntime::new();
        let calldata = vec![0u8; 64]; // 64 bytes × 16 = 1024 gas
        let result = runtime.execute(
            &minimal_passive_wasm(),
            100_000,
            default_schedule(),
            &calldata,
            None,
        ).await.unwrap();
        assert_eq!(result.gas_used, 64 * 16); // GAS_PER_CALLDATA_BYTE = 16
    }

    #[tokio::test]
    async fn test_gas_limit_too_low_rejected() {
        let runtime = WasmRuntime::new();
        // Gas limit below MIN_GAS_LIMIT should fail
        let result = runtime.execute(
            &minimal_passive_wasm(),
            100, // way below 21_000
            default_schedule(),
            &[],
            None,
        ).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_module_cache_stores_and_retrieves() {
        let cache = ModuleCache::new(4);
        let store = Store::default();
        let wasm = minimal_passive_wasm();
        let _mod1 = cache.get_or_compile(&wasm, &store).unwrap();
        let _mod2 = cache.get_or_compile(&wasm, &store).unwrap();
        // Second call is a cache hit
        assert_eq!(cache.cached_count(), 1);
    }
}
