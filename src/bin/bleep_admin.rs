// src/bin/bleep_admin.rs

use clap::{Arg, App, SubCommand};
use log::{info, error};
use std::error::Error;

use bleep_core::blockchain::Blockchain;
use bleep_core::transaction_pool::TransactionPool;
use bleep_governance::governance_engine::GovernanceEngine;
use bleep_state::state_manager::StateManager;
use bleep_wallet_core::wallet::WalletManager;

fn main() {
    env_logger::init();

    let matches = App::new("BLEEP Admin CLI")
        .version("1.0")
        .author("Bleep Tech")
        .about("Manage and query the BLEEP blockchain")
        .subcommand(SubCommand::with_name("status").about("Show current chain and node status"))
        .subcommand(SubCommand::with_name("mempool").about("List transactions in the mempool"))
        .subcommand(SubCommand::with_name("governance").about("Display active governance proposals"))
        .subcommand(SubCommand::with_name("state").about("Show latest state snapshot info"))
        .subcommand(SubCommand::with_name("wallets").about("List managed wallets"))
        .get_matches();

    if let Err(e) = run(matches) {
        error!("❌ CLI admin error: {}", e);
        std::process::exit(1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<(), Box<dyn Error>> {
    if matches.subcommand_matches("status").is_some() {
        let chain = Blockchain::load_or_initialize()?;
        println!("✔ Chain height: {} | Latest hash: {}", chain.len(), chain.latest_block_hash());
    }

    if matches.subcommand_matches("mempool").is_some() {
        let pool = TransactionPool::load()?;
        println!("📨 Mempool contains {} txs", pool.len());
    }

    if matches.subcommand_matches("governance").is_some() {
        let gov = GovernanceEngine::load_or_initialize()?;
        let proposals = gov.list_active_proposals();
        println!("🏛 Active Proposals ({}):", proposals.len());
        for p in proposals { println!("- {}", p.title); }
    }

    if matches.subcommand_matches("state").is_some() {
        let state = StateManager::load_latest()?;
        println!("🧠 State root: {} | Snapshot: {}", state.state_root(), state.snapshot_timestamp());
    }

    if matches.subcommand_matches("wallets").is_some() {
        let manager = WalletManager::load_or_create()?;
        let all = manager.list_wallets();
        println!("💼 {} wallets managed:", all.len());
        for w in all { println!("- {}", w.address()); }
    }

    Ok(())
}
 
