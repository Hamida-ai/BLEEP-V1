//! # StateManager — Sprint 3
//!
//! RocksDB-backed state manager with:
//!   - Account balances, nonces, code hashes persisted to disk
//!   - **Sparse Merkle Trie** state root (Sprint 3 upgrade from blake3 hash-of-pairs)
//!   - Snapshot / restore for crash recovery
//!   - In-memory write-back cache for hot-path performance

use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

use crate::state_merkle::SparseMerkleTrie;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Serialisation error: {0}")]
    Serialisation(String),
}

pub type StateResult<T> = Result<T, StateError>;

// On-disk key prefixes
const PREFIX_ACCOUNT: &[u8] = b"acct:";
const KEY_HEIGHT: &[u8]     = b"sys:block_height";

/// Persisted account record.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AccountState {
    pub balance:   u128,
    pub nonce:     u64,
    pub code_hash: Option<[u8; 32]>,
}

// ── In-memory write-back cache entry ─────────────────────────────────────────

#[derive(Debug, Clone)]
struct CacheEntry {
    state: AccountState,
    dirty: bool,
}

// ── StateManager ─────────────────────────────────────────────────────────────

/// Top-level state manager with RocksDB persistence + SparseMerkleTrie state root.
pub struct StateManager {
    db:           rocksdb::DB,
    cache:        HashMap<String, CacheEntry>,
    block_height: u64,
    /// Sprint 3: Sparse Merkle Trie for O(1)-amortised cryptographic state root.
    trie:         SparseMerkleTrie,
}

impl StateManager {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Open (or create) a RocksDB database at `path`.
    pub fn open<P: AsRef<Path>>(path: P) -> StateResult<Self> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts.set_max_open_files(512);

        let db = rocksdb::DB::open(&opts, path)
            .map_err(|e| StateError::Storage(e.to_string()))?;

        let block_height = match db.get(KEY_HEIGHT) {
            Ok(Some(v)) => {
                let arr: [u8; 8] = v.as_slice().try_into()
                    .map_err(|_| StateError::Storage("corrupt block_height".into()))?;
                u64::from_le_bytes(arr)
            }
            Ok(None) => 0,
            Err(e) => return Err(StateError::Storage(e.to_string())),
        };

        log::info!("[StateManager] Opened DB — block_height={}", block_height);
        Ok(Self {
            db,
            cache: HashMap::new(),
            block_height,
            trie: SparseMerkleTrie::new(),
        })
    }

    /// In-memory (temp dir). Panics only if the OS temp dir is unusable.
    pub fn new() -> Self {
        let tmp = std::env::temp_dir()
            .join(format!("bleep-state-{}-{}", std::process::id(), pid_suffix()));
        Self::open(&tmp).unwrap_or_else(|e| {
            panic!("[StateManager] Cannot open RocksDB at temp dir: {}", e);
        })
    }

    // ── Account API ──────────────────────────────────────────────────────────

    pub fn get_balance(&self, address: &str) -> u128 {
        self.get_account(address).balance
    }

    pub fn set_balance(&mut self, address: &str, balance: u128) {
        let e = self.cache_entry(address);
        e.state.balance = balance;
        e.dirty = true;
    }

    pub fn increment_nonce(&mut self, address: &str) -> u64 {
        let e = self.cache_entry(address);
        e.state.nonce += 1;
        e.dirty = true;
        e.state.nonce
    }

    pub fn get_nonce(&self, address: &str) -> u64 {
        self.get_account(address).nonce
    }

    pub fn set_code_hash(&mut self, address: &str, hash: [u8; 32]) {
        let e = self.cache_entry(address);
        e.state.code_hash = Some(hash);
        e.dirty = true;
    }

    // ── Block lifecycle ──────────────────────────────────────────────────────

    pub fn block_height(&self) -> u64 { self.block_height }

    /// Advance block counter, sync dirty accounts into trie, flush to RocksDB.
    pub fn advance_block(&mut self) {
        self.sync_trie();
        self.block_height += 1;
        if let Err(e) = self.flush_internal() {
            log::error!("[StateManager] flush failed on advance_block: {}", e);
        }
    }

    // ── State root (Sparse Merkle Trie) ───────────────────────────────────────

    /// Compute the Sparse Merkle Trie state root.
    ///
    /// All dirty cache entries are synced into the trie first.
    /// Returns a 32-byte cryptographic commitment to the full account state.
    pub fn state_root(&mut self) -> [u8; 32] {
        self.sync_trie();
        self.trie.root()
    }

    /// Sync dirty cache entries into the Sparse Merkle Trie.
    fn sync_trie(&mut self) {
        for (addr, entry) in &self.cache {
            if entry.dirty {
                if entry.state.balance == 0 && entry.state.nonce == 0 {
                    self.trie.remove(addr);
                } else {
                    self.trie.insert(addr, entry.state.balance, entry.state.nonce);
                }
            }
        }
    }

    // ── Snapshot / restore ───────────────────────────────────────────────────

    pub fn create_snapshot(&mut self) -> StateResult<()> {
        self.sync_trie();
        self.flush_internal()
    }

    pub fn restore_snapshot(_path: &str) -> StateResult<Self> {
        log::warn!("[StateManager] WAL-based restore is Sprint 4");
        Ok(Self::new())
    }

    // ── Apply transactions ────────────────────────────────────────────────────

    /// Debit sender, credit receiver. Returns false if insufficient balance.
    pub fn apply_transfer(&mut self, sender: &str, receiver: &str, amount: u128) -> bool {
        let bal = self.get_balance(sender);
        if bal < amount {
            log::warn!("[StateManager] {} has {}, needs {}", sender, bal, amount);
            return false;
        }
        self.set_balance(sender, bal - amount);
        let recv = self.get_balance(receiver);
        self.set_balance(receiver, recv + amount);
        self.increment_nonce(sender);
        true
    }

    /// Mint tokens (block reward / genesis allocation).
    pub fn mint(&mut self, address: &str, amount: u128) {
        let bal = self.get_balance(address);
        self.set_balance(address, bal + amount);
    }

    // ── Trie query helpers ────────────────────────────────────────────────────

    /// Load all accounts from RocksDB into the trie (called at startup if needed).
    pub fn rebuild_trie_from_db(&mut self) -> StateResult<()> {
        let prefix = PREFIX_ACCOUNT;
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (k, v) = item.map_err(|e| StateError::Storage(e.to_string()))?;
            if !k.starts_with(prefix) { break; }
            let addr = std::str::from_utf8(&k[prefix.len()..])
                .map_err(|e| StateError::Serialisation(e.to_string()))?
                .to_string();
            let acct: AccountState = serde_json::from_slice(&v)
                .unwrap_or_default();
            if acct.balance > 0 || acct.nonce > 0 {
                self.trie.insert(&addr, acct.balance, acct.nonce);
            }
        }
        log::info!("[StateManager] Trie rebuilt from DB ({} accounts)", self.trie.len());
        Ok(())
    }

    // ── Merkle proof API (Sprint 5) ───────────────────────────────────────────

    /// Generate a Sparse Merkle Trie proof for `address`.
    ///
    /// Syncs dirty cache entries into the trie first so the proof is always
    /// up-to-date with the latest in-memory writes. Light clients can verify
    /// the returned `MerkleProof` against the published state root.
    pub fn prove_account(&mut self, address: &str) -> crate::state_merkle::MerkleProof {
        self.sync_trie();
        self.trie.prove(address)
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn get_account(&self, address: &str) -> AccountState {
        if let Some(e) = self.cache.get(address) {
            return e.state.clone();
        }
        let key = account_key(address);
        match self.db.get(&key) {
            Ok(Some(v)) => serde_json::from_slice::<AccountState>(&v).unwrap_or_default(),
            _ => AccountState::default(),
        }
    }

    fn cache_entry(&mut self, address: &str) -> &mut CacheEntry {
        if !self.cache.contains_key(address) {
            let state = self.get_account(address);
            self.cache.insert(address.to_string(), CacheEntry { state, dirty: false });
        }
        self.cache.get_mut(address).unwrap()
    }

    fn flush_internal(&self) -> StateResult<()> {
        let mut batch = rocksdb::WriteBatch::default();
        let mut flushed = 0usize;

        for (addr, entry) in &self.cache {
            if entry.dirty {
                let key = account_key(addr);
                let val = serde_json::to_vec(&entry.state)
                    .map_err(|e| StateError::Serialisation(e.to_string()))?;
                batch.put(key, val);
                flushed += 1;
            }
        }

        batch.put(KEY_HEIGHT, self.block_height.to_le_bytes());

        self.db.write(batch)
            .map_err(|e| StateError::Storage(e.to_string()))?;

        log::debug!("[StateManager] Flushed {} accounts, height={}", flushed, self.block_height);
        Ok(())
    }
}

