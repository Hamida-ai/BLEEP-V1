use std::sync::{Arc, Mutex};
use log::{info, warn};
use crate::{
    Wallet, WalletError, 
    p2p::P2PNode,
    state_merkle::StateMerkle,
    Transaction,
};

pub fn init() {
    // Initialize logger (will silently fail if already initialized)
    let _ = env_logger::builder().is_test(true).try_init();

    println!("🔧 Initializing bleep_wallet_core...");
    info!("🔐 Bootstrapping BLEEP components...");

    // 🔌 Initialize core dependencies
    let p2p_node = Arc::new(P2PNode::new());
    let state_merkle = Arc::new(Mutex::new(StateMerkle::new()));

    // 🧠 Create a new Wallet instance
    match Wallet::new(p2p_node.clone(), state_merkle.clone()) {
        Ok(mut wallet) => {
            info!("✅ Wallet initialized for address: {}", wallet.address);
            println!("🔑 Wallet created with address: {}", wallet.address);

            // 📦 Prepare a dummy transaction
            let tx = Transaction {
                id: "init_tx_001".to_string(),
                from: wallet.address.clone(),
                to: "recipient_xyz".to_string(),
                amount: 10.5,
                fee: match wallet.optimize_gas_fee("BLEEP-NET") {
                    Ok(fee) => fee,
                    Err(_) => 0.01,
                },
                signature: vec![],
            };

            // ✅ Sign the transaction
            match wallet.sign_transaction(&tx) {
                Ok(signature) => {
                    let mut signed_tx = tx.clone();
                    signed_tx.signature = signature;

                    // 🧱 Store it in the Merkle state
                    wallet.store_transaction(signed_tx);
                    println!("📝 Transaction signed and stored in Merkle state.");
                },
                Err(e) => {
                    warn!("❌ Failed to sign transaction: {}", e);
                    println!("❌ Signing error: {}", e);
                }
            }

            println!("✅ bleep_wallet_core initialization complete.");
        }
        Err(e) => {
            warn!("❌ Wallet initialization failed: {}", e);
            println!("🚨 Wallet initialization error: {}", e);
        }
    }
}
