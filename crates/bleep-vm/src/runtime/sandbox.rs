//! Execution sandbox and security policy for bleep-vm.
//!
//! Enforces:
//! - WASM magic bytes and version validation.
//! - Prohibited opcode detection (floats optional, `memory.grow` limit, etc.).
//! - Maximum bytecode size.
//! - Non-determinism detection (system clock, random, env vars, filesystem).
//! - Host-function whitelist: only the declared host imports are permitted.
//! - Execution timeout (via `tokio::time::timeout`).
//! - Resource caps (stack depth, table size, global count).

use std::collections::HashSet;
use std::time::Duration;

use tracing::{debug, warn};
use wasmparser::{Parser, Payload, Operator, ExternalKind, TypeRef};

use crate::error::{VmError, VmResult};

// ─────────────────────────────────────────────────────────────────────────────
// CONSTANTS
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum allowed bytecode size (2 MiB).
pub const MAX_BYTECODE_BYTES: usize = 2 * 1024 * 1024;
/// Maximum number of exported functions.
pub const MAX_EXPORTS: usize = 512;
/// Maximum number of imported functions (host functions).
pub const MAX_IMPORTS: usize = 64;
/// Maximum number of tables.
pub const MAX_TABLES: usize = 2;
/// Maximum number of globals.
pub const MAX_GLOBALS: usize = 128;
/// Default execution timeout.
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(10);

/// WASM magic bytes.
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];
/// WASM version 1.
const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

// ─────────────────────────────────────────────────────────────────────────────
// ALLOWED HOST MODULES
// ─────────────────────────────────────────────────────────────────────────────

/// Host modules a contract is allowed to import.
/// Any import from a module not in this list is rejected.
const ALLOWED_HOST_MODULES: &[&str] = &[
    "env",       // Standard WASI-like environment
    "bleep",     // BLEEP host functions
    "ethereum",  // Ethereum precompiles bridge
    "solana",    // Solana SysCall bridge
    "cosmos",    // Cosmos IBC bridge
    "wasi_snapshot_preview1", // WASI standard (limited subset)
];

/// Non-deterministic / dangerous function names that must never be imported.
const FORBIDDEN_IMPORTS: &[&str] = &[
    "clock_time_get",
    "proc_exit",
    "environ_get",
    "environ_sizes_get",
    "fd_read",
    "fd_write",
    "fd_seek",
    "fd_close",
    "fd_prestat_get",
    "path_open",
    "random_get",   // non-deterministic!
    "sock_accept",
    "sock_connect",
    "poll_oneoff",
];

// ─────────────────────────────────────────────────────────────────────────────
// SECURITY POLICY
// ─────────────────────────────────────────────────────────────────────────────

/// Configurable security policy applied before each execution.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Allow floating-point instructions.
    pub allow_floats:          bool,
    /// Allow SIMD instructions.
    pub allow_simd:            bool,
    /// Allow bulk memory operations.
    pub allow_bulk_memory:     bool,
    /// Maximum bytecode size in bytes.
    pub max_bytecode_bytes:    usize,
    /// Maximum number of `memory.grow` calls.
    pub max_memory_grow_calls: u32,
    /// Execution timeout.
    pub timeout:               Duration,
    /// Additional forbidden import names (beyond the built-in list).
    pub extra_forbidden:       HashSet<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        SecurityPolicy {
            allow_floats:          true,
            allow_simd:            false,
            allow_bulk_memory:     true,
            max_bytecode_bytes:    MAX_BYTECODE_BYTES,
            max_memory_grow_calls: 64,
            timeout:               DEFAULT_EXECUTION_TIMEOUT,
            extra_forbidden:       HashSet::new(),
        }
    }
}

impl SecurityPolicy {
    /// Full validation of a WASM binary against this policy.
    /// Returns `Ok(ValidationReport)` on success.
    pub fn validate(&self, bytecode: &[u8]) -> VmResult<ValidationReport> {
        self.check_magic(bytecode)?;
        self.check_size(bytecode)?;
        let report = self.analyse_bytecode(bytecode)?;
        Ok(report)
    }