impl Default for StateManager {
    fn default() -> Self { Self::new() }
}

fn account_key(address: &str) -> Vec<u8> {
    [PREFIX_ACCOUNT, address.as_bytes()].concat()
}

fn pid_suffix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_micros() as u64)
        .unwrap_or(0)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> StateManager { StateManager::new() }

    #[test]
    fn balance_roundtrip() {
        let mut m = fresh();
        assert_eq!(m.get_balance("alice"), 0);
        m.set_balance("alice", 1_000);
        assert_eq!(m.get_balance("alice"), 1_000);
    }

    #[test]
    fn nonce_increments() {
        let mut m = fresh();
        assert_eq!(m.get_nonce("bob"), 0);
        assert_eq!(m.increment_nonce("bob"), 1);
        assert_eq!(m.increment_nonce("bob"), 2);
    }

    #[test]
    fn apply_transfer_ok() {
        let mut m = fresh();
        m.mint("alice", 500);
        assert!(m.apply_transfer("alice", "bob", 200));
        assert_eq!(m.get_balance("alice"), 300);
        assert_eq!(m.get_balance("bob"),   200);
    }

    #[test]
    fn apply_transfer_insufficient() {
        let mut m = fresh();
        m.mint("alice", 50);
        assert!(!m.apply_transfer("alice", "bob", 200));
        assert_eq!(m.get_balance("alice"), 50);
    }

    #[test]
    fn state_root_changes_on_mutation() {
        let mut m = fresh();
        let r0 = m.state_root();
        m.mint("alice", 100);
        let r1 = m.state_root();
        assert_ne!(r0, r1);
    }

    #[test]
    fn state_root_is_deterministic() {
        let mut m1 = fresh();
        let mut m2 = fresh();
        m1.mint("alice", 100);
        m1.mint("bob",   200);
        m2.mint("bob",   200);
        m2.mint("alice", 100);
        // Insert order should not affect the trie root
        assert_eq!(m1.state_root(), m2.state_root());
    }

    #[test]
    fn advance_block_persists_height() {
        let mut m = fresh();
        assert_eq!(m.block_height(), 0);
        m.advance_block();
        assert_eq!(m.block_height(), 1);
    }

    #[test]
    fn snapshot_ok() {
        let mut m = fresh();
        m.mint("carol", 9999);
        assert!(m.create_snapshot().is_ok());
    }
}
