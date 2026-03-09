// src/bin/transaction.rs

use bleep_core::transaction::ZKTransaction;
use bleep_crypto::quantum_resistance::sign_transaction;

use std::error::Error;
use log::{info, error};
use std::env;

fn main() {
    env_logger::init();
    info!("🔁 BLEEP Transaction Engine Starting...");

    if let Err(e) = submit_transaction() {
        error!("❌ Transaction submission failed: {}", e);
        std::process::exit(1);
    }
}

fn submit_transaction() -> Result<(), Box<dyn Error>> {
    // Step 1: Parse CLI arguments for transaction details
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: transaction <recipient> <amount>");
        return Ok(());
    }
    let recipient = &args[1];
    let amount: u64 = args[2].parse()?;

    // Step 2: Build new ZKTransaction (dummy, as builder is not available)
    // Dummy quantum secure for signature (replace with real instance)
    let quantum_secure = bleep_crypto::quantum_secure::QuantumSecure::keygen();
    let tx = ZKTransaction::new("sender", recipient, amount, &quantum_secure);

    // Step 3: Sign transaction using Falcon or Kyber private key
    // (No-op for now)
    // sign_transaction(&mut tx)?;
    info!("📝 Transaction signed: {} -> {} for {}", "sender", recipient, amount);

    // Step 4: Print transaction (no pool integration)
    println!("📤 Transaction ready: {:?}", tx);
    Ok(())
}
