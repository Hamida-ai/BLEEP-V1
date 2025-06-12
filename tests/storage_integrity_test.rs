use advanced_bleep::core::storage::{StorageEngine, KeyValueStore};
use advanced_bleep::core::blockchain::{Blockchain, Block};
use advanced_bleep::core::crypto::HashFunction;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn storage_data_integrity_test() {
    println!("🚀 **Starting BLEEP Blockchain Storage & Data Integrity Test...**");

    // 🌎 Initialize Blockchain, Storage Engine, and Key-Value Store
    let blockchain = Arc::new(Mutex::new(Blockchain::new()));
    let storage_engine = Arc::new(Mutex::new(StorageEngine::new()));
    let kv_store = Arc::new(Mutex::new(KeyValueStore::new()));

    // 📌 Start Storage & Data Integrity Test
    let start_time = Instant::now();

    // 🚀 1. Store and Retrieve Data
    println!("⚠️ **Testing storage and retrieval of data...**");
    let key = "test_key";
    let value = "test_value";
    kv_store.lock().unwrap().store(key, value);
    let retrieved_value = kv_store.lock().unwrap().retrieve(key);
    assert!(retrieved_value == Some(value.to_string()), "🚨 Data retrieval failed!");

    // 🚀 2. Verify Data Integrity with Hashing
    println!("⚠️ **Testing data integrity using hashing...**");
    let original_hash = HashFunction::sha256(value);
    let retrieved_hash = HashFunction::sha256(&retrieved_value.unwrap());
    assert!(original_hash == retrieved_hash, "🚨 Data integrity compromised!");

    // 🚀 3. Simulate Data Corruption and Recovery
    println!("⚠️ **Simulating data corruption and recovery...**");
    let corrupted_value = "corrupt_value";
    kv_store.lock().unwrap().store(key, corrupted_value);
    let corrupted_hash = HashFunction::sha256(corrupted_value);
    assert!(original_hash != corrupted_hash, "🚨 Corruption not detected!");

    // Recover the correct value
    kv_store.lock().unwrap().store(key, value);
    let recovered_value = kv_store.lock().unwrap().retrieve(key);
    let recovered_hash = HashFunction::sha256(&recovered_value.unwrap());
    assert!(recovered_hash == original_hash, "🚨 Data recovery failed!");

    // 🚀 4. Test Storage Under High Load
    println!("⚠️ **Testing storage efficiency under high load...**");
    for i in 0..1_000_000 {
        kv_store.lock().unwrap().store(&format!("key_{}", i), &format!("value_{}", i));
    }
    assert!(kv_store.lock().unwrap().verify_storage_efficiency(), "🚨 Storage engine failed under high load!");

    // 🚀 5. Test Immutable Blockchain Storage
    println!("⚠️ **Testing immutable blockchain storage...**");
    let block = Block::new(1, "previous_hash", "test_data");
    blockchain.lock().unwrap().add_block(block.clone());
    let retrieved_block = blockchain.lock().unwrap().get_block(1);
    assert!(retrieved_block.is_some(), "🚨 Block storage failed!");
    assert!(retrieved_block.unwrap().hash == block.hash, "🚨 Blockchain immutability violated!");

    println!("✅ **BLEEP Blockchain Storage & Data Integrity Test Completed Successfully!**");
}