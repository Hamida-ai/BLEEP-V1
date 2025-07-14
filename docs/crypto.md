# BLEEP Cryptographic Architecture

## Overview

The BLEEP blockchain uses an advanced, hybrid cryptographic system for transaction integrity, quantum resistance, secure encryption, adaptive consensus, and zero-knowledge proof mechanisms.

---

## 🔐 Quantum-Resistant Transactions

### Structure: `Transaction`
- Uses **Falcon** (quantum-safe) for signing and verification.
- SHA3-256 is used for hashing.
- Each transaction includes:
  - `id`, `from`, `to`, `amount`, `timestamp`
  - Falcon `signature` and `public_key`.

### Key Methods:
- `new(...)` — Create and sign a transaction.
- `sign(...)` — Sign using Falcon SecretKey.
- `verify()` — Verify with Falcon PublicKey.

---

## ⛓️ Block Structure

### Structure: `Block`
- Contains:
  - `id`, `previous_hash`, `transactions`, `timestamp`, `hash`
- Hash is computed using:
  - SHA3-256 + transaction serialization.

### Key Method:
- `calculate_hash(...)` — Determines block hash from transactions and metadata.

---

## 🛡️ QuantumSecure (Kyber + AES-GCM)

### Structure: `QuantumSecure`
- Combines **Kyber** (post-quantum key encapsulation) with **AES-256-GCM**.
- Handles:
  - Encryption and decryption of serialized transactions.

### Key Methods:
- `encrypt_transaction(...)` → `Vec<u8>`
- `decrypt_transaction(...)` → `Transaction`

---

## 🧠 Blockchain State

### Structure: `BlockchainState`
- Manages:
  - `chain`: `Vec<Block>`
  - `mempool`: `HashSet<Transaction>`

### Async Methods:
- `add_transaction(...)`
- `add_block(...)`

---

## ⚖️ Adaptive Consensus

### Structure: `AdaptiveConsensus`
- Dynamically switches between:
  - `PoS`, `PBFT`, `PoW` based on network load.

### Key Method:
- `switch_mode(network_load: u64)`

---

## 📦 Zero-Knowledge Proof Module (`BLEEPZKPModule`)

### Uses:
- `Groth16` with `Bls12_381` for zk-SNARKs.
- PoseidonCRH and Bulletproofs for compression.
- Secure key revocation via Merkle Trees.
- Hybrid encryption via `KyberAESHybrid`.

### Key Capabilities:
- `generate_batch_proofs(...)`
- `aggregate_proofs(...)`
- `revoke_key(...)`, `is_key_revoked(...)`
- `save/load_proving_key`, `save/load_revocation_tree`

---

## 🧬 Asset Recovery (Anti-Asset Loss)

### Structure: `AssetRecoveryRequest`
- ZK-verified ownership claims.
- Broadcasts encrypted proof to the network.
- Uses expiration and approval thresholds.

### Lifecycle:
1. `submit()` – Submits request via governance.
2. `validate()` – ZK-SNARK proof verification.
3. `finalize()` – Approves or rejects based on quorum.

---

## 📡 Logging, Auditing, and Fraud Detection

- `BLEEPLogger`: records activity.
- `AnomalyDetector`: used in off-chain voting (in governance module).
- IPFS/Arweave storage for immutable audit trails.

---

## Dependencies Summary

| Library/Tool        | Purpose                             |
|---------------------|-------------------------------------|
| Falcon              | Quantum-resistant digital signatures |
| Kyber               | Post-quantum key encapsulation      |
| AES-GCM             | Symmetric authenticated encryption  |
| SHA3-256            | Cryptographic hashing               |
| Groth16 (arkworks)  | zk-SNARK proof generation           |
| Bulletproofs        | Compact zero-knowledge proofs       |
| MerkleTree          | Key revocation and inclusion proofs |
| bincode             | Serialization                       |
| hex                 | Hash encoding                       |

---

## Initialization

```rust
fn main() {
    init_logger();
    let blockchain = BlockchainState::new();
    let consensus = AdaptiveConsensus::new();
    info!("Blockchain initialized with genesis block.");
}
 
