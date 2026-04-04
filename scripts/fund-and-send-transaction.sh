#!/usr/bin/env bash
set -euo pipefail

BIN="./target/release/bleep-cli"
RPC="${BLEEP_RPC:-http://127.0.0.1:8545}"

function usage() {
  cat <<EOF
Usage: $0 --to <BLEEP_ADDRESS> --amount <BLEEP_AMOUNT>

Environment:
  BLEEP_RPC   RPC endpoint (default: http://127.0.0.1:8545)

Example:
  $0 --to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8 --amount 100
EOF
  exit 1
}

# Check binary exists
if [[ ! -x "$BIN" ]]; then
  echo "Error: $BIN not found. Run: cargo build --release --bin bleep-cli"
  exit 1
fi

# Parse args
TO=""
AMOUNT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --to)     TO="$2";     shift 2 ;;
    --amount) AMOUNT="$2"; shift 2 ;;
    *)        usage ;;
  esac
done

if [[ -z "$TO" || -z "$AMOUNT" ]]; then
  usage
fi

# Check RPC is up
if ! curl -sS --fail --connect-timeout 5 "$RPC/rpc/health" >/dev/null; then
  echo "Error: RPC not reachable at $RPC"
  echo "Start node with: cargo run --release"
  exit 1
fi
echo "✅ Node RPC is reachable."

# Get current active wallet address from balance output
# Format: "Address: BLEEP1xxx  Balance: N BLEEP  Nonce: N  Root: ..."
ADDRESS=$("$BIN" wallet balance 2>/dev/null | grep "^Address:" | head -n 1 | awk '{print $2}')

if [[ -z "$ADDRESS" ]]; then
  echo "No wallet found. Creating one..."
  "$BIN" wallet create
  ADDRESS=$("$BIN" wallet balance 2>/dev/null | grep "^Address:" | head -n 1 | awk '{print $2}')
fi

if [[ -z "$ADDRESS" ]]; then
  echo "Error: Could not determine wallet address after creation."
  exit 1
fi
echo "Active wallet: $ADDRESS"

# Check balance before attempting faucet or send
BALANCE=$("$BIN" wallet balance 2>/dev/null | grep "^Address: $ADDRESS" | awk '{print $4}')
echo "Current balance: ${BALANCE:-0} BLEEP"

# Only hit faucet if balance is 0
if [[ "${BALANCE:-0}" == "0" ]]; then
  echo "Balance is zero. Requesting faucet funds for $ADDRESS..."
  FAUCET_RESP=$(curl -sS -X POST "$RPC/faucet/$ADDRESS")

  if echo "$FAUCET_RESP" | grep -q '"error"'; then
    echo "⚠️  Faucet error: $FAUCET_RESP"
    echo "You may be on cooldown. Wait or fund manually."
    exit 1
  else
    echo "✅ Faucet response: $FAUCET_RESP"
  fi

  echo "Updated balance:"
  "$BIN" wallet balance
else
  echo "✅ Wallet already has funds, skipping faucet."
fi

# Send the transaction
# Usage: bleep-cli tx send <TO> <AMOUNT>  (positional, no flags)
echo ""
echo "Sending $AMOUNT BLEEP → $TO ..."
RESULT=$("$BIN" tx send "$TO" "$AMOUNT")
echo "$RESULT"

# Check result
if echo "$RESULT" | grep -q "validation_failed\|rejected"; then
  echo ""
  echo "❌ Transaction failed validation."
  echo "   Make sure the active wallet has sufficient balance."
  echo "   Run: $BIN wallet balance"
  exit 1
elif echo "$RESULT" | grep -q "submitted\|tx_id"; then
  echo ""
  echo "✅ Transaction submitted successfully!"
else
  echo ""
  echo "⚠️  Unexpected response. Check node logs."
fi
