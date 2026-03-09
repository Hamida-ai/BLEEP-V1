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

Full Groth16 SNARK circuit (ark-bls12-381) replaces this in Sprint 6.

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

`BlockProducer` applies the diff under a single `StateManager` lock after all VM calls complete, eliminating the deadlock class from the Sprint 2 design.

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

Ed25519 in `MessageProtocol` is scheduled for replacement with SPHINCS+ in Sprint 6.

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
| `GET` | `/rpc/economics/supply` | Circulating supply, minted, burned, base fee *(Sprint 7)* |
| `GET` | `/rpc/economics/fee` | Current EIP-1559 base fee + last epoch *(Sprint 7)* |
| `GET` | `/rpc/economics/epoch/{n}` | Full epoch output (emissions, burns, price) *(Sprint 7)* |
| `GET` | `/rpc/oracle/price/{asset}` | Aggregated oracle price (median, sources) *(Sprint 7)* |
| `POST` | `/rpc/oracle/update` | Submit oracle price update *(Sprint 7)* |
| `GET` | `/rpc/connect/intents/pending` | Pending Layer 4 instant intents *(Sprint 7)* |
| `POST` | `/rpc/connect/intent` | Submit a new instant intent *(Sprint 7)* |

The `/rpc/economics/*` and `/rpc/oracle/*` handlers return HTTP 503 when `BleepEconomicsRuntime` is not attached.

#### Wiring at startup

```rust
let rpc_state = RpcState::new()
    .with_state_manager(Arc::clone(&state_arc))
    .with_validator_registry(Arc::clone(&validator_registry))
    .with_slashing_engine(Arc::clone(&slashing_engine))
    .with_economics_runtime(Arc::clone(&economics_runtime));  // Sprint 7
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
  pat mint --to <addr> --amount <n> Mint PAT tokens (owner only)        [Sprint 7]
  pat burn --amount <n>             Burn PAT tokens                      [Sprint 7]
  pat transfer --to <addr> --amount Transfer PAT with auto burn-rate     [Sprint 7]
  pat balance <address>             Query PAT balance                    [Sprint 7]
  oracle price <asset>              Query aggregated oracle price        [Sprint 7]
  oracle submit --asset ... --price Submit oracle price update           [Sprint 7]
  economics supply                  Circulating supply, minted, burned   [Sprint 7]
  economics fee                     Current EIP-1559 base fee            [Sprint 7]
  economics epoch <n>               Epoch emissions, burns, price        [Sprint 7]
  telemetry                         Print telemetry metrics
  info                              Node version and RPC health
```

### Executor node (Sprint 7)

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

- **SPHINCS+-SHAKE-256** (NIST PQC, stateless hash-based) signs all transactions and is the planned block signing scheme (replacing the current SHA3 scheme in Sprint 6).
- **Kyber-768** (ML-KEM, NIST FIPS 203) is used for all key encapsulation.
- **AES-256-GCM** is used for symmetric encryption (128-bit post-quantum security at 256-bit key size).
- **SHA3-256 and BLAKE3** provide collision-resistant hashing.
- **Ed25519** is retained in P2P message authentication at the 128-bit classical security level. It will be replaced with SPHINCS+ in Sprint 6.

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

## Roadmap to Public Testnet

Six sprints complete the journey from the current single-validator local node to a publicly accessible, adversarially tested mainnet. Each sprint has a concrete set of engineering deliverables and a binary definition of done.

---

### Sprint 1 — Build Stability ✅ *Complete*

All 20 workspace crates compile cleanly. Zero hard build errors. No `unwrap()` panics in hot paths.

---

### Sprint 2 — State and Execution ✅ *Complete*

RocksDB `StateManager`, full `BlockProducer` execution loop (mempool → VM → state → sign → gossip), `bleep-vm` wired returning `StateDiff`, `MempoolBridge` 500 ms drain.

---

### Sprint 3 — Correctness and P2P ✅ *Complete*

Two-phase VM/lock eliminating `StateManager` deadlock. `GossipBridge` connecting block output to `P2PNode`. `SparseMerkleTrie` replacing hash-of-pairs root. Correct 32-byte block signing.

---

### Sprint 4 — End-to-End and Inbound Sync ✅ *Complete*

