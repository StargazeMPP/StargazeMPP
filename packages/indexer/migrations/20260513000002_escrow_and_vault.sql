-- Escrow + vault registry + verifier event projection. Mirrors the
-- conventions in `20260513000000_init.sql`: one table per Anchor event
-- variant, keyed by (slot, signature) for replay idempotency, with a
-- domain index where one applies (provider_id for vault rows, session_id
-- for escrow rows).

-- ============ ESCROW ============

CREATE TABLE IF NOT EXISTS escrow_initialized (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    admin           BYTEA NOT NULL,
    usdc_mint       BYTEA NOT NULL,
    router          BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);

CREATE TABLE IF NOT EXISTS session_opened (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    session_id      BYTEA NOT NULL,
    agent_wallet    BYTEA NOT NULL,
    deposit         BIGINT NOT NULL,
    spending_limit  BIGINT NOT NULL,
    expires_at      BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_session_opened_session_id ON session_opened (session_id);

CREATE TABLE IF NOT EXISTS voucher_settled (
    id                  BIGSERIAL PRIMARY KEY,
    slot                BIGINT NOT NULL,
    signature           TEXT NOT NULL DEFAULT '',
    session_id          BYTEA NOT NULL,
    provider_id         BYTEA NOT NULL,
    cumulative_amount   BIGINT NOT NULL,
    delta               BIGINT NOT NULL,
    to_provider         BIGINT NOT NULL,
    fee                 BIGINT NOT NULL,
    nonce               BIGINT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_voucher_settled_session_id ON voucher_settled (session_id);
CREATE INDEX IF NOT EXISTS idx_voucher_settled_provider_id ON voucher_settled (provider_id);

CREATE TABLE IF NOT EXISTS session_settled (
    id                  BIGSERIAL PRIMARY KEY,
    slot                BIGINT NOT NULL,
    signature           TEXT NOT NULL DEFAULT '',
    session_id          BYTEA NOT NULL,
    total_to_providers  BIGINT NOT NULL,
    routing_fee         BIGINT NOT NULL,
    refund_to_agent     BIGINT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_session_settled_session_id ON session_settled (session_id);

-- ============ VAULT REGISTRY ============

CREATE TABLE IF NOT EXISTS vault_configured (
    id                  BIGSERIAL PRIMARY KEY,
    slot                BIGINT NOT NULL,
    signature           TEXT NOT NULL DEFAULT '',
    provider_id         BYTEA NOT NULL,
    tier                SMALLINT NOT NULL,
    on_chain_verifier   BYTEA NOT NULL,
    arweave_cid         BYTEA NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_vault_configured_provider_id ON vault_configured (provider_id);

CREATE TABLE IF NOT EXISTS vault_auditor_key_set (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    previous        BYTEA NOT NULL,
    current_key     BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_vault_auditor_key_set_provider_id
    ON vault_auditor_key_set (provider_id);

CREATE TABLE IF NOT EXISTS vault_buyer_key_rotation_updated (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    cid             BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_vault_buyer_key_rotation_updated_provider_id
    ON vault_buyer_key_rotation_updated (provider_id);

CREATE TABLE IF NOT EXISTS vault_deactivated (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_vault_deactivated_provider_id ON vault_deactivated (provider_id);

-- ============ VERIFIER ============

CREATE TABLE IF NOT EXISTS vault_proof_verified (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    tier            SMALLINT NOT NULL,
    signals_hash    BYTEA NOT NULL,
    submitter       BYTEA NOT NULL,
    on_chain_slot   BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_vault_proof_verified_provider_id
    ON vault_proof_verified (provider_id);
CREATE INDEX IF NOT EXISTS idx_vault_proof_verified_signals_hash
    ON vault_proof_verified (signals_hash);

-- ============ REPUTATION (solana-only successor) ============

CREATE TABLE IF NOT EXISTS reputation_score_set (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    score           INTEGER NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_reputation_score_set_provider_id
    ON reputation_score_set (provider_id);