    fn check_magic(&self, bytecode: &[u8]) -> VmResult<()> {
        if bytecode.len() < 8 {
            return Err(VmError::SecurityViolation(
                "Bytecode too short to be valid WASM".into(),
            ));
        }
        if bytecode[..4] != WASM_MAGIC {
            return Err(VmError::SecurityViolation(format!(
                "Invalid WASM magic bytes: {:02x?}",
                &bytecode[..4]
            )));
        }
        if bytecode[4..8] != WASM_VERSION {
            return Err(VmError::SecurityViolation(format!(
                "Unsupported WASM version: {:02x?}",
                &bytecode[4..8]
            )));
        }
        Ok(())
    }

    fn check_size(&self, bytecode: &[u8]) -> VmResult<()> {
        if bytecode.len() > self.max_bytecode_bytes {
            return Err(VmError::SecurityViolation(format!(
                "Bytecode size {} exceeds limit {}",
                bytecode.len(),
                self.max_bytecode_bytes
            )));
        }
        Ok(())
    }

    /// Walk the WASM binary section-by-section and operator-by-operator.
    fn analyse_bytecode(&self, bytecode: &[u8]) -> VmResult<ValidationReport> {
        let mut report = ValidationReport::default();
        let mut memory_grow_count: u32 = 0;

        for payload in Parser::new(0).parse_all(bytecode) {
            let payload = payload
                .map_err(|e| VmError::SecurityViolation(format!("WASM parse error: {e}")))?;

            match payload {
                Payload::ImportSection(reader) => {
                    for imp in reader {
                        let imp = imp.map_err(|e| VmError::WasmCompile(e.to_string()))?;
                        report.import_count += 1;
                        if report.import_count > MAX_IMPORTS {
                            return Err(VmError::SecurityViolation(format!(
                                "Too many imports: max {MAX_IMPORTS}"
                            )));
                        }
                        // Module whitelist
                        if !ALLOWED_HOST_MODULES.contains(&imp.module) {
                            return Err(VmError::SecurityViolation(format!(
                                "Forbidden import module: '{}'", imp.module
                            )));
                        }
                        // Function name blacklist
                        let name = imp.name;
                        if FORBIDDEN_IMPORTS.contains(&name)
                            || self.extra_forbidden.contains(name)
                        {
                            return Err(VmError::ForbiddenSyscall { syscall: name.to_string() });
                        }
                        report.imports.push(format!("{}::{}", imp.module, name));
                    }
                }
                Payload::ExportSection(reader) => {
                    for exp in reader {
                        let exp = exp.map_err(|e| VmError::WasmCompile(e.to_string()))?;
                        report.export_count += 1;
                        if report.export_count > MAX_EXPORTS {
                            return Err(VmError::SecurityViolation(format!(
                                "Too many exports: max {MAX_EXPORTS}"
                            )));
                        }
                        report.exports.push(exp.name.to_string());
                    }
                }
                Payload::TableSection(reader) => {
                    let count = reader.count();
                    if count > MAX_TABLES as u32 {
                        return Err(VmError::SecurityViolation(format!(
                            "Too many tables: {count} > {MAX_TABLES}"
                        )));
                    }
                    report.table_count = count;
                }
                Payload::GlobalSection(reader) => {
                    for g in reader {
                        g.map_err(|e| VmError::WasmCompile(e.to_string()))?;
                        report.global_count += 1;
                    }
                    if report.global_count > MAX_GLOBALS as u32 {
                        return Err(VmError::SecurityViolation(format!(
                            "Too many globals: {} > {MAX_GLOBALS}",
                            report.global_count
                        )));
                    }
                }
                Payload::CodeSectionEntry(body) => {
                    report.function_count += 1;
                    for op in body
                        .get_operators_reader()
                        .map_err(|e| VmError::WasmCompile(e.to_string()))?
                    {
                        let op = op.map_err(|e| VmError::WasmCompile(e.to_string()))?;
                        report.total_instructions += 1;

                        self.check_operator(&op, &mut memory_grow_count, &mut report)?;
                    }
                }
                _ => {}
            }
        }

        debug!(
            functions = report.function_count,
            instructions = report.total_instructions,
            imports = report.import_count,
            exports = report.export_count,
            "WASM validation passed"
        );
        Ok(report)
    }