BIP-39 PBKDF2-HMAC-SHA512 derivation. SPHINCS+-SHAKE-256 transaction signing. `EncryptedWallet.signing_key`. `MerkleProof::verify()`. `InboundBlockHandler` Tokio task. `StateDiff` unified with `StateManager`.

---

### Sprint 5 — Hardening ✅ *Complete*

O(256) Merkle proofs. Full nonce accounting via `StateDiff.nonces`. 64-byte Fiat-Shamir ZKP binding all semantic block fields. AES-256-GCM wallet key encryption with `lock()`/`unlock()`. RPC `/rpc/state` and `/rpc/proof` with live `StateManager` integration. Per-tx SPHINCS+ sig verification in `InboundBlockHandler`. CLI `wallet balance` from live RPC with offline fallback.

---

### Sprint 6 — ZK Circuits and Multi-Validator Devnet ✅ *Complete*

Real Groth16 block validity circuit (ark-bls12-381, 5 public inputs, 6 R1CS constraints). Groth16 batch aggregation for tx proofs. 4-node docker-compose devnet with shared genesis. Validator stake/unstake via CLI. SPHINCS+ block signing migration replacing SHA3 scheme. Slashing auto-executed on double-sign evidence. 5 new `/rpc/validator/*` endpoints. `ValidatorRegistry` and `SlashingEngine` wired into the node.

---

### Sprint 7 — Cross-Chain Alpha and Live Economics ✅ *Complete — current codebase*

**Goal:** BLEEP Connect Layer 4 operational on Ethereum Sepolia. Tokenomics engine live. PAT mint/burn/transfer fully wired.

**Delivered:**

- **`BleepEconomicsRuntime`** (`crates/bleep-economics/src/runtime.rs`) — full epoch processor live:
  - `FeeMarket::update_base_fee()` — EIP-1559-style base fee adjustment per epoch
  - `OracleBridgeEngine::aggregate_prices("BLEEP/USD")` — 3-of-5 operator quorum with **cryptographic signature verification** (Sprint 7)
  - `ValidatorIncentivesEngine::compute_epoch_rewards()` — per-validator reward records distributed at epoch boundary
  - `CanonicalTokenomicsEngine::record_emission() / record_burn() / finalize_epoch()` — supply accounting with hash commitment
  - Full invariant: `circulating = minted − burned` verified after every epoch
- **`SepoliaRelay`** (`crates/bleep-interop/src/bleep-connect-adapters/`) — Ethereum Sepolia testnet relay:
  - `SepoliaRelay::build_relay_tx()` — ABI-encodes a `fulfillIntent(bytes32,address,uint256,uint256)` call
  - `SepoliaRelay::simulate_relay()` — local calldata validation before broadcast
  - `SepoliaRelay::relay_status()` — tx hash → relay status mapping
  - Contract: `SEPOLIA_BLEEP_FULFILL_ADDR` on Sepolia (`chain_id = 11155111`)
- **`BleepConnectOrchestrator` Sprint 7 API extensions:**
  - `pending_intent_ids()` / `get_pending_intent()` — live intent pool for executor polling
  - `build_sepolia_relay_tx()` — builds relay tx for any Ethereum-bound intent
  - `sepolia_relay_status()` — relay confirmation status
  - `start_background_tasks()` — hook for node startup sequence
- **5 new `/rpc/connect/*` endpoints:**
  - `GET /rpc/connect/intents/pending` — live Layer 4 intent pool (backed by real `Layer4Instant`)
  - `POST /rpc/connect/intent` — submit intent to live pool with full `InstantIntent` fields
  - `GET /rpc/connect/intent/{id}` — status with all 11 `TransferStatus` variants
  - `GET /rpc/connect/intent/{id}/relay_tx` — Sepolia relay tx for Ethereum-bound intents
- **`PATRegistry`** (`crates/bleep-pat/src/pat_engine.rs`) — production PAT engine:
  - `create_token()` — configurable supply cap, burn rate (max 1000 bps), 8 decimals default
  - `mint()` — owner-only, supply cap enforced
  - `burn()` — any holder, updates `total_burned` + state hash
  - `transfer()` — automatic deflationary burn deduction, returns net amount received
  - Full event log: `TokenCreated`, `Mint`, `Burn`, `Transfer`
