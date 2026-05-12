#!/usr/bin/env bash
# Broadcast `Deploy.s.sol` to Tempo testnet.
#
# Required env (load via `.env` or your shell):
#   DEPLOYER_PRIVATE_KEY   — hex-encoded deployer EOA key (no 0x prefix)
#   ADMIN_MULTISIG         — 4-of-7 Safe multisig that becomes day-one admin
#   PATHUSD_ADDRESS        — PathUSD ERC-20 deployed on the target network
#   TEMPO_TESTNET_RPC      — RPC endpoint, e.g. https://rpc.testnet.tempo.xyz
#
# Optional:
#   GAZE_INITIAL_SUPPLY    — defaults to 1e9 * 1e18 (see Deploy.s.sol)
#   TEMPO_BROADCAST_FLAGS  — extra forge flags (e.g. --verify --etherscan-api-key …)
#
# Dry run (no broadcast): set DRY_RUN=1.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f .env ]; then
  # shellcheck source=/dev/null
  set -a; source .env; set +a
fi

: "${DEPLOYER_PRIVATE_KEY:?missing}"
: "${ADMIN_MULTISIG:?missing}"
: "${PATHUSD_ADDRESS:?missing}"
: "${TEMPO_TESTNET_RPC:?missing}"

EXTRA_FLAGS="${TEMPO_BROADCAST_FLAGS:-}"
if [ "${DRY_RUN:-0}" = "1" ]; then
  echo "DRY_RUN=1 — simulating only, not broadcasting"
  BROADCAST_FLAG=""
else
  BROADCAST_FLAG="--broadcast"
fi

# shellcheck disable=SC2086
forge script script/Deploy.s.sol \
  --rpc-url "$TEMPO_TESTNET_RPC" \
  --private-key "$DEPLOYER_PRIVATE_KEY" \
  $BROADCAST_FLAG \
  $EXTRA_FLAGS
