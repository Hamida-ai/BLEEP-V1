#!/usr/bin/env bash
set -euo pipefail

BIN="./target/release/bleep-cli"
RPC="${BLEEP_RPC:-http://127.0.0.1:8545}"

function usage() {
  cat <<EOF
Usage: $0 --to <BLEEP_ADDRESS> --amount <BLEEP_AMOUNT>

This script creates a wallet if needed, requests faucet funds, and sends a native BLEEP transfer.

Environment:
  BLEEP_RPC              RPC endpoint (default: http://127.0.0.1:8545)
  BLEEP_WALLET_PASSWORD  Wallet passphrase if your wallet is encrypted

Example:
  BLEEP_WALLET_PASSWORD="correct horse battery staple" \
  $0 --to BLEEP1a3f7b2c9d4e8f1a0b5c6d7e9f2a3b4c5d6e7f8 --amount 1000
EOF
  exit 1
}

if [[ ! -x "$BIN" ]]; then
  echo "Error: $BIN not found. Build it first with: cargo build --release --bin bleep-cli"
  exit 1
fi

TO=""
AMOUNT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --to)
      TO="$2"
      shift 2
      ;;
    --amount)
      AMOUNT="$2"
      shift 2
      ;;
    *)
      usage
      ;;
  esac
done

if [[ -z "$TO" || -z "$AMOUNT" ]]; then
  usage
fi

if ! curl -sS --fail --connect-timeout 5 "$RPC/rpc/health" >/dev/null; then
  echo "Error: RPC endpoint not reachable at $RPC"
  echo "Start the node with: ./target/release/bleep"
  exit 1
fi

if [[ -z "$("$BIN" wallet export | head -n 1)" ]]; then
  echo "No wallet found. Creating a new wallet first..."
  "$BIN" wallet create
fi

ADDRESS=$("$BIN" wallet export | head -n 1 | awk '{print $NF}')
if [[ -z "$ADDRESS" ]]; then
  echo "Error: could not determine wallet address."
  exit 1
fi

echo "Funding wallet $ADDRESS from faucet..."
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

echo "Current balance:"
"$BIN" wallet balance

echo "Sending transaction to $TO for $AMOUNT BLEEP..."
"$BIN" tx send --to "$TO" --amount "$AMOUNT"
