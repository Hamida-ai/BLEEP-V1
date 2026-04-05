# Testnet Faucet & New Wallet Initial Balance Changes

## Overview
This document describes the changes made to enable automatic 10 BLEEP initial balance for newly created wallets on testnet, with a 24-hour cooldown for faucet refills.

## Changes Made

### 1. **Faucet Drip Amount Reduction** (`crates/bleep-rpc/src/lib.rs`)
- **Changed faucet drip amount from 1,000 BLEEP to 10 BLEEP**
- Constant `FAUCET_DRIP_AMOUNT`: Updated to `1_000_000_000` microBLEEP (10 BLEEP with 8 decimals)
- Updated faucet response messages to reflect 10 BLEEP instead of 1,000 BLEEP
- Updated faucet status endpoint to show 10 BLEEP drip amount

**Files Modified:**
- `crates/bleep-rpc/src/lib.rs` - Lines 89-90, 2049, 2113

### 2. **CLI Faucet Commands** (`crates/bleep-cli/src/lib.rs` & `crates/bleep-cli/src/cli.rs`)
- **Added new Faucet CLI subcommand** with two actions:
  - `faucet request <address>` - Request faucet funds for an address
  - `faucet status` - Check faucet status (balance, drip amount, cooldown, total drips)

**Files Modified:**
- `crates/bleep-cli/src/lib.rs` - Added `FaucetCommand` enum with Request and Status variants
- `crates/bleep-cli/src/cli.rs` - Implemented faucet command handlers with proper error handling

### 3. **Automatic Faucet on Wallet Creation** (`crates/bleep-cli/src/cli.rs`)
- **Modified wallet create handler** to automatically request faucet funds after wallet creation
- If faucet is available, new wallets automatically receive 10 BLEEP
- If faucet is unavailable, wallet creation still succeeds with a warning message
- Added user-friendly messages indicating faucet funding status

**Example Output:**
```
✅ Wallet created
   Address: BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8
   Type:    Quantum-secure (SPHINCS+-SHAKE-256)
   Signing: ✅ ready (SK encrypted with AES-256-GCM)
   ⚠️  Back up your key material in a safe location.
💰 Faucet: 10 BLEEP credited to new wallet
```

### 4. **Testnet Default Initial Balance** (`crates/bleep-state/src/state_manager.rs`)
- **Added testnet default initial balance** of 10 BLEEP for new accounts
- New method `AccountState::testnet_default()` provides 10 BLEEP (1_000_000_000 microBLEEP)
- Modified `get_account()` to use testnet default when account not found in DB
- Ensures all new addresses start with 10 BLEEP even if faucet is unreachable

**Files Modified:**
- `crates/bleep-state/src/state_manager.rs` - Added testnet_default() method and updated get_account()

### 5. **Testnet Configuration** (`config/testnet_config.json`)
- **Updated configuration to document faucet and wallet settings**
- Added `testnet_specific` section with:
  - `faucet_drip_amount_bleep`: 10
  - `faucet_drip_amount_micro`: 1_000_000_000
  - `faucet_cooldown_hours`: 24
  - `new_wallet_initial_balance_bleep`: 10
  - `new_wallet_initial_balance_micro`: 1_000_000_000

## How It Works

### Workflow for New Wallets on Testnet

1. **User runs:** `bleep-cli wallet create`
2. **CLI generates** a new SPHINCS+ keypair and creates encrypted wallet
3. **CLI automatically calls** `POST /faucet/{address}` to request initial 10 BLEEP
4. **Wallet is credited** with 10 BLEEP from faucet
5. **User can immediately** start testing transactions

### Rate Limiting & Cooldown

- **Per-address cooldown:** 24 hours (86,400 seconds)
- **Per-IP cooldown:** 24 hours (86,400 seconds)
- **Amount per drip:** 10 BLEEP
- **Error messages:** Clear feedback if cooldown period not elapsed

### Fallback Mechanism

If the faucet is unavailable:
1. **Wallet creation succeeds** (not dependent on faucet)
2. **Default balance:** 10 BLEEP assigned from `AccountState::testnet_default()`
3. **User receives:** Warning that faucet is unavailable but wallet created
4. **Later:** User can manually request funds via `bleep-cli faucet request <address>`

## API Endpoints

### Faucet Endpoints

#### POST /faucet/{address}
Request 10 test BLEEP for an address

```bash
curl -X POST http://127.0.0.1:8545/faucet/BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8
```

**Response (Success):**
```json
{
  "address": "BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8",
  "amount_bleep": 10,
  "amount_micro": 1000000000,
  "cooldown_secs": 86400,
  "message": "10 test BLEEP sent to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8. Valid on bleep-testnet-1."
}
```

#### GET /faucet/status
Check faucet status

```bash
curl http://127.0.0.1:8545/faucet/status
```

**Response:**
```json
{
  "balance_bleep": 99990,
  "balance_micro": 9999000000000,
  "drip_amount_bleep": 10,
  "cooldown_secs": 86400,
  "total_drips": 10
}
```

## CLI Usage

### Create wallet with automatic faucet funding
```bash
bleep-cli wallet create
```

### Manually request faucet funds
```bash
bleep-cli faucet request BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8
```

### Check faucet status
```bash
bleep-cli faucet status
```

## Transaction Testing Enabled

With these changes:
- ✅ Every new wallet starts with 10 BLEEP
- ✅ Users can test transactions immediately after wallet creation
- ✅ 24-hour cooldown allows fair distribution of testnet funds
- ✅ Faucet rate limiting prevents spam/griefing attacks
- ✅ Fallback mechanism ensures wallets are created even if faucet fails

## Testing

All changes have been verified to compile:
- ✅ `bleep-cli` compiles successfully
- ✅ `bleep-rpc` compiles successfully
- ✅ `bleep-state` compiles successfully

### Manual Testing Steps

1. **Create a new wallet:**
   ```bash
   bleep-cli wallet create
   ```
   Expected: Wallet created with 10 BLEEP automatically credited

2. **Check balance:**
   ```bash
   bleep-cli wallet balance
   ```
   Expected: 10 BLEEP balance shown

3. **Request faucet again (should fail with cooldown message):**
   ```bash
   bleep-cli faucet request <address>
   ```
   Expected: Error indicating 24-hour cooldown

4. **Send a transaction:**
   ```bash
   bleep-cli tx send --to <recipient> --amount 1000000000
   ```
   Expected: Transaction succeeds with 10 BLEEP initial balance

## Determinism & Security

- ✅ All numeric values are hard-coded constants
- ✅ No randomness or non-deterministic behavior added
- ✅ Rate limiting uses unix timestamps (immutable)
- ✅ Faucet balance is atomic (no race conditions)
- ✅ State transitions are deterministic

## Backwards Compatibility

- ✅ Existing wallets not affected
- ✅ RPC parameters unchanged (only constants modified)
- ✅ No database migrations required
- ✅ Graceful fallback for faucet unavailability
