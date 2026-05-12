#!/usr/bin/env bash
# Build and deploy `stargaze_anchor` to Solana devnet.
#
# Required env (or `.env`):
#   ANCHOR_WALLET   — path to the deployer keypair (default ~/.config/solana/id.json)
#   ANCHOR_PROVIDER — defaults to "devnet"
#
# Generates a fresh program keypair if `target/deploy/stargaze_anchor-keypair.json`
# is missing, otherwise re-uses the existing one (preserves the program id).

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f .env ]; then
  # shellcheck source=/dev/null
  set -a; source .env; set +a
fi

WALLET="${ANCHOR_WALLET:-$HOME/.config/solana/id.json}"
CLUSTER="${ANCHOR_PROVIDER:-devnet}"

if [ ! -f "$WALLET" ]; then
  echo "deployer wallet missing at $WALLET — run \`solana-keygen new -o $WALLET\` first" >&2
  exit 1
fi

KEYPAIR="target/deploy/stargaze_anchor-keypair.json"
if [ ! -f "$KEYPAIR" ]; then
  echo "generating fresh program keypair at $KEYPAIR"
  mkdir -p target/deploy
  solana-keygen new --no-bip39-passphrase --silent --force --outfile "$KEYPAIR"
fi

PROGRAM_ID="$(solana-keygen pubkey "$KEYPAIR")"
echo "program id: $PROGRAM_ID"

anchor build
anchor deploy \
  --provider.cluster "$CLUSTER" \
  --provider.wallet "$WALLET" \
  --program-name stargaze_anchor \
  --program-keypair "$KEYPAIR"

echo ""
echo "Deployed. Update packages/shared/src/solana/programs.ts:"
echo "  '$CLUSTER': { StargazeAnchor: '$PROGRAM_ID' }"
