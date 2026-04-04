#!/usr/bin/env bash
set -euo pipefail

BIN="./target/release/bleep-cli"

function build_cli() {
  echo "Building bleep-cli release binary..."
  cargo build --release --bin bleep-cli
  echo "Built $BIN"
}

if [[ ! -x "$BIN" ]]; then
  build_cli
fi

echo "Creating a new wallet..."
"$BIN" wallet create

echo
if [[ -n "$(command -v jq 2>/dev/null)" ]]; then
  echo "Wallet health check:"
  curl -sS http://127.0.0.1:8545/rpc/health | jq .
else
  echo "Wallet created. If you want to validate the local node, run:"
  echo "  curl -sS http://127.0.0.1:8545/rpc/health"
fi

echo
echo "Your wallet addresses:"
"$BIN" wallet export

echo
echo "Check your balance with: $BIN wallet balance"
