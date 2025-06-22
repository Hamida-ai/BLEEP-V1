// src/bin/transaction.rs

use bleep_core::transaction::{Transaction, TransactionBuilder};
use bleep_core::transaction_pool::TransactionPool;
use bleep_crypto::quantum_resistance::sign_transaction;
use bleep_core::proof_of_identity::Identity;

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

    // Step 2: Build new transaction
    let sender = Identity::current_node_identity_hash();
    let mut tx = TransactionBuilder::new(sender.clone(), recipient.clone(), amount).build();

    // Step 3: Sign transaction using Falcon or Kyber private key
    sign_transaction(&mut tx)?;
    info!("📝 Transaction signed: {} -> {} for {}", sender, recipient, amount);

    // Step 4: Submit transaction to the mempool
    let mut pool = TransactionPool::load()?;
    pool.add_transaction(tx.clone())?;
    pool.persist()?;
    info!("📤 Transaction submitted to mempool: {}", tx.tx_hash);

    Ok(())
}
