# `packages/anchor-program` — `StargazeAnchor` Solana program

**Owner:** this team.

Solana-side mirror of `StargazeRegistry` for Solana-native providers. Bridges to Tempo via Chainlink CCIP for unified reputation + provider state across chains.

Pinocchio vs Anchor: default Anchor 0.30+, evaluate Pinocchio for the hot paths (voucher settle ix, registry mirror sync) if compute budget gets tight.
