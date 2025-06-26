// src/bin/bleep_interop.rs

use bleep-interop::interoperability::{InteropEngine, InteropConfig};
use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("🌉 BLEEP Interop Engine Starting...");

    if let Err(e) = run_interop_engine() {
        error!("❌ Interop engine failed: {}", e);
        std::process::exit(1);
    }
}

fn run_interop_engine() -> Result<(), Box<dyn Error>> {
    // Step 1: Load interoperability configuration
    let config = InteropConfig::load_or_default()?;
    info!("✅ Loaded interop config for external network: {}", config.target_chain);

    // Step 2: Initialize interop engine
    let mut interop = InteropEngine::new(config);
    interop.initialize()?;
    info!("🔧 Interop engine initialized.");

    // Step 3: Perform sync or cross-chain state exchange
    interop.perform_handshake()?;
    interop.sync_state()?;
    info!("🔁 State synchronized with external chain.");

    Ok(())
}
 
