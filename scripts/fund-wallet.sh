#!/usr/bin/env bash
set -euo pipefail

BIN="./target/release/bleep-cli"
RPC="${BLEEP_RPC:-http://127.0.0.1:8545}"

function usage() {
  cat <<EOF
Usage: $0 [--address <BLEEP_ADDRESS>]

This script requests 1,000 test BLEEP from the node faucet and verifies the wallet balance.

Environment:
  BLEEP_RPC              RPC endpoint (default: http://127.0.0.1:8545)

Example:
  $0 --address BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8
EOF
  exit 1
}

if [[ ! -x "$BIN" ]]; then
  echo "Error: $BIN not found. Build it first with: cargo build --release --bin bleep-cli"
  exit 1
fi

ADDRESS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --address)
      ADDRESS="$2"
      shift 2
      ;;
    *)
      usage
      ;;
  esac
done

if [[ -z "$ADDRESS" ]]; then
  ADDRESS=$("$BIN" wallet export | head -n 1 | awk '{print $NF}')
  if [[ -z "$ADDRESS" ]]; then
    echo "Error: no wallet address found. Run ./scripts/create-wallet.sh first."
    exit 1
  fi
fi

echo "Requesting faucet drip for wallet: $ADDRESS"
RESPONSE=$(curl -sS -X POST "$RPC/faucet/$ADDRESS")
STATUS=$?

if [[ $STATUS -ne 0 || "$RESPONSE" == "" ]]; then
  echo "Error: faucet request failed. Check node availability at $RPC."
  exit 1
fi

if command -v jq >/dev/null 2>&1; then
  echo "$RESPONSE" | jq .
else
  echo "$RESPONSE"
fi

echo
echo "Wallet balance after faucet drip:"
"$BIN" wallet balance
