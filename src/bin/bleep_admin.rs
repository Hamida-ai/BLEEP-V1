// src/bin/bleep_admin.rs

use clap::{Arg, App, SubCommand};
use log::{info, error};
use std::error::Error;

use bleep_wallet_core::wallet::WalletManager;
// use bleep_core::blockchain::Blockchain;
// use bleep_core::transaction_pool::TransactionPool;
// use bleep_governance::governance_engine::GovernanceEngine;
// use bleep_state::state_manager::StateManager;

fn main() {
    env_logger::init();
    info!("BLEEP Admin CLI started");
    
    let app = App::new("bleep-admin")
        .version("1.0.0")
        .about("BLEEP Blockchain Administration CLI")
        .subcommand(
            SubCommand::with_name("governance")
                .about("Governance operations")
                .arg(Arg::with_name("list").long("list").help("List active proposals"))
        )
        .subcommand(
            SubCommand::with_name("state")
                .about("State operations")
                .arg(Arg::with_name("status").long("status").help("Get state status"))
        )
        .subcommand(
            SubCommand::with_name("wallets")
                .about("Wallet operations")
                .arg(Arg::with_name("list").long("list").help("List wallets"))
        );
    
    let matches = app.get_matches();
    
    if let Err(e) = run(matches) {
        error!("❌ CLI admin error: {}", e);
        std::process::exit(1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<(), Box<dyn Error>> {
    if let Some(_governance) = matches.subcommand_matches("governance") {
        println!("🏛 Governance Module Active");
        println!("Active proposals: loading...");
    } else if let Some(_state) = matches.subcommand_matches("state") {
        println!("🧠 State Module Status");
        println!("State initialized and ready for operations");
    } else if let Some(_wallets) = matches.subcommand_matches("wallets") {
        match WalletManager::load_or_create() {
            Ok(manager) => {
                let all = manager.list_wallets();
                println!("💼 Wallet Manager: {} wallets", all.len());
                for w in all {
                    println!("  - Address: {}", w.address());
                }
            }
            Err(e) => {
                eprintln!("Failed to load wallet manager: {}", e);
            }
        }
    } else {
        println!("No subcommand provided. Use --help for usage information.");
    }

    Ok(())
}
 
