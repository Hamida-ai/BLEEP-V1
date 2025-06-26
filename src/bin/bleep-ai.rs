// src/bin/bleep_ai.rs

use bleep-ai::ai_assistant::AIAssistant;
use bleep-ai::smart_contracts::SmartContractAdvisor;
use bleep-ai::decision_engine::run_decision_loop;
use std::error::Error;
use log::{info, error};

fn main() {
    env_logger::init();
    info!("🧠 BLEEP AI Engine Launching...");

    if let Err(e) = run_ai_services() {
        error!("❌ AI engine failed: {}", e);
        std::process::exit(1);
    }
}

fn run_ai_services() -> Result<(), Box<dyn Error>> {
    // Step 1: Initialize AI Assistant service
    let assistant = AIAssistant::new();
    assistant.bootstrap()?;
    info!("✅ AI Assistant ready.");

    // Step 2: Smart Contract Advisor activation
    let mut advisor = SmartContractAdvisor::new();
    advisor.audit_all()?;
    info!("📋 Smart Contract Advisor ran audits.");

    // Step 3: Begin decision-making loop
    run_decision_loop()?;
    info!("🌀 AI decision loop active.");

    Ok(())
}
