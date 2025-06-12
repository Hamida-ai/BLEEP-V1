use clap::{Parser, Subcommand};
use tracing::{info, error};
use anyhow::Result;

use bleep_core::blockchain;
use bleep_wallet_core::wallet_core;
use bleep_p2p::p2p_network;
use bleep_consensus::consensus;
use bleep_governance::governance_engine;
use bleep_ai::{ai_assistant, machine_learning};
use bleep_crypto::zkp_verification;
use bleep_state::state_manager;
use bleep_telemetry::telemetry;
use bleep_pat::pat_engine;

#[derive(Parser)]
#[command(name = "bleep-cli")]
#[command(about = "BLEEP Blockchain CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the BLEEP blockchain node
    StartNode,

    /// Wallet operations
    Wallet {
        #[command(subcommand)]
        action: WalletCommand,
    },

    /// Send and manage transactions
    Tx {
        #[command(subcommand)]
        action: TxCommand,
    },

    /// AI-powered utilities
    Ai {
        #[command(subcommand)]
        task: AiCommand,
    },

    /// Governance proposals and voting
    Governance {
        #[command(subcommand)]
        task: GovernanceCommand,
    },

    /// Verify ZKPs and proofs
    Zkp {
        proof: String,
    },

    /// State management utilities
    State {
        #[command(subcommand)]
        task: StateCommand,
    },

    /// Telemetry reporting
    Telemetry,

    /// Predictive Autonomous Technology (PAT) tasks
    Pat {
        #[command(subcommand)]
        task: PatCommand,
    },

    /// Display node information
    Info,

    /// Blockchain core operations
    Block {
        #[command(subcommand)]
        task: BlockCommand,
    },
}

#[derive(Subcommand)]
enum WalletCommand {
    Create,
    Balance,
    Import { phrase: String },
    Export,
}

#[derive(Subcommand)]
enum TxCommand {
    Send { to: String, amount: f64 },
    History,
}

#[derive(Subcommand)]
enum AiCommand {
    Ask { prompt: String },
    Learn { dataset: String },
}

#[derive(Subcommand)]
enum GovernanceCommand {
    Propose { proposal: String },
    Vote { proposal_id: u32, yes: bool },
}

#[derive(Subcommand)]
enum StateCommand {
    Snapshot,
    Restore { snapshot_path: String },
}

#[derive(Subcommand)]
enum PatCommand {
    Predict { input: String },
    Autonomy,
}

#[derive(Subcommand)]
enum BlockCommand {
    /// Get latest block
    Latest,

    /// Get block by hash or height
    Get { identifier: String },

    /// Validate block
    Validate { hash: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::StartNode => {
            info!("Starting node...");
            consensus::start_node().await?;
        }
        Commands::Wallet { action } => match action {
            WalletCommand::Create => wallet_core::create_wallet()?,
            WalletCommand::Balance => wallet_core::get_balance()?,
            WalletCommand::Import { phrase } => wallet_core::import_wallet(&phrase)?,
            WalletCommand::Export => wallet_core::export_wallet()?,
        },
        Commands::Tx { action } => match action {
            TxCommand::Send { to, amount } => wallet_core::send_transaction(&to, amount)?,
            TxCommand::History => wallet_core::transaction_history()?,
        },
        Commands::Ai { task } => match task {
            AiCommand::Ask { prompt } => ai_assistant::run_prompt(&prompt).await?,
            AiCommand::Learn { dataset } => machine_learning::train_on_dataset(&dataset).await?,
        },
        Commands::Governance { task } => match task {
            GovernanceCommand::Propose { proposal } => governance_engine::submit_proposal(&proposal)?,
            GovernanceCommand::Vote { proposal_id, yes } => governance_engine::vote(proposal_id, yes)?,
        },
        Commands::Zkp { proof } => {
            if zkp_verification::verify_proof(&proof)? {
                println!("ZKP valid");
            } else {
                println!("ZKP invalid");
            }
        }
        Commands::State { task } => match task {
            StateCommand::Snapshot => state_manager::create_snapshot()?,
            StateCommand::Restore { snapshot_path } => state_manager::restore_snapshot(&snapshot_path)?,
        },
        Commands::Telemetry => {
            telemetry::report_metrics()?;
        }
        Commands::Pat { task } => match task {
            PatCommand::Predict { input } => pat_engine::predict(&input)?,
            PatCommand::Autonomy => pat_engine::enable_autonomy()?,
        },
        Commands::Info => {
            let info = p2p_network::get_node_info()?;
            println!("{}", info);
        }
        Commands::Block { task } => match task {
            BlockCommand::Latest => {
                let block = blockchain::latest_block()?;
                println!("{:?}", block);
            }
            BlockCommand::Get { identifier } => {
                let block = blockchain::get_block(&identifier)?;
                println!("{:?}", block);
            }
            BlockCommand::Validate { hash } => {
                let is_valid = blockchain::validate_block(&hash)?;
                println!("Block valid: {}", is_valid);
            }
        }
    }

    Ok(())
}

