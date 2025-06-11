use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rand::Rng;
use aes_gcm::{Aes256Gcm, Key, Nonce}; // AES-GCM encryption
use aes_gcm::aead::{Aead, NewAead};
use tch::{CModule, Tensor}; // AI-based insights
use crate::{
    quantum_secure::QuantumSecure,
    zkp_verification::{BLEEPZKPModule, TransactionCircuit},
    interoperability::BLEEPInteroperabilityModule,
    governance::SelfAmendingGovernance,
};

// Representing the wallet
pub struct BLEEPWallet {
    pub bleeppats: HashMap<String, BLEEPpat>, // Mapping from token name to BLEEPpat
    pub balances: HashMap<String, u128>,     // Mapping from token name to balance
    private_key: String,                     // Secure key for signing transactions
    quantum_secure: Arc<QuantumSecure>,      // Quantum security integration
    zkp_module: Arc<BLEEPZKPModule>,         // ZKP integration
    interoperability: Arc<BLEEPInteroperabilityModule>, // Interoperability module
    governance: Arc<SelfAmendingGovernance>, // Governance module
    ai_module: Arc<CModule>,                 // AI module for wallet insights
}

impl BLEEPWallet {
    /// Initialize a new wallet with advanced features
    pub fn new(
        quantum_secure: Arc<QuantumSecure>,
        zkp_module: Arc<BLEEPZKPModule>,
        interoperability: Arc<BLEEPInteroperabilityModule>,
        governance: Arc<SelfAmendingGovernance>,
        ai_model_path: &str,
    ) -> Self {
        let private_key = generate_private_key();
        println!("New wallet created with private key: {}", private_key);

        // Load AI model for insights
        let ai_module = Arc::new(CModule::load(ai_model_path).expect("Failed to load AI model"));

        Self {
            bleeppats: HashMap::new(),
            balances: HashMap::new(),
            private_key,
            quantum_secure,
            zkp_module,
            interoperability,
            governance,
            ai_module,
        }
    }

    /// Display wallet balance
    pub fn get_balance(&self, token_name: &str) -> u128 {
        *self.balances.get(token_name).unwrap_or(&0)
    }

    /// Add a balance (for testing or initial distribution)
    pub fn add_balance(&mut self, token_name: &str, amount: u128) {
        let entry = self.balances.entry(token_name.to_string()).or_insert(0);
        *entry += amount;
    }

    /// AI-based insights for wallet automation
    pub fn get_insights(&self) -> String {
        let balances: Vec<f32> = self
            .balances
            .values()
            .map(|&balance| balance as f32)
            .collect();
        let tensor = Tensor::of_slice(&balances);

        // Run the AI model
        let predictions = self
            .ai_module
            .forward_ts(&[tensor])
            .expect("AI model failed to generate insights");

        format!("Predicted Trends: {:?}", predictions)
    }

    /// Create a new BLEEPpat with AES-GCM encryption
    pub fn create_bleeppat(&mut self, name: &str, metadata: &str, owner: &str) -> Result<(), String> {
        if self.bleeppats.contains_key(name) {
            return Err(format!("BLEEPpat with name '{}' already exists!", name));
        }

        // Encrypt metadata using AES-GCM
        let key = Key::from_slice(&self.private_key.as_bytes()[..32]); // Use private key as AES key
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(b"unique_nonce"); // Ensure a unique nonce per transaction
        let encrypted_metadata = cipher
            .encrypt(nonce, metadata.as_bytes())
            .map_err(|_| "Failed to encrypt metadata".to_string())?;

        let pat = BLEEPpat {
            name: name.to_string(),
            metadata: base64::encode(encrypted_metadata), // Store encrypted metadata
            owner: owner.to_string(),
        };

        self.bleeppats.insert(name.to_string(), pat);
        println!("BLEEPpat '{}' created successfully!", name);
        Ok(())
    }

    /// Transfer a token to another wallet with ZKP validation
    pub fn transfer(
        &mut self,
        token_name: &str,
        amount: u128,
        recipient_wallet: &Mutex<Self>,
        proof: Vec<u8>,
    ) -> Result<(), String> {
        // Check if the token exists in the wallet
        if !self.balances.contains_key(token_name) {
            return Err(format!("Token '{}' does not exist in the wallet.", token_name));
        }

        // Check for sufficient balance
        let balance = self.get_balance(token_name);
        if balance < amount {
            return Err("Insufficient balance.".to_string());
        }

        // Validate ZKP proof before transfer
        let circuit = TransactionCircuit {
            sender_balance: balance.into(),
            amount: amount.into(),
            receiver_balance: (balance - amount).into(),
        };
        let is_valid = self
            .zkp_module
            .verify_proof(&proof, &vec![balance as u8, amount as u8])
            .map_err(|_| "Invalid ZKP proof".to_string())?;
        ensure!(is_valid, "Proof verification failed!");

        // Deduct the amount
        let sender_balance = self.balances.get_mut(token_name).unwrap();
        *sender_balance -= amount;

        // Add the amount to the recipient
        let mut recipient = recipient_wallet.lock().unwrap();
        recipient.add_balance(token_name, amount);

        println!(
            "Transferred {} of '{}' to recipient wallet.",
            amount, token_name
        );

        Ok(())
    }

    /// Encrypt wallet data securely using AES-GCM
    pub fn encrypt_data(&self) -> Result<String, String> {
        let data = format!("{:?}", self);

        // Encrypt wallet data using AES-GCM
        let key = Key::from_slice(&self.private_key.as_bytes()[..32]);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(b"wallet_nonce");
        let encrypted_data = cipher
            .encrypt(nonce, data.as_bytes())
            .map_err(|_| "Encryption failed".to_string())?;

        Ok(base64::encode(encrypted_data))
    }

    /// Cross-chain transfer using BLEEPat
    pub fn cross_chain_transfer(
        &mut self,
        token_name: &str,
        amount: u128,
        chain_id: u32,
    ) -> Result<(), String> {
        // Check if the chain ID is trusted
        let trusted_chain_ids = self.interoperability.get_trusted_chains();
        ensure!(
            trusted_chain_ids.contains(&chain_id),
            "Invalid or untrusted chain ID!"
        );

        // Check for sufficient balance
        let balance = self.get_balance(token_name);
        ensure!(balance >= amount, "Insufficient balance!");

        // Deduct amount for cross-chain transfer
        let sender_balance = self.balances.get_mut(token_name).unwrap();
        *sender_balance -= amount;

        // Relay data via BLEEPConnect
        self.interoperability
            .relay_data("cross_chain_transfer", &amount.to_be_bytes(), chain_id)
            .map_err(|_| "Failed to relay data".to_string())?;

        println!(
            "Cross-chain transfer of {} {} to chain {} successful!",
            amount, token_name, chain_id
        );
        Ok(())
    }
}

// Struct representing a Programmable Asset Token (BLEEPpat)
#[derive(Debug, Clone)]
pub struct BLEEPpat {
    pub name: String,
    pub metadata: String,
    pub owner: String,
}

// Utility function to generate a random private key
fn generate_private_key() -> String {
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| rng.gen_range(0..=255))
        .map(|byte| format!("{:02x}", byte))
        .collect()
  } 