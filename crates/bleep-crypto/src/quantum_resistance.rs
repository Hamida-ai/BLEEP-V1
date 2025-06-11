use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use kyber::Kyber;
use falcon::{SecretKey, PublicKey, Signature};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use std::time::{SystemTime, UNIX_EPOCH};
use log::{info, warn, error};
use ring::digest::{Context, SHA3_256};
use rand::Rng;
use hex;
use bincode;
use tokio::sync::RwLock;

// Initialize logging
fn init_logger() {
    env_logger::init();
}

// 🔹 Quantum-Resistant Transaction Structure
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Transaction {
    pub id: u64,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: u64,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl Transaction {
    pub fn new(id: u64, from: &str, to: &str, amount: u64, sk: &SecretKey, pk: &PublicKey) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time issue")
            .as_secs();
        
        let mut transaction = Transaction {
            id,
            from: from.to_string(),
            to: to.to_string(),
            amount,
            timestamp,
            signature: vec![],
            public_key: pk.to_bytes().to_vec(),
        };

        transaction.signature = transaction.sign(sk);
        transaction
    }

    // Compute SHA3-256 hash of the transaction
    fn hash(&self) -> Vec<u8> {
        let mut context = Context::new(&SHA3_256);
        context.update(&self.id.to_be_bytes());
        context.update(self.from.as_bytes());
        context.update(self.to.as_bytes());
        context.update(&self.amount.to_be_bytes());
        context.update(&self.timestamp.to_be_bytes());
        context.finish().as_ref().to_vec()
    }

    // Sign transaction using Falcon
    pub fn sign(&self, sk: &SecretKey) -> Vec<u8> {
        falcon::sign(&sk, &self.hash()).expect("Transaction signing failed")
    }

    // Verify signature using Falcon
    pub fn verify(&self) -> bool {
        let pk = PublicKey::from_bytes(&self.public_key).expect("Invalid public key");
        falcon::verify(&pk, &self.hash(), &self.signature).is_ok()
    }
}

// 🔹 Blockchain Block Structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub previous_hash: String,
    pub transactions: Vec<Transaction>,
    pub timestamp: u64,
    pub hash: String,
}

impl Block {
    pub fn new(id: u64, previous_hash: String, transactions: Vec<Transaction>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time error")
            .as_secs();
        let hash = Self::calculate_hash(&transactions, &previous_hash, timestamp);

        Block {
            id,
            previous_hash,
            transactions,
            timestamp,
            hash,
        }
    }

    pub fn calculate_hash(transactions: &[Transaction], previous_hash: &str, timestamp: u64) -> String {
        let mut hasher = Context::new(&SHA3_256);
        hasher.update(previous_hash.as_bytes());
        hasher.update(&timestamp.to_be_bytes());
        for tx in transactions {
            hasher.update(&bincode::serialize(tx).unwrap());
        }
        hex::encode(hasher.finish())
    }
}

// 🔹 Quantum-Secure Encryption & Decryption System
pub struct QuantumSecure {
    pub public_key: Arc<Mutex<Kyber>>,
    pub private_key: Arc<Mutex<Kyber>>,
}

impl QuantumSecure {
    pub fn new() -> Self {
        let (pk, sk) = Kyber::key_gen().expect("Kyber Key generation failed");
        QuantumSecure {
            public_key: Arc::new(Mutex::new(pk)),
            private_key: Arc::new(Mutex::new(sk)),
        }
    }

    pub fn encrypt_transaction(&self, transaction: &Transaction) -> Vec<u8> {
        let serialized = bincode::serialize(transaction).expect("Serialization failed");
        let pk = self.public_key.lock().unwrap();
        
        let (ciphertext, shared_secret) = pk.encapsulate().expect("Kyber Encryption failed");

        let key = Key::from_slice(&shared_secret);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&rand::thread_rng().gen::<[u8; 12]>());
        let encrypted_data = cipher.encrypt(nonce, serialized.as_ref()).expect("AES encryption failed");

        [nonce.as_ref(), &ciphertext, &encrypted_data].concat()
    }

    pub fn decrypt_transaction(&self, encrypted_data: &[u8]) -> Transaction {
        let nonce = &encrypted_data[0..12];
        let ciphertext = &encrypted_data[12..Kyber::cipher_text_bytes() + 12];
        let encrypted_tx = &encrypted_data[Kyber::cipher_text_bytes() + 12..];

        let sk = self.private_key.lock().unwrap();
        let shared_secret = sk.decapsulate(ciphertext).expect("Decryption failed");

        let key = Key::from_slice(&shared_secret);
        let cipher = Aes256Gcm::new(key);
        let decrypted_data = cipher.decrypt(Nonce::from_slice(nonce), encrypted_tx).expect("AES decryption failed");
        bincode::deserialize(&decrypted_data).expect("Deserialization failed")
    }
}

// 🔹 Blockchain State Management
pub struct BlockchainState {
    pub chain: Arc<RwLock<Vec<Block>>>,
    pub mempool: Arc<RwLock<HashSet<Transaction>>>,
}

impl BlockchainState {
    pub fn new() -> Self {
        BlockchainState {
            chain: Arc::new(RwLock::new(vec![Block::new(0, String::new(), vec![])])),
            mempool: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn add_transaction(&self, transaction: Transaction) {
        if transaction.verify() {
            let mut mempool = self.mempool.write().await;
            mempool.insert(transaction);
            info!("Transaction added to mempool.");
        } else {
            error!("Invalid transaction: signature verification failed!");
        }
    }

    pub async fn add_block(&self, block: Block) {
        let mut chain = self.chain.write().await;
        chain.push(block);
        info!("New block added to blockchain.");
    }
}

// 🔹 Adaptive Consensus System
pub struct AdaptiveConsensus {
    pub validators: HashMap<String, f64>,
    pub network_reliability: f64,
    pub consensus_mode: String,
}

impl AdaptiveConsensus {
    pub fn new() -> Self {
        AdaptiveConsensus {
            validators: HashMap::new(),
            network_reliability: 0.9,
            consensus_mode: "PoS".to_string(),
        }
    }

    pub fn switch_mode(&mut self, network_load: u64) {
        self.consensus_mode = if network_load > 80 {
            "PoW".to_string()
        } else if network_load > 40 {
            "PBFT".to_string()
        } else {
            "PoS".to_string()
        };
        info!("Consensus mode switched to {}", self.consensus_mode);
    }
}

// 🔹 Main Blockchain Initialization
#[tokio::main]
async fn main() {
    init_logger();
    let blockchain = BlockchainState::new();
    let consensus = AdaptiveConsensus::new();
    info!("Blockchain initialized with genesis block.");
}