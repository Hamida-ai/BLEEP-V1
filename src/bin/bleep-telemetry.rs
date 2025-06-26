// src/bin/bleep-telemetry.rs

use bleep_telemetry::metrics::{TelemetryCollector, TelemetryConfig};
use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("📡 BLEEP Telemetry Engine Starting...");

    if let Err(e) = run_telemetry_engine() {
        error!("❌ Telemetry engine failed: {}", e);
        std::process::exit(1);
    }
}

fn run_telemetry_engine() -> Result<(), Box<dyn Error>> {
    // Step 1: Load telemetry configuration
    let config = TelemetryConfig::load_or_default()?;
    info!("✅ Telemetry config loaded. Endpoint: {}", config.endpoint);

    // Step 2: Start the telemetry collector
    let mut collector = TelemetryCollector::new(config);
    collector.initialize()?;
    info!("📈 Telemetry collector initialized.");

    // Step 3: Start collection and reporting loop
    collector.run()?;
    info!("📡 Telemetry engine actively collecting metrics.");

    Ok(())
}