- **7 new `/rpc/pat/*` endpoints:**
  - `POST /rpc/pat/create` — create a new PAT token
  - `POST /rpc/pat/mint` — mint tokens (owner only)
  - `POST /rpc/pat/burn` — burn tokens
  - `POST /rpc/pat/transfer` — transfer with auto-burn
  - `GET /rpc/pat/balance/{symbol}/{address}` — token balance
  - `GET /rpc/pat/info/{symbol}` — token metadata + supply stats
  - `GET /rpc/pat/list` — all registered tokens
- **`bleep-cli pat` fully wired** (7 subcommands):
  - `create --symbol --name --owner --supply-cap --burn-rate-bps`
  - `mint --symbol --from --to --amount`
  - `burn --symbol --from --amount`
  - `transfer --symbol --from --to --amount`
  - `balance --symbol <address>`
  - `info <symbol>`
  - `list`
- **`bleep-executor` standalone binary** (`src/bin/executor.rs`) — Layer 4 intent market maker with bid/claim loop
- **Oracle signature verification live** — `submit_price_update` verifies `SHA-256(operator_id || asset || price || ts)` signatures; 64-byte zero placeholders accepted in devnet mode; `InvalidSignature` error + `rejected_updates` counter added
- **3 `/rpc/economics/*` + 2 `/rpc/oracle/*` endpoints** live and wired to `BleepEconomicsRuntime`
- **Node startup** — `BleepConnectOrchestrator` + `PATRegistry` initialized in `main.rs`, wired into `RpcState`

**Definition of done:** 1,000 cross-chain transfers complete with full commitment chain proof; tokenomics emission and burns match the constitutional schedule for 100 epochs; PAT mint/burn/transfer all operational via CLI; Sepolia relay tx generation verified for all Ethereum-bound intents.

---

### Sprint 8 — Public Testnet Alpha ✅ *Complete*

### Sprint 9 — Testnet Hardening ✅ *Complete*

**Goal:** Publicly accessible testnet with external validators, a faucet, and a block explorer.

**Infrastructure:**

- 7 geographically distributed validator nodes (minimum 3 continents)
- Public DNS seed nodes: `seeds.testnet.bleep.network`
- Persistent chain state; no resets after genesis
- `testnet` Cargo feature active (100 blocks per epoch)

**Deliverables:**

- ✅ **`testnet-genesis.toml`** — 7-validator genesis with 4 continents, pre-funded faucet and treasury accounts, oracle bonds, executor bonds
- ✅ **Node join guide** — `docs/VALIDATOR_GUIDE.md` — step-by-step operator onboarding with hardware requirements, keygen, staking, systemd and Docker configs, slashing conditions, and troubleshooting
- ✅ **Public faucet** — `POST /faucet/{address}` dispensing 1,000 test BLEEP per address per 24 hours; rate-limited by both IP (`X-Forwarded-For`) and address; faucet balance tracked live; `GET /faucet/status` for monitoring
- ✅ **Block explorer** — read-only web UI at `GET /explorer` with live chain height, peer count, validator table, block list; auto-refreshes every 6 s; backed by `GET /rpc/explorer/blocks` and `GET /rpc/explorer/validators`
- ✅ **`bleep-auth` hardening** — async JWT secret rotation via `POST /rpc/auth/rotate` with base64 secret validation; `rotation_count` tracked; audit log NDJSON export at `GET /rpc/auth/audit` with optional `?limit=N`; `export_ndjson()` and `export_range_ndjson()` methods on `AuditLog`
- ✅ **Telemetry dashboard** — Grafana JSON dashboard (`devnet/grafana/dashboards/bleep-testnet-overview.json`) with 12 panels; Prometheus scrape config for all 7 validators; provisioning configs for automated Grafana setup; `GET /metrics` Prometheus text endpoint on every node
- ✅ **Full CI pipeline** — `.github/workflows/ci.yml` with 7 jobs: `fmt` (rustfmt --check), `clippy` (deny warnings + unwrap_used), `test` (full workspace), `audit` (cargo-audit CVE scan), `build` (release binary), `fuzz-smoke` (60 s libFuzzer runs), `docker-smoke` (health + faucet + explorer + metrics checks)
- ✅ **Security audit preparation** — `docs/THREAT_MODEL.md` with 30 catalogued threats across 6 categories, full invariant list (18 invariants), trust boundary map, known gaps for Sprint 9; `proptest` property tests in `bleep-state/src/proptest_sprint8.rs` (8 properties); `cargo-fuzz` targets for `bleep-crypto` (hash, SPHINCS+, Kyber) and `bleep-state` (Merkle, state apply_tx)
- ✅ **7-validator docker-compose** — `devnet/docker-compose-testnet.yml` with health checks, Prometheus, and Grafana

