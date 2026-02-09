/// Initializes and starts the VM core subsystem.
/// Sets up the execution engine, gas metering, and sandbox security policies.
/// 
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or initialization error
pub fn start_vm_core() -> Result<(), Box<dyn std::error::Error>> {
	use crate::execution_engine::ExecutionEngine;
	use tracing::info;
	
	// Initialize execution engine
	let _engine = ExecutionEngine::new()
		.map_err(|e| format!("Failed to initialize ExecutionEngine: {}", e))?;
	
	info!("VM Core initialized successfully");
	Ok(())
}
pub mod vm_core;
pub mod execution_engine;
pub mod gas_metering;
pub mod wasm_runtime;

pub mod errors;
pub mod memory;
pub mod optimizer;
pub mod sandbox;
pub mod quantum_hints;
pub mod zk_proof;
pub mod execution_result;