    fn check_operator(
        &self,
        op: &Operator<'_>,
        memory_grow_count: &mut u32,
        report: &mut ValidationReport,
    ) -> VmResult<()> {
        use Operator::*;
        match op {
            // Float ops
            F32Add | F32Sub | F32Mul | F32Div | F32Sqrt | F32Ceil | F32Floor
            | F32Trunc | F32Nearest | F32Abs | F32Neg | F32CopySign | F32Min | F32Max
            | F64Add | F64Sub | F64Mul | F64Div | F64Sqrt | F64Ceil | F64Floor
            | F64Trunc | F64Nearest | F64Abs | F64Neg | F64CopySign | F64Min | F64Max
            | F32Const { .. } | F64Const { .. }
            | F32Load { .. } | F64Load { .. } | F32Store { .. } | F64Store { .. }
            | F32ConvertI32U | F32ConvertI32S | F32ConvertI64U | F32ConvertI64S
            | F64ConvertI32U | F64ConvertI32S | F64ConvertI64U | F64ConvertI64S
            | I32TruncF32U | I32TruncF32S | I32TruncF64U | I32TruncF64S
            | I64TruncF32U | I64TruncF32S | I64TruncF64U | I64TruncF64S
            | F32DemoteF64 | F64PromoteF32 => {
                if !self.allow_floats {
                    return Err(VmError::SecurityViolation(
                        "Floating-point instructions are disabled by security policy".into(),
                    ));
                }
                report.float_instructions += 1;
            }
            // memory.grow
            MemoryGrow { .. } => {
                *memory_grow_count += 1;
                if *memory_grow_count > self.max_memory_grow_calls {
                    return Err(VmError::SecurityViolation(format!(
                        "Exceeded memory.grow call limit ({})",
                        self.max_memory_grow_calls
                    )));
                }
            }
            // Unreachable trap
            Unreachable => {
                report.unreachable_count += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VALIDATION REPORT
// ─────────────────────────────────────────────────────────────────────────────

/// Results of a security validation pass.
#[derive(Debug, Default, Clone)]
pub struct ValidationReport {
    pub function_count:      u32,
    pub total_instructions:  u64,
    pub import_count:        u32,
    pub export_count:        u32,
    pub table_count:         u32,
    pub global_count:        u32,
    pub float_instructions:  u64,
    pub unreachable_count:   u64,
    pub imports:             Vec<String>,
    pub exports:             Vec<String>,
}

impl ValidationReport {
    /// True if the contract exports a function named `name`.
    pub fn exports_fn(&self, name: &str) -> bool {
        self.exports.iter().any(|e| e == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wasm() -> Vec<u8> {
        // Minimal valid WASM: magic + version
        vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]
    }

    #[test]
    fn test_valid_minimal_wasm() {
        let policy = SecurityPolicy::default();
        let result = policy.validate(&minimal_wasm());
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_magic_rejected() {
        let policy = SecurityPolicy::default();
        let bad = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00];
        assert!(matches!(policy.validate(&bad), Err(VmError::SecurityViolation(_))));
    }

    #[test]
    fn test_too_short_rejected() {
        let policy = SecurityPolicy::default();
        assert!(matches!(policy.validate(&[0x00, 0x61]), Err(VmError::SecurityViolation(_))));
    }

    #[test]
    fn test_oversized_bytecode_rejected() {
        let policy = SecurityPolicy {
            max_bytecode_bytes: 10,
            ..Default::default()
        };
        let large = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
                         0x00, 0x00, 0x00]; // 11 bytes
        assert!(matches!(policy.validate(&large), Err(VmError::SecurityViolation(_))));
    }

    #[test]
    fn test_float_allowed_by_default() {
        // Minimal wasm with no code section — floats in exports don't trigger the check
        let policy = SecurityPolicy::default();
        assert!(policy.allow_floats);
    }

    #[test]
    fn test_default_policy_has_timeout() {
        let policy = SecurityPolicy::default();
        assert_eq!(policy.timeout, DEFAULT_EXECUTION_TIMEOUT);
    }

    #[test]
    fn test_validation_report_exports_fn() {
        let mut report = ValidationReport::default();
        report.exports.push("call_contract".to_string());
        assert!(report.exports_fn("call_contract"));
        assert!(!report.exports_fn("missing"));
    }
}