**Definition of done:** External validators join without assistance, the chain produces blocks for 7 consecutive days without intervention, and the public block explorer shows live data updated every block.

---

### Sprint 9 — Testnet Hardening ✅ *Complete*

**Goal:** Adversarial robustness demonstrated before mainnet commit.

**Deliverables:**

- ✅ **Independent security audit** — 14 findings (2 Critical, 3 High, 4 Medium, 3 Low, 2 Info); all Critical/High resolved; see `docs/SECURITY_AUDIT_SPRINT9.md`
- ✅ **Chaos testing** — 14 adversarial scenarios (crashes, partitions, reorgs, double-sign, replay, eclipse, flood, load); 72-hour continuous harness; see `docs/CHAOS_TESTING.md`
- ✅ **ZKP MPC ceremony** — 5-participant Powers-of-Tau ceremony for BLS12-381; transcript published at `https://ceremony.bleep.network/transcript-v1.json`; GET `/rpc/ceremony/status`
- ✅ **Cross-shard stress test** — 10-shard configuration; 1,000 concurrent cross-shard transactions; 100 epochs; all txs committed or rolled back
- ✅ **BLEEP Connect Layer 3 (ZK Proof)** — Groth16 batch proof bridge live on testnet; `crates/bleep-interop/src/layer3_bridge.rs`; POST `/rpc/layer3/intent`
- ✅ **Governance live** — `LiveGovernanceEngine` with typed parameter-change proposals, weighted voting, on-chain execution, veto mechanism; POST `/rpc/governance/propose` + `/vote`
- ✅ **Performance benchmark** — avg **10,921 TPS**, peak **13,200 TPS**, min **9,840 TPS** across 10 shards for 1 hour; GET `/rpc/benchmark/latest`
- ✅ **All audit findings addressed** — `AuditReport::sprint9_report()` verified in CI `audit` job on every PR
- ✅ **Audit fixes** — `GlobalNullifierSet` (SA-C1), JWT entropy gate (SA-C2), proxy CIDR guard (SA-H1), RocksDB CAS (SA-H2), gossip size cap (SA-H3), `AuditLogStore` persistence (SA-L1), `Zeroizing<>` for SK (SA-L3)
- ✅ **12 CI jobs** — fmt, clippy, test, audit, build, fuzz-smoke, chaos-smoke, mpc-ceremony, layer3-bridge, governance-live, docker-smoke, benchmark-report
- ✅ **441-line integration test suite** — `tests/sprint9_integration.rs`

**Definition of done:** Security audit report fully resolved ✅ · Chaos suite passes 72 hours ✅ · 10,000 TPS benchmark met (avg 10,921 TPS) ✅

---

### Sprint 10 — Mainnet Launch 🔜 *Planned*

**Goal:** Production mainnet with post-quantum security, live economics, and cross-chain connectivity from genesis.

**Deliverables:**

- **Mainnet genesis ceremony** — public, multi-party verifiable genesis block construction
- **Validator onboarding program** — documentation, tooling, and a minimum 21-validator initial set with geographic diversity
- **BLEEP Connect mainnet** — Layer 4 and Layer 3 live on Ethereum mainnet and Solana mainnet from day one
- **Governance from genesis** — community-controlled parameter upgrades, treasury spend proposals active immediately
- **Full tokenomics** — emission, burn, staking rewards, oracle price feed, and EIP-1559 fee market all active
- **Mainnet block explorer** — production deployment at `explorer.bleep.network`
- **SDK releases** — `bleep-sdk-js` and `bleep-sdk-python` wrapping the complete RPC API
- **Developer documentation** — contract deployment guide (EVM and WASM), cross-chain integration guide, validator operations manual

**Definition of done:** Mainnet genesis block produced by ≥ 21 independent validators, a governance proposal passes on-chain within the first week, and a cross-chain transfer from Ethereum mainnet confirms within 1 second.

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
