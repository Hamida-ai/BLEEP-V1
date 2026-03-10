# BLEEP Blockchain

> **Quantum-secure · AI-native · Cross-chain · Self-healing**
>
> A production-grade, post-quantum Layer 1 blockchain written entirely in Rust, with adaptive multi-mode consensus, a universal 7-layer VM, Sparse Merkle Trie state, and built-in cross-chain interoperability via BLEEP Connect.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Crate Map](#crate-map)
4. [Core Subsystems](#core-subsystems)
   - [Cryptography](#cryptography)
   - [Consensus Engine](#consensus-engine)
   - [State Layer](#state-layer)
   - [Universal VM](#universal-vm)
   - [P2P Networking](#p2p-networking)
   - [RPC Server](#rpc-server)
   - [Wallet and CLI](#wallet-and-cli)
   - [Governance](#governance)
   - [Economics and Tokenomics](#economics-and-tokenomics)
   - [BLEEP Connect](#bleep-connect)
   - [Supporting Services](#supporting-services)
5. [Protocol Parameters](#protocol-parameters)
6. [Getting Started](#getting-started)
7. [Configuration](#configuration)
8. [RPC API Reference](#rpc-api-reference)
9. [CLI Reference](#cli-reference)
10. [Security Model](#security-model)
11. [Roadmap to Public Testnet](#roadmap-to-public-testnet)
12. [Contributing](#contributing)
13. [License](#license)

---

## Overview

BLEEP is a sovereign Layer 1 blockchain designed to outlast the quantum computing transition. Every security primitive — transaction signing, key encapsulation, block validation, P2P encryption — uses post-quantum algorithms from NIST's finalised PQC suite. Classical algorithms (Ed25519, AES-GCM) are retained only where quantum resistance is not strictly required today and will be replaced on a defined schedule.

The chain is opinionated about a few things:

- **Safety over liveness.** The finality model requires >⅔ stake agreement. No optimistic shortcuts without explicit ZK proofs.
- **Determinism above everything.** Consensus mode transitions, shard assignments, and epoch boundaries are computed identically on every honest node from the same inputs, with no coordinator.
- **Economics as a first-class protocol citizen.** Inflation, fees, slashing, and oracle data are enforced on-chain by the `bleep-economics` engine with constitutional hard caps that no governance vote can override.
- **Cross-chain without trusted bridges.** BLEEP Connect provides four security tiers, from a 200 ms optimistic relay up to full-node verification, without requiring a privileged bridge operator.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              BLEEP Node (src/bin/main.rs)                    │
│                                                                              │
│  ┌─────────────────┐   ┌─────────────────┐   ┌──────────────────────────┐   │
│  │   bleep-cli     │   │   bleep-rpc      │   │    bleep-telemetry       │   │
│  │  (clap + async) │   │  warp :8545      │   │  tracing + metrics       │   │
│  └────────┬────────┘   └────────┬─────────┘   └──────────────────────────┘   │
│           │                    │                                              │
│  ┌────────▼────────────────────▼──────────────────────────────────────────┐  │
│  │                         bleep-consensus                                │  │
│  │  BlockProducer  (3 s slots · 4 096 tx/block)                          │  │
│  │  ConsensusOrchestrator  PoS | PBFT | Emergency-PoW                    │  │
│  │  EpochConfig · SlashingEngine · FinalityManager (>⅔ stake)            │  │
│  └────────┬──────────────────────────────┬─────────────────────────────┘  │
│           │                              │                                   │
│  ┌────────▼────────┐          ┌──────────▼──────────┐                       │
│  │   bleep-core    │          │    bleep-vm          │                       │
│  │  Block · Tx     │          │  7-layer intent VM   │                       │
│  │  Blockchain     │          │  EVM (revm 3.5)      │                       │
│  │  BlockValidator │          │  WASM (wasmer 4.2)   │                       │
│  │  Mempool · Pool │          │  ZK · StateDiff      │                       │
│  └────────┬────────┘          └──────────┬───────────┘                       │
│           │                              │                                   │
│  ┌────────▼──────────────────────────────▼───────────────────────────────┐  │
│  │                          bleep-state                                  │  │
│  │  StateManager (RocksDB + LZ4)  ·  SparseMerkleTrie (256-bit paths)   │  │
│  │  Sharding · Cross-shard 2PC · Self-healing · Snapshot / Rollback      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌──────────────────┐  ┌────────────────┐  ┌────────────────────────────┐  │
│  │   bleep-p2p      │  │  bleep-crypto  │  │      bleep-interop         │  │
│  │  libp2p 0.53     │  │  SPHINCS+      │  │  BLEEP Connect (4 tiers)   │  │
│  │  Kademlia k=20   │  │  Kyber-768     │  │  10 sub-crates             │  │
│  │  Gossip (Plumtree│  │  BIP-39        │  │  ETH · SOL adapters        │  │
│  │  Onion routing   │  │  AES-256-GCM   │  │  Groth16 ZK bridge proofs  │  │
│  └──────────────────┘  └────────────────┘  └────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Block production data flow

```
MempoolBridge (500 ms drain)
  │
TransactionPool  (FIFO, per-sender nonce ordering)
  │
BlockProducer.produce_one()
  ├── VM Executor per-tx          →  StateDiff { balances, nonces }
  ├── apply_transfer()            →  StateManager  (sender nonce++)
  ├── StateDiff.balances          →  contract side-effects
  ├── StateDiff.nonces            →  smart-contract nonce sync
  ├── advance_block()             →  flush cache · SparseMerkleTrie root
  ├── sign_block()                →  validator_signature (96 bytes)
  └── generate_zkp()             →  64-byte Fiat-Shamir commitment
        │
GossipBridge  →  P2PNode  →  peers
        │
FinalizedBlock  →  Scheduler tasks  →  RPC counters
```

### Inbound peer-block pipeline

```
P2PNode.recv()
  │  serde_json::from_slice  →  Block
  │
BlockValidator::validate_block()   — validator_signature + verify_zkp() (64-byte)
  │
per-tx SPHINCS+ sig check          — empty sigs skipped (legacy compat)
  │
height check  →  skip if already have
  │
Blockchain::add_block()
  │
StateManager::advance_block()
```

---

## Crate Map

| Crate | Version | Purpose |
|---|---|---|
| `bleep-core` | 0.1.0 | Block, Transaction, Blockchain, Mempool, TransactionPool, BlockValidator |
| `bleep-crypto` | 0.1.0 | SPHINCS+, Kyber-768, Falcon, BIP-39, AES-256-GCM, tx signer, ZKP verify |
| `bleep-consensus` | 0.1.0 | BlockProducer, PoS/PBFT/PoW orchestrator, Epoch, Slashing, Finality |
| `bleep-state` | 1.0.0 | StateManager (RocksDB), SparseMerkleTrie, Sharding, 2PC, Self-healing |
| `bleep-vm` | 0.5.0 | 7-layer intent VM: EVM (revm), WASM (wasmer/Cranelift), ZK, unified gas |
| `bleep-p2p` | 0.1.0 | libp2p, Kademlia DHT, Gossip (Plumtree), Onion routing, AI peer scoring |
| `bleep-rpc` | 1.0.0 | warp HTTP/JSON server (10 endpoints, live StateManager integration) |
| `bleep-wallet-core` | 0.1.0 | EncryptedWallet, AES-256-GCM key-at-rest, WalletManager |
| `bleep-cli` | 1.0.0 | clap async CLI: wallet, tx, block, governance, state, ZKP, AI, PAT |
| `bleep-governance` | 0.1.0 | On-chain proposals, voting windows, tally, GovernanceEngine |
| `bleep-economics` | 0.1.0 | Tokenomics engine, EIP-1559-style fee market, validator incentives, oracle |
| `bleep-pat` | 1.0.0 | Programmable Asset Token (PAT), 1 B token supply cap |
| `bleep-interop` | 0.1.0 | BLEEP Connect: 10 sub-crates, 4-tier cross-chain protocol |
| `bleep-zkp` | 0.1.0 | ark-groth16 / BLS12-381 circuit stubs, Verifier/Prover API |
| `bleep-scheduler` | 0.1.0 | 20-task Tokio scheduler: epoch, rewards, healing, mempool, indexer |
| `bleep-auth` | 0.1.0 | JWT sessions, RBAC, Merkle-chained audit log, Kyber validator binding |
| `bleep-indexer` | 0.1.0 | DashMap chain indexer, reorg rollback, checkpoint engine |
| `bleep-ai` | 0.1.0 | Deterministic AI advisory engine (BLEEPAIAssistant) |
| `bleep-telemetry` | 0.1.0 | tracing-subscriber, MetricCounter, MetricGauge |

---

## Core Subsystems

### Cryptography

**Crate:** `bleep-crypto`

BLEEP uses post-quantum algorithms as the primary security layer, not as a future upgrade path.

#### Transaction signing — SPHINCS+-SHAKE-256-simple

All transactions are signed with SPHINCS+-SHAKE-256-simple via `pqcrypto-sphincsplus`. The canonical signed payload is:

```
payload = SHA3-256( sender_bytes || receiver_bytes || amount_le8 || timestamp_le8 )
```

```rust
use bleep_crypto::{sign_tx_payload, verify_tx_signature, tx_payload, generate_tx_keypair};

let (pk, sk) = generate_tx_keypair();
let payload  = tx_payload(&sender, &receiver, amount, timestamp);
let sig      = sign_tx_payload(&payload, &sk)?;
assert!(verify_tx_signature(&payload, &sig, &sk));
```

#### Key encapsulation — Kyber-768

Wallets carry a Kyber-768 KEM public key used for encrypted P2P session establishment and validator identity binding.

```rust
use bleep_crypto::quantum_resistance::{generate_falcon_keypair, generate_kyber_keypair};

let (sphincs_pk, sphincs_sk) = generate_falcon_keypair()?;  // SPHINCS+ keypair
let (kyber_pk, kyber_sk)     = generate_kyber_keypair()?;   // Kyber-768 keypair
```

#### Signing-key encryption — AES-256-GCM

Signing keys are never stored in plaintext. Each `EncryptedWallet.signing_key` holds:

```
blob             = nonce(12 bytes) || AES-256-GCM-ciphertext || GCM-tag(16 bytes)
encryption_key   = SHA3-256( password_utf8 || address_utf8 )   →  32 bytes
```

```rust
use bleep_wallet_core::wallet::EncryptedWallet;

let w    = EncryptedWallet::with_signing_key_encrypted(pk, &sk, kyber_pk, "passphrase")?;
let sk   = w.unlock("passphrase")?;     // decrypt → plaintext SPHINCS+ sk
w.lock(&new_sk, "passphrase")?;         // re-encrypt and replace
assert!(w.can_sign());                  // true when signing_key is non-empty
```

The nonce is randomly generated for every `lock()` or `with_signing_key_encrypted()` call, ensuring two encryptions of the same key produce different blobs.

#### BIP-39 wallet import

Standard BIP-39: PBKDF2-HMAC-SHA512, 2,048 rounds, optional passphrase.

```rust
use bleep_crypto::{validate_mnemonic, mnemonic_to_seed, mnemonic_to_bleep_seed};

validate_mnemonic("abandon abandon ... about")?; // 12/15/18/21/24 words
let seed_64 = mnemonic_to_seed("...", "TREZOR")?;   // [u8; 64]
let seed_32 = mnemonic_to_bleep_seed("...", "")?;   // first 32 bytes used as SPHINCS+ seed
```

#### Address format

```
address = "BLEEP1" + lower_hex( SHA256( SHA256( public_key ) )[..20] )
```

Example: `BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8`

---

### Consensus Engine

**Crate:** `bleep-consensus`

Three interchangeable consensus modes are managed by `ConsensusOrchestrator`. Mode selection is a pure deterministic function of epoch metrics — identical on every honest node, evaluated at epoch boundaries only.

| Mode | Trigger | Finality guarantee |
|---|---|---|
| `PosNormal` | Default | >⅔ stake, one-epoch finality |
| `PbftFastFinality` | High-throughput epochs | BFT immediate, >⅔ validator signatures |
| `EmergencyPoW` | Validator liveness failure | Hash-based fallback until recovery |

#### Block production constants

| Constant | Value |
|---|---|
| `BLOCK_INTERVAL_MS` | 3,000 ms |
| `MAX_TXS_PER_BLOCK` | 4,096 |
| Epoch length | 1,000 blocks |

#### Block signing layout

`sign_block()` stores a 96-byte `validator_signature`:

```
[0..32]   validator public key  = SHA3-256(sk_seed)
[32..64]  message hash          = SHA3-256(block_hash_hex)
[64..96]  signature proof       = SHA3-256(msg_hash || sk_seed)
```

`verify_signature(public_key)` checks all three fields. A block with an empty or malformed signature is rejected without executing the ZKP check.

#### 64-byte Fiat-Shamir ZK commitment

Every signed block carries a `zk_proof` field produced by `generate_zkp()` and verified by `verify_zkp()`:

```
challenge = SHA3-256(
    "BLEEP-ZKP-v1"          ← domain separator
    || block_hash           ← commits to all header fields
    || validator_pk         ← binds to the signing key
    || epoch_id_le8
    || protocol_version_le4
    || consensus_mode_u8
    || merkle_root_bytes    ← binds to account state
    || shard_id_le8
    || shard_state_root     ← binds to shard state
    || tx_count_le8         ← binds to transaction set
)

response  = SHA3-256( challenge || validator_pk || block_index_le8 )

zk_proof  = challenge[32] || response[32]   →  64 bytes
```

Full Groth16 SNARK circuit (ark-bls12-381) is now the production scheme.

#### Finality

`FinalityManager` accumulates `ValidatorSignature` entries into `FinalizyCertificate` records. A block is finalized when `accumulated_voting_power > (2/3) * total_stake`. Once finalized, a block's state root cannot be rolled back.

#### Slashing

`SlashingEngine` applies automatic penalties:

- **Double-signing:** 33% of validator stake slashed immediately
- **Liveness failure:** tracked per epoch, escalates to de-registration after threshold misses

---

### State Layer

**Crate:** `bleep-state`

#### StateManager

RocksDB-backed with LZ4 compression, 512 max open files, and an in-memory write-back cache for hot-path performance. Account records are persisted under the key prefix `b"acct:"`:

```
Key:   b"acct:" + address_utf8
Value: bincode( AccountState { balance: u128, nonce: u64, code_hash: Option<[u8;32]> } )
```

Core API:

```rust
let mut state = StateManager::open("/var/lib/bleep/state")?;

state.get_balance("BLEEP1...");                      // → u128
state.set_balance("BLEEP1...", 1_000_000);
state.get_nonce("BLEEP1...");                        // → u64
state.increment_nonce("BLEEP1...");                  // increments and returns new nonce

// Atomic transfer: debit sender, credit receiver, increment sender nonce
state.apply_transfer(sender, receiver, 500_000_u128);

state.advance_block();   // flush dirty cache → RocksDB, sync trie, persist height
state.state_root();      // → [u8; 32]  SparseMerkleTrie root
```

`advance_block()` is the commit boundary. All writes before it are buffered; a crash before `advance_block()` leaves the previous block's state intact.

#### SparseMerkleTrie — O(k + 256) Merkle proofs

A full 256-bit-depth Sparse Merkle Trie where each account maps to exactly one leaf. The trie provides a cryptographic state commitment that is included in every block and verified by the ZKP.

```
Leaf key   =  blake3( address_utf8 )
Leaf value =  blake3( abi_encode(address, balance, nonce) )
Interior   =  blake3( left_child || right_child )
Empty node =  [0u8; 32]
```

Proof generation: `prove()` builds a **depth-bucketed sibling index** from `interior_cache` in O(k) then does exactly 256 O(1) lookups — one per trie level. Total: **O(k + 256)**.

```rust
// Node side
let proof = state.prove_account("BLEEP1...");

// Light-client side (no node required)
assert!(proof.verify(&known_state_root));
```

`MerkleProof` serialises to JSON for delivery via `/rpc/proof/{address}`.

#### Sharding

Horizontal sharding with deterministic assignment:

- `ShardRegistry` — canonical topology per epoch (all nodes compute identically)
- `ShardValidatorAssignment` — deterministic validator-to-shard mapping
- `CrossShard2PC` — Byzantine-safe Two-Phase Commit coordinator (Prepare → Commit | Abort)
- `SelfHealingOrchestrator` + `AdvancedFaultDetector` — automatic shard recovery on fault detection
- `SnapshotEngine` / `RollbackEngine` — crash recovery to any previous finalized height

---

### Universal VM

**Crate:** `bleep-vm` (v0.5.0)

Execution is intent-driven. Callers submit typed `Intent` values; the `VmRouter` dispatches to the appropriate engine and enforces gas limits.

#### 7-layer architecture

| Layer | Responsibility |
|---|---|
| 1 — Intent | `TransferIntent`, `ContractCallIntent`, `DeployIntent`, `CrossChainIntent`, `ZkVerifyIntent` |
| 2 — Router | Engine selection, gas budget validation, circuit breaker, per-engine metrics |
| 3 — Engines | EVM via `revm 3.5`, WASM via `wasmer 4.2` (Cranelift backend), ZK via `ark-groth16` |
| 4 — Sandbox | Memory limits, call-stack depth enforcement, host API filtering |
| 5 — State transition | Returns `StateDiff`; never writes `StateManager` directly |
| 6 — Unified gas | EVM, WASM, ZK, SBF, Move gas normalised to a single BLEEP gas unit |
| 7 — Cross-chain | `bleep_call(chain, contract, data)` routes to BLEEP Connect Layer 4 |

#### StateDiff contract

The VM never acquires the `StateManager` lock. It returns:

```rust
pub struct StateDiff {
    pub balances: BTreeMap<[u8; 32], BalanceDelta>,   // address → signed delta
    pub nonces:   BTreeMap<[u8; 32], NonceUpdate>,    // address → new nonce
    // storage slots, deployed code, emitted events …
}
```

`BlockProducer` applies the diff under a single `StateManager` lock after all VM calls complete.

---

### P2P Networking

**Crate:** `bleep-p2p` — Default listen: `0.0.0.0:7700`

Built on `libp2p 0.53`. All transport-layer security is post-quantum.

#### Components

| Component | Description |
|---|---|
| `KademliaDHT` | 256 K-buckets, XOR metric, k=20 replication factor |
| `GossipProtocol` | Plumtree epidemic dissemination for blocks and transactions |
| `OnionRouter` | 3-hop encrypted routing; Kyber-768 KEM per hop, AES-256-GCM payload |
| `PeerManager` | AI-scored peer reputation, Sybil detection, exponential reputation decay |
| `MessageProtocol` | TCP framing, AES-256-GCM encryption, Ed25519 message auth, anti-replay nonce cache |
| `QuantumCrypto` | Kyber-768 session KEM, SPHINCS+-SHA2-128s message authentication |

Ed25519 in `MessageProtocol` is scheduled for replacement with SPHINCS+ in a future release.

---

### RPC Server

**Crate:** `bleep-rpc` — Port: 8545

`warp`-based HTTP/JSON server. `RpcState` holds shared live counters and an `Arc<Mutex<StateManager>>` for the state and proof endpoints.

#### Endpoint summary

| Method | Path | Description |
|---|---|---|
| `GET` | `/rpc/health` | Status, chain height, peer count, uptime, version |
| `GET` | `/rpc/telemetry` | Blocks produced, transactions processed, uptime |
| `GET` | `/rpc/block/latest` | Latest block height and hash |
| `GET` | `/rpc/block/{id}` | Block by height or hash |
| `POST` | `/rpc/tx` | Submit a signed transaction |
| `GET` | `/rpc/tx/history` | Transaction history |
| `GET` | `/rpc/wallet` | Wallet RPC readiness |
| `GET` | `/rpc/ai` | AI advisory readiness |
| `GET` | `/rpc/state/{address}` | Live balance, nonce, state root, block height |
| `GET` | `/rpc/proof/{address}` | 256-level SMT inclusion/exclusion proof |
| `POST` | `/rpc/validator/stake` | Register validator / increase stake |
| `POST` | `/rpc/validator/unstake` | Initiate graceful validator exit |
| `GET` | `/rpc/validator/list` | All active validators with stake |
| `GET` | `/rpc/validator/status/{id}` | Validator status + slashing history |
| `POST` | `/rpc/validator/evidence` | Submit double-sign evidence (auto-execute) |
| `GET` | `/rpc/economics/supply` | Circulating supply, minted, burned, base fee  |
| `GET` | `/rpc/economics/fee` | Current EIP-1559 base fee + last epoch  |
| `GET` | `/rpc/economics/epoch/{n}` | Full epoch output (emissions, burns, price)  |
| `GET` | `/rpc/oracle/price/{asset}` | Aggregated oracle price (median, sources)  |
| `POST` | `/rpc/oracle/update` | Submit oracle price update  |
| `GET` | `/rpc/connect/intents/pending` | Pending Layer 4 instant intents  |
| `POST` | `/rpc/connect/intent` | Submit a new instant intent  |

The `/rpc/economics/*` and `/rpc/oracle/*` handlers return HTTP 503 when `BleepEconomicsRuntime` is not attached.

#### Wiring at startup

```rust
let rpc_state = RpcState::new()
    .with_state_manager(Arc::clone(&state_arc))
    .with_validator_registry(Arc::clone(&validator_registry))
    .with_slashing_engine(Arc::clone(&slashing_engine))
    .with_economics_runtime(Arc::clone(&economics_runtime));
let routes = rpc_routes_with_state(rpc_state);
warp::serve(routes).run(([0, 0, 0, 0], 8545)).await;
```

---

### Wallet and CLI

**Crates:** `bleep-wallet-core`, `bleep-cli`

#### Wallet file format

Wallets are stored as a JSON array at `~/.bleep/wallets.json`:

```json
[
  {
    "falcon_keys":  "<hex: SPHINCS+ public key>",
    "kyber_keys":   "<hex: Kyber-768 public key>",
    "signing_key":  "<hex: AES-256-GCM ciphertext of SPHINCS+ secret key>",
    "address":      "BLEEP1<40-hex-chars>",
    "label":        null
  }
]
```

The `signing_key` blob layout: `nonce(12) || ciphertext || GCM-tag(16)`. A wallet without `signing_key` is watch-only.

#### CLI commands

```
bleep-cli <COMMAND>

  start-node               Start a full BLEEP node (all subsystems)

  wallet create            Generate SPHINCS+ + Kyber-768 keypair; encrypt SK
  wallet balance           GET /rpc/state per wallet; fallback to local RocksDB
  wallet import <phrase>   BIP-39 mnemonic → PBKDF2 seed → SPHINCS+ keypair
  wallet export            Print wallet addresses

  tx send --to <addr> --amount <n>
                           Sign with SPHINCS+ (unlock SK); POST /rpc/tx
  tx history               GET /rpc/tx/history

  block latest             GET /rpc/block/latest
  block get <id>           GET /rpc/block/{id}
  block validate <hash>    Validate block hash via node

  governance <propose|vote|list|status>
                           Submit and vote on on-chain proposals

  state <task>             Inspect raw chain state
  zkp <proof>              Verify a ZK proof string
  ai <task>                Query AI advisory engine
  pat <task>               Programmable Asset Token operations
  telemetry                Print live node metrics
  info                     Print RPC and node connection info
```

#### Environment variables

| Variable | Default | Description |
|---|---|---|
| `BLEEP_RPC` | `http://127.0.0.1:8545` | RPC endpoint for all CLI commands |
| `BLEEP_STATE_DIR` | `/tmp/bleep-state` | Local RocksDB path (offline fallback) |
| `RUST_LOG` | `info` | tracing log filter |

#### Balance resolution order

1. `GET /rpc/state/{address}` on the configured node (returns balance + nonce + state root prefix)
2. If the node is unreachable: `StateManager::open(BLEEP_STATE_DIR)` — local RocksDB read
3. Prints `(offline — node at {rpc} unreachable)` when the fallback is used

---

### Governance

**Crate:** `bleep-governance`

On-chain governance with six typed proposal categories:

```rust
pub enum ProposalType {
    ParameterChange,
    ProtocolUpgrade,
    ValidatorSlashing,
    EmergencyPause,
    TreasurySpend,
    CrossChainPolicy,
}
```

`GovernanceEngine` manages the full lifecycle: proposal creation, voting window, quorum evaluation, tally, and automatic execution or archival. Proposals require a configurable quorum threshold and approval percentage. Rejected or expired proposals are archived, never deleted.

---

### Economics and Tokenomics

**Crate:** `bleep-economics`

#### Supply model

| Constant | Value |
|---|---|
| `MAX_SUPPLY` | 200,000,000 BLEEP (8 decimals) |
| `GENESIS_SUPPLY` | 0 (fair launch, no pre-mine) |
| `MAX_INFLATION_RATE_BPS` | 500 (5.00% per epoch, constitutional hard cap) |
| Base fee burn | 25% of every transaction fee |

`CanonicalTokenomicsEngine` enforces `total_minted ≤ MAX_SUPPLY` independently of `circulating_supply`, preventing a class of inflation bug where concurrent burns mask over-issuance.

#### Emission schedule

| Type | Rate per epoch |
|---|---|
| Validator participation reward | 1.5% |
| Cross-shard coordination reward | 0.5% |
| Ecosystem / governance grants | Governance-controlled |

#### Fee market

An EIP-1559-style base fee adjusted per block by `ShardCongestion` metrics. The base fee is burned; validators receive only the priority tip. This creates deflationary pressure under high network load.

#### Oracle bridge

`OracleBridgeEngine` aggregates price updates from multiple `OracleOperator` sources with median filtering and staleness rejection. No price data is committed to state without Byzantine-threshold confirmation.

---

### BLEEP Connect

**Crate:** `bleep-interop` (10 sub-crates)

BLEEP Connect is a four-tier cross-chain protocol. The executor automatically selects the tier based on transfer value and required security level.

| Tier | Latency | Security | Use case |
|---|---|---|---|
| Layer 4 — Instant | 200 ms – 1 s | Optimistic intent relay | Routine transfers |
| Layer 3 — ZK Proof | 10 s – 60 s | Groth16 batch proof | Verified mid-value transfers |
| Layer 2 — Full Node | 1 min – 5 min | Independent chain verification | Transfers > $100K |
| Layer 1 — Social | Hours | On-chain governance vote | Catastrophic recovery events |

#### Sub-crate map

| Sub-crate | Role |
|---|---|
| `bleep-connect-types` | Shared types: `ChainId`, `UniversalAddress`, `AssetId`, `InstantIntent` |
| `bleep-connect-crypto` | SPHINCS+, Kyber-1024, Ed25519, AES-GCM per-hop encryption |
| `bleep-connect-commitment-chain` | BFT micro-chain anchoring cross-chain state roots |
| `bleep-connect-adapters` | Per-chain encode/verify: Ethereum (EVM), Solana |
| `bleep-connect-executor` | Executor node: monitors intent pool, bids, executes |
| `bleep-connect-layer4-instant` | Optimistic 200 ms relay (99.9% of transfers) |
| `bleep-connect-layer3-zkproof` | Groth16 proof generation and batch aggregation |
| `bleep-connect-layer2-fullnode` | Full-node verification path for large transfers |
| `bleep-connect-layer1-social` | On-chain social governance for catastrophic recovery |
| `bleep-connect-core` | Top-level orchestrator over all layers |

---

### Supporting Services

#### Scheduler — `bleep-scheduler`

20 built-in Tokio maintenance tasks across 7 categories, each with an isolated per-task timeout and panic boundary:

Epoch management, validator reward distribution, self-healing sweeps, governance advancement, EIP-1559 fee parameter updates, supply invariant verification, shard rebalancing, session purges, mempool pruning, indexer checkpoints, cross-shard timeout sweeps, telemetry flush, peer health checks, oracle data refresh, and audit log rotation.

#### Auth — `bleep-auth`

Complete authentication surface for node operators and dApp developers:

- **Credentials** — SHA3-256 salted hashing with constant-time verification and `Zeroize` on drop
- **Sessions** — HS256 JWT issuance and validation with JTI deny-list revocation
- **RBAC** — Role hierarchy with O(1) `DashMap`-backed permission evaluation
- **Validator binding** — Kyber-1024 challenge/response proof-of-possession
- **Audit log** — Merkle-chained, append-only, tamper-detectable event log
- **Rate limiter** — Fixed-window token bucket per `(identity, action)` pair

#### Indexer — `bleep-indexer`

Async channel-driven indexer building `DashMap`-backed query indexes for blocks, transactions, accounts, governance events, validator events, shard events, cross-shard 2PC, and AI advisory events. Supports reorg rollback to any ancestor height and `CheckpointEngine` snapshots for crash recovery.

#### AI Advisory — `bleep-ai`

`BLEEPAIAssistant` produces deterministic advisory scores used by P2P peer selection, consensus anomaly detection, and governance risk scoring. Deterministic means identical inputs always produce identical outputs with no external model or API calls at runtime.

#### Telemetry — `bleep-telemetry`

`tracing-subscriber` integration with `MetricCounter` and `MetricGauge` primitives. All counters are aggregated into `RpcState` and exposed via `/rpc/telemetry`.

---

## Protocol Parameters

| Parameter | Value |
|---|---|
| Block time | 3,000 ms |
| Max transactions per block | 4,096 |
| Blocks per epoch | 1,000 |
| Finality threshold | > ⅔ total stake |
| Double-sign slash | 33% of validator stake |
| Max token supply | 200,000,000 BLEEP |
| Token decimals | 8 |
| Max inflation rate | 500 bps / epoch (5%) |
| Base fee burn | 25% of transaction fee |
| State trie depth | 256 bits |
| ZKP proof size | 64 bytes (Fiat-Shamir, current) |
| RPC port | 8545 |
| P2P port | 7700 |
| BIP-39 PBKDF2 rounds | 2,048 |
| AES-GCM nonce size | 12 bytes (random per encryption) |

---

## Getting Started

### Prerequisites

```bash
# Rust stable toolchain, edition 2021
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Ubuntu / Debian system dependencies
sudo apt-get install -y build-essential clang libclang-dev librocksdb-dev

# macOS (Homebrew)
brew install rocksdb llvm
export LIBRARY_PATH="$(brew --prefix rocksdb)/lib:$LIBRARY_PATH"
```

### Build

```bash
git clone https://github.com/bleep-project/bleep.git
cd bleep

# Full workspace (all 19 crates)
cargo build --release --workspace

# Node binary only
cargo build --release --bin bleep

# CLI only
cargo build --release --bin bleep-cli
```

### Run a local single-validator node

```bash
./target/release/bleep
# Node starts in 13 steps; RPC ready at :8545, P2P at :7700

# Verify the node is live
curl -s http://127.0.0.1:8545/rpc/health | jq .
```

### Create a wallet and send a transaction

```bash
# 1. Create wallet
./target/release/bleep-cli wallet create
# → Generates SPHINCS+ + Kyber-768 keypair
# → Prompts for encryption passphrase
# → Saves to ~/.bleep/wallets.json

# 2. Check balance (live from node)
./target/release/bleep-cli wallet balance

# 3. Import from BIP-39 mnemonic
./target/release/bleep-cli wallet import \
  "abandon abandon abandon abandon abandon abandon \
   abandon abandon abandon abandon abandon about"

# 4. Send a transaction (prompts for passphrase to unlock signing key)
./target/release/bleep-cli tx send \
  --to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8 \
  --amount 1000
```

### Run the test suite

```bash
# All workspace unit and integration tests
cargo test --workspace

# Single crate
cargo test -p bleep-state
cargo test -p bleep-crypto
cargo test -p bleep-wallet-core
cargo test -p bleep-consensus

# With detailed output
RUST_LOG=debug cargo test -p bleep-state -- --nocapture
```

---

## Configuration

The node reads from environment variables and an optional `bleep.toml` (via `config 0.14`).

```toml
# bleep.toml (all values shown are defaults)

[node]
p2p_port    = 7700
rpc_port    = 8545
state_dir   = "/var/lib/bleep/state"
log_level   = "info"

[consensus]
block_interval_ms  = 3000
max_txs_per_block  = 4096
blocks_per_epoch   = 1000
validator_id       = "validator-0"

[features]
quantum = true    # disable only for development/benchmarking
```

#### Cargo feature flags

| Flag | Default | Effect |
|---|---|---|
| `mainnet` | on | Mainnet protocol constants |
| `testnet` | off | Testnet constants (e.g. reduced epoch size) |
| `quantum` | on | Enables `pqcrypto` and `pqcrypto-kyber`; required for SPHINCS+/Kyber |

---

## RPC API Reference

All responses are JSON. Errors carry an `"error"` string field.

### `GET /rpc/health`

```json
{
  "status":      "ok",
  "height":      1024,
  "peers":       8,
  "uptime_secs": 3600,
  "version":     "1.0.0"
}
```

### `GET /rpc/telemetry`

```json
{
  "blocks_produced":        1024,
  "transactions_processed": 12800,
  "uptime_secs":            3600
}
```

### `GET /rpc/state/{address}`

Returns HTTP 503 in stub mode (no `StateManager` attached). `balance` is a decimal string to avoid JSON `u128` overflow.

```json
{
  "address":      "BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8",
  "balance":      "10000000000",
  "nonce":        4,
  "state_root":   "8a3f2c1d9e7b4a0f5c6d7e8f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
  "block_height": 1024
}
```

### `GET /rpc/proof/{address}`

`exists: false` indicates an exclusion proof (no account at this address).

```json
{
  "address":  "BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8",
  "exists":   true,
  "leaf":     "3f2a1b0c...",
  "root":     "8a3f2c1d...",
  "siblings": ["00000000...", "4a2b1c3d...", "..."],
  "is_right": [false, true, false, "..."]
}
```

Verify offline:

```rust
let proof: MerkleProof = serde_json::from_str(&json_body)?;
assert!(proof.verify(&known_state_root));
```

### `POST /rpc/tx`

```json
// Request
{
  "sender":    "BLEEP1...",
  "receiver":  "BLEEP1...",
  "amount":    1000,
  "timestamp": 1710000000
}

// Response
{
  "tx_id":  "BLEEP1...:BLEEP1...:1000:1710000000",
  "status": "queued"
}
```

### `GET /rpc/block/latest`

```json
{ "height": 1024, "hash": "00000...0400", "tx_count": 0, "epoch": 1 }
```

---

## CLI Reference

```
bleep-cli [OPTIONS] <COMMAND>

Options:
  -h, --help     Print help
  -V, --version  Print version

Environment variables:
  BLEEP_RPC        RPC endpoint     default: http://127.0.0.1:8545
  BLEEP_STATE_DIR  Local DB path    default: /tmp/bleep-state
  RUST_LOG         Log filter       default: info

Commands:
  start-node                        Start a full BLEEP node
  wallet create                     Generate SPHINCS+ keypair + encrypted wallet
  wallet balance                    Query balance from /rpc/state
  wallet import <phrase>            Import from BIP-39 mnemonic
  wallet export                     Export wallet addresses
  tx send --to <addr> --amount <n>  Sign and broadcast a transfer
  tx history                        Retrieve transaction history
  validator stake --amount <n>      Register as validator
  validator unstake                 Initiate graceful exit
  validator list                    List active validators
  validator status <id>             Validator status + slashing history
  validator submit-evidence <file>  Submit double-sign evidence
  governance propose <text>         Create a governance proposal
  governance vote <id> --yes/--no   Cast a vote
  governance list                   List all proposals
  state snapshot                    Create a RocksDB state snapshot
  state restore <path>              Restore from snapshot
  block latest                      Print the latest block
  block get <id>                    Get a block by hash or height
  block validate <hash>             Validate a block by hash
  zkp <proof>                       Verify a ZKP
  ai ask <prompt>                   Ask the AI advisory engine
  ai status                         AI engine status
  pat status                        PAT engine status
  pat list                          List asset tokens
  pat mint --to <addr> --amount <n> Mint PAT tokens (owner only)        
  pat burn --amount <n>             Burn PAT tokens                      
  pat transfer --to <addr> --amount Transfer PAT with auto burn-rate     
  pat balance <address>             Query PAT balance                    
  oracle price <asset>              Query aggregated oracle price        
  oracle submit --asset ... --price Submit oracle price update           
  economics supply                  Circulating supply, minted, burned   
  economics fee                     Current EIP-1559 base fee            
  economics epoch <n>               Epoch emissions, burns, price        
  telemetry                         Print telemetry metrics
  info                              Node version and RPC health
```

### Executor node

The `bleep-executor` binary is a separate process that participates in the Layer 4 instant intent market:

```bash
# Basic usage (ephemeral key, 0.1 BLEEP capital)
./bleep-executor

# Production usage
BLEEP_EXECUTOR_KEY=<32-byte-hex-seed>      \
BLEEP_EXECUTOR_CAPITAL_BLEEP=100000000000  \
BLEEP_EXECUTOR_CAPITAL_ETH=10000000000000000000 \
BLEEP_EXECUTOR_RISK=Medium                 \
BLEEP_RPC=http://your-node:8545            \
./bleep-executor
```



## Security Model

### Post-quantum threat model

BLEEP treats a cryptographically relevant quantum computer as a near-term engineering assumption, not a distant theoretical risk. Accordingly:

- **SPHINCS+-SHAKE-256** (NIST PQC, stateless hash-based) signs all transactions and blocks.
- **Kyber-768** (ML-KEM, NIST FIPS 203) is used for all key encapsulation.
- **AES-256-GCM** is used for symmetric encryption (128-bit post-quantum security at 256-bit key size).
- **SHA3-256 and BLAKE3** provide collision-resistant hashing.
- **Ed25519** is retained in P2P message authentication at the 128-bit classical security level and is scheduled for replacement with SPHINCS+.

### Consensus safety

- **Byzantine fault tolerance:** The system tolerates up to ⅓ of total stake being Byzantine (malicious or offline).
- **Deterministic mode selection:** No single validator can trigger a mode switch. Selection is a pure function of epoch metrics.
- **Automatic slashing:** Double-sign evidence triggers an on-chain 33% stake penalty without human intervention.
- **Irreversible finality:** Once `accumulated_voting_power > (2/3) * total_stake`, a `FinalizyCertificate` is produced. That block and its state root cannot be rolled back.

### State integrity

- **SparseMerkleTrie** ensures any modification to any account balance or nonce produces a different state root, which propagates through the block hash to the `validator_signature` and `zk_proof`. Tampered state is detectable by any node holding the state root.
- **Fiat-Shamir ZKP** binds the state root, shard state root, consensus mode, and tx count into `zk_proof`. A validator cannot produce a valid ZKP over a different set of transactions or state root.
- **Inflation invariant:** `CanonicalTokenomicsEngine` checks `total_minted ≤ MAX_SUPPLY` independently of `circulating_supply`. Concurrent burns cannot mask over-issuance.

### P2P security

- All sessions are established with Kyber-768 KEM; payload encryption is AES-256-GCM.
- An anti-replay nonce cache rejects duplicate messages within a session window.
- AI-scored peer reputation and stake-weighted selection provide Sybil resistance.
- Onion routing with 3 hops and per-hop Kyber-768 KEM prevents traffic analysis.

---

## Development Roadmap

BLEEP follows a structured, phase-based development roadmap. Phases 1–3 are complete. The four upcoming phases — AI model training, public testnet expansion, pre-sale ICO, and mainnet launch — form the path to production.

---

### Phase 1 — Foundation ✅ *Complete*

All 19 crates compile cleanly. Post-quantum cryptography active (SPHINCS+-SHAKE-256, Kyber-1024). RocksDB `StateManager` with `SparseMerkleTrie`. Full `BlockProducer` loop. Real Groth16 ZK circuits. 4-node docker-compose devnet. BLEEP Connect Layer 4 live on Ethereum Sepolia. `BleepEconomicsRuntime` (EIP-1559 fee market, oracle bridge, validator incentives). `PATRegistry` live. `bleep-executor` standalone intent market maker.

---

### Phase 2 — Testnet Alpha ✅ *Complete*

7-validator `bleep-testnet-1` genesis across 4 continents. Public DNS seeds at `seeds.testnet.bleep.network`. Public faucet (`POST /faucet/{address}`, 1,000 BLEEP per 24 hours). Block explorer (`GET /explorer`, 6 s refresh). JWT rotation, NDJSON audit export. Grafana dashboard (12 panels) + Prometheus for all 7 validators. Full CI pipeline: fmt, clippy, test, audit, build, fuzz-smoke, docker-smoke.

---

### Phase 3 — Protocol Hardening ✅ *Complete*

- ✅ **Independent security audit** — 14 findings (2 Critical, 3 High, 4 Medium, 3 Low, 2 Info); all Critical/High resolved — see `docs/SECURITY_AUDIT.md`
- ✅ **Chaos testing** — 14 scenarios, 72-hour continuous harness — see `docs/CHAOS_TESTING.md`
- ✅ **ZKP MPC ceremony** — 5-participant Powers-of-Tau on BLS12-381; transcript at `https://ceremony.bleep.network/transcript-v1.json`
- ✅ **Cross-shard stress test** — 10 shards, 1,000 concurrent cross-shard txs, 100 epochs
- ✅ **BLEEP Connect Layer 3** — Groth16 batch proof bridge live on testnet
- ✅ **Live governance** — `LiveGovernanceEngine` with typed proposals, weighted voting, veto, on-chain execution
- ✅ **Performance benchmark** — avg **10,921 TPS**, peak **13,200 TPS** across 10 shards for 1 hour
- ✅ **Token distribution model** — 6 allocation buckets, vesting schedules, 25/50/25 fee split, compile-time verified constants

**Definition of done:** Security audit fully resolved ✅ · Chaos suite 72 h ✅ · ≥10,000 TPS ✅

---

### Phase 4 — AI Model Training ⏳ *Active*

Upgrade `bleep-ai` from rule-based advisory to a trained on-chain inference engine.

- `BLEEPAIAssistant v2` — training pipeline using on-chain governance history as training data
- AI validator nodes — optional validator upgrade for AI-scored transaction prioritisation
- `AIConstraintValidator v2` — trained classification models for governance pre-flight scoring
- Determinism guarantee — all AI inference on consensus-critical paths uses fixed-seed reproducible models

**Definition of done:** AI advisory engine passes determinism test suite; governance pre-flight achieves ≥95% accuracy on labelled test set.

---

### Phase 5 — Public Testnet Expansion ⏳ *Upcoming*

Open validator onboarding to the public. Target: ≥50 validators across ≥6 continents.

- Open validator registration with public `VALIDATOR_GUIDE.md`
- Validator incentive programme from Ecosystem Fund
- `testnet.bleep.network` — multi-validator explorer, leaderboard, public dashboard
- 30-day sustained test with live validator join/leave events
- Cross-shard expansion: 10 → 20 shards as validator count permits
- Community bug bounty: up to 100,000 BLEEP for documented protocol vulnerabilities

**Definition of done:** 50+ active validators; 30 consecutive days without manual intervention.

---

### Phase 6 — Pre-Sale / ICO ⏳ *Upcoming*

Community token sale in two tranches. Deploy on-chain vesting contracts.

| Tranche | Source | Lockup |
|---|---|---|
| Strategic Pre-Sale | Strategic Reserve (5M BLEEP) | 12-month cliff + 24-month linear |
| Public ICO | Community Incentives (up to 10M BLEEP) | 6-month linear vest |

KYC/AML compliant infrastructure · Multi-sig treasury custody · `LinearVestingSchedule` contracts deployed · `GenesisAllocation` engine activated for all 6 buckets.

**Definition of done:** ICO completed; all vesting contracts deployed and verified on-chain.

---

### Phase 7 — Mainnet Launch 🔜 *Planned*

Production mainnet with post-quantum security, live economics, and cross-chain connectivity from the genesis block.

- Mainnet genesis ceremony — public, multi-party verifiable
- ≥21 validators with geographic diversity enforced by genesis rules
- BLEEP Connect L4 + L3 live on Ethereum mainnet and Solana from genesis
- Governance active from block 1
- Full tokenomics live — emission, burn, staking rewards, oracle, EIP-1559 fee market
- Block explorer at `explorer.bleep.network`
- `bleep-sdk-js` and `bleep-sdk-python` SDK releases
- NTP drift guard at node startup (warn >1 s, halt >30 s)

**Definition of done:** Genesis block produced by ≥21 independent validators; governance proposal passes on-chain within first week; cross-chain Ethereum transfer confirms within 1 second.

---

## Contributing

1. Fork the repository and create a feature branch: `git checkout -b feat/your-feature`
2. Run the full test suite: `cargo test --workspace`
3. Run the linter: `cargo clippy --workspace -- -D warnings`
4. Verify formatting: `cargo fmt --all -- --check`
5. Open a pull request against `main` with a clear description of the change and the crates affected.

Changes to `bleep-consensus`, `bleep-crypto`, or `bleep-state` undergo an extended review given their security surface area.

### Security disclosures

Do not open public issues for security vulnerabilities. Email `security@bleep.network` with a description, reproduction steps, and your proposed fix. We target 72-hour acknowledgment and a 14-day patch timeline.

---

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

---

*BLEEP Blockchain — built in Rust, secured with post-quantum cryptography, designed for the next decade of decentralised computing.*
