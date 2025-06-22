// src/bin/bleep_governance.rs

use bleep_governance::governance_engine::GovernanceEngine;
use bleep_governance::off_chain_voting::load_off_chain_votes;
use bleep_governance::self_amending::apply_protocol_updates;

use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("🏛️ BLEEP Governance Module Starting...");

    if let Err(e) = run_governance_module() {
        error!("❌ Governance module failed: {}", e);
        std::process::exit(1);
    }
}

fn run_governance_module() -> Result<(), Box<dyn Error>> {
    // Step 1: Initialize governance engine
    let mut engine = GovernanceEngine::load_or_initialize()?;
    info!("✅ Governance engine loaded.");

    // Step 2: Load and process off-chain votes
    let proposals = load_off_chain_votes()?;
    info!("📮 Loaded {} off-chain proposals.", proposals.len());
    engine.process_proposals(proposals);
    info!("✅ Proposals processed.");

    // Step 3: Apply any approved self-amending protocol changes
    apply_protocol_updates()?;
    info!("🔧 Self-amendment logic executed.");

    // Step 4: Persist governance state
    engine.persist()?;
    info!("💾 Governance state saved.");

    info!("🏛️ BLEEP Governance Module completed.");
    Ok(())
}
