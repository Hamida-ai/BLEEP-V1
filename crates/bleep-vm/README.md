# BLEEP VM v0.5 — 7-Layer Intent-Driven Universal Execution Engine

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│  Layer 1 — Intent Layer                                             │
│                                                                     │
│  TransferIntent | ContractCallIntent | DeployIntent                 │
│  CrossChainIntent | ZkVerifyIntent                                  │
│                                                                     │
│  Everything is an intent. No raw bytecode at the API surface.       │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 2 — VM Router (vm_router.rs)                                 │
│                                                                     │
│  • Verifies Ed25519 signatures                                      │
│  • Detects VM type from magic bytes (Auto mode)                     │
│  • Enforces per-intent gas caps (default: 30M gas)                  │
│  • Circuit-breaker per engine (5 failures → 30s backoff)            │
│  • Per-chain VM overrides (e.g. Ethereum → always EVM)              │
│  • Routing metrics (total intents, success/fail, gas)               │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 3 — Execution Engines (completely isolated)                  │
│                                                                     │
│  EvmEngine    — revm (Foundry's EVM, Berlin/London/Shanghai)       │
│  WasmEngine   — Wasmer 4.2 + Cranelift JIT                         │
│  ZkEngine     — Groth16 on BN254 (ark-groth16)                     │
│                                                                     │
│  Bug in EVM cannot affect WASM. Bug in WASM cannot affect ZK.      │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 4 — Deterministic Execution Sandbox (sandbox.rs)             │
│                                                                     │
│  • Bytecode validation before deployment                            │
│  • Memory limits (max 256 WASM pages = 16MB)                       │
│  • Call stack depth limit (1024 frames)                             │
│  • Host API whitelist (storage, events, crypto, BLEEP Connect)      │
│  • No filesystem, no network, no random — deterministic execution   │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 5 — State Transition (state_transition.rs)                   │
│                                                                     │
│  VM NEVER writes state directly.                                    │
│                                                                     │
│  Execution Result → StateDiff → bleep-state (atomic commit)        │
│                                                                     │
│  StateDiff { storage_updates, balance_updates, events, code }      │
│  + commitment_hash() for validator signatures                       │
│  + simulate() for dry-run without commitment                        │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 6 — Unified Gas Model (gas_model.rs)                         │
│                                                                     │
│  All VMs emit native gas. GasModel normalises to BLEEP gas.        │
│                                                                     │
│  EVM:  1 BLEEP gas = 1.0 EVM gas (baseline)                        │
│  WASM: 1 BLEEP gas = 2.5 WASM gas (instructions cheaper)           │
│  SBF:  1 BLEEP gas = 3.0 SBF  gas (Solana very cheap)              │
│  Move: 1 BLEEP gas = 2.0 Move gas                                  │
│  ZK:   1 BLEEP gas = 0.1 ZK   gas (proofs expensive)               │
│                                                                     │
│  Without this, attackers exploit cheaper VMs for DoS.              │
├─────────────────────────────────────────────────────────────────────┤
│  Layer 7 — Cross-Chain Native Execution (native_bridge.rs)          │
│                                                                     │
│  Contracts call other chains natively:                              │
│  bleep_call(chain="ethereum", contract=0xABC, data=...)            │
│                                                                     │
│  Supported: Ethereum, Solana, Cosmos, Polkadot, Near, Sui, Aptos   │
│  Each chain uses its native ABI encoding automatically.             │
│  Routes through BLEEP Connect (Kyber-768 KEM + SPHINCS+ signatures)│
└─────────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
crates/bleep-vm/
├── src/
│   ├── lib.rs                        # Public API + integration tests
│   ├── intent.rs                     # Layer 1: Intent types + builders
│   ├── types.rs                      # Shared types (ChainId, Gas, etc.)
│   ├── error.rs                      # Error hierarchy
│   │
│   ├── router/
│   │   └── vm_router.rs              # Layer 2: VmRouter + Engine trait
│   │
│   ├── engines/
│   │   ├── evm_engine.rs             # Layer 3: revm EVM (100% compatible)
│   │   ├── wasm_engine_adapter.rs    # Layer 3: Wasmer WASM
│   │   ├── zk_engine_adapter.rs      # Layer 3: Groth16 ZK verifier
│   │   ├── wasm_engine.rs            # Full WASM runtime (v4)
│   │   └── zk_engine.rs              # ZK proof utilities (v4)
│   │
│   ├── runtime/
│   │   ├── gas_model.rs              # Layer 6: Unified gas normalisation
│   │   ├── sandbox.rs                # Layer 4: Bytecode validation
│   │   └── memory.rs                 # Memory pool + limits
│   │
│   ├── execution/
│   │   ├── executor.rs               # Top-level orchestrator (all 7 layers)
│   │   ├── execution_context.rs      # Block/tx env + gas accounting state
│   │   ├── call_stack.rs             # Layer 4: Call depth + frame tracking
│   │   └── state_transition.rs       # Layer 5: StateDiff + StateTransition
│   │
│   └── crosschain/
│       ├── native_bridge.rs          # Layer 7: Cross-chain execution
│       └── connect_bridge.rs         # BLEEP Connect ABI bridges (v4)
```

## Quick Start

### Transfer

```rust
use bleep_vm::{Executor, ExecutorConfig, Intent, IntentKind, TransferIntent};
use bleep_vm::types::ChainId;

let executor = Executor::production(ExecutorConfig::default());

let intent = Intent::new_unsigned(
    IntentKind::Transfer(TransferIntent {
        from:   [1u8; 32],
        to:     [2u8; 32],
        amount: 1_000_000,
        memo:   Some("payment".into()),
    }),
    ChainId::Bleep,
);

let outcome = executor.execute(&intent).await?;
println!("Gas used: {} BLEEP gas", outcome.bleep_gas);
println!("State diff: {:?}", outcome.state_diff());
```

### EVM Contract Call (Solidity)

```rust
use bleep_vm::{ContractCallBuilder, TargetVm};

let intent = Intent::new_unsigned(
    ContractCallBuilder::new([0xABu8; 32])  // contract address
        .vm(TargetVm::Evm)
        .calldata(abi_encode("transfer", &[recipient, amount]))
        .gas(300_000)
        .value(0)
        .build(),
    ChainId::Ethereum,
);

let outcome = executor.execute(&intent).await?;
```

### Deploy WASM Contract

```rust
use bleep_vm::{DeployBuilder, TargetVm};

let bytecode = std::fs::read("contract.wasm")?;

let intent = Intent::new_unsigned(
    DeployBuilder::new(bytecode)
        .vm(TargetVm::Wasm)
        .gas(2_000_000)
        .salt([0x42u8; 32])  // deterministic address
        .build(),
    ChainId::Bleep,
);

let outcome = executor.execute(&intent).await?;
let contract_address = outcome.output(); // 32-byte address
```

### Native Cross-Chain Call

```rust
let intent = Intent::new_unsigned(
    IntentKind::CrossChain(CrossChainIntent {
        destination_chain: ChainId::Ethereum,
        contract:          eth_contract_address.to_vec(),
        calldata:          abi_encode("swap", &[token_in, amount_in, token_out]),
        source_gas_limit:  100_000,
        dest_gas_limit:    300_000,
        bridge_value:      0,
        relay_fee:         1_000_000,   // pay BLEEP Connect relayers
        require_zk_proof:  false,
    }),
    ChainId::Bleep,
);

let outcome = executor.execute(&intent).await?;
let message_id = outcome.output(); // track on BLEEP Connect
```

### ZK Proof Verification

```rust
let intent = Intent::new_unsigned(
    IntentKind::ZkVerify(ZkVerifyIntent {
        proof_bytes:      groth16_proof_bytes,
        public_inputs:    vec![state_root_before, state_root_after],
        vk_id:            "rollup-v1".into(),
        post_verify_wasm: Some(settlement_wasm),
    }),
    ChainId::Bleep,
);
```

## Adding a New Engine

```rust
use bleep_vm::router::vm_router::{Engine, EngineResult};

struct CairoEngine;

#[async_trait]
impl Engine for CairoEngine {
    fn name(&self) -> &'static str { "cairo-vm" }
    fn supports(&self, vm: &TargetVm) -> bool {
        matches!(vm, TargetVm::Cairo) // add Cairo to TargetVm enum
    }
    async fn execute(&self, ctx, bytecode, calldata, gas) -> VmResult<EngineResult> {
        // ... Cairo execution ...
    }
    async fn deploy(&self, ...) -> VmResult<EngineResult> { ... }
}

// Register:
executor.router.register_engine(Arc::new(CairoEngine));
```

## Security Properties

| Property | Mechanism |
|---|---|
| VM Isolation | Each engine is a separate type; no shared mutable state |
| Deterministic execution | No system calls, no randomness, same bytecode = same result |
| Clean state management | VM outputs `StateDiff` only; bleep-state does atomic commit |
| Cross-chain security | BLEEP Connect: Kyber-768 KEM + SPHINCS+ + ZK proofs |
| DoS prevention | Unified gas model prevents cheap-VM exploitation |
| Upgradability | New engines registered dynamically; no fork required |
| Signature verification | Ed25519 on every intent (skippable for trusted internal calls) |
| Bytecode validation | Sandbox validator runs before any deployment |

## What BLEEP VM Supports

| Feature | Status |
|---|---|
| Solidity / Vyper contracts (EVM) | ✅ revm — 100% compatible |
| Rust WASM contracts | ✅ Wasmer 4.2 + Cranelift |
| ZK contracts (Groth16) | ✅ ark-groth16 on BN254 |
| Cross-chain native calls | ✅ BLEEP Connect (7 chains) |
| Intent execution | ✅ All 5 intent types |
| Move VM (Sui/Aptos) | 🔜 v0.6 |
| Cairo VM (StarkNet) | 🔜 v0.7 |
| AI VM | 🔜 Future |
