-- StargazeAnchor event projection: one table per Anchor event variant.
-- Each row is keyed by (slot, signature) to make replay idempotent — the
-- runtime sink uses `ON CONFLICT (slot, signature) DO NOTHING` so that
-- reprocessing the same Yellowstone update is a no-op.

CREATE TABLE IF NOT EXISTS provider_registered (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    owner           BYTEA NOT NULL,
    category_hash   BYTEA NOT NULL,
    meta_cid        BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_provider_registered_provider_id ON provider_registered (provider_id);

CREATE TABLE IF NOT EXISTS reputation_voted (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    voter           BYTEA NOT NULL,
    accurate        BOOL NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_reputation_voted_provider_id ON reputation_voted (provider_id);

CREATE TABLE IF NOT EXISTS x402_receipt_recorded (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    session_id      BYTEA NOT NULL,
    provider_id     BYTEA NOT NULL,
    payer           BYTEA NOT NULL,
    amount          BIGINT NOT NULL,
    paid_at         BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_x402_receipt_recorded_provider_id ON x402_receipt_recorded (provider_id);

CREATE TABLE IF NOT EXISTS reputation_mirrored (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    score           INTEGER NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_reputation_mirrored_provider_id ON reputation_mirrored (provider_id);

CREATE TABLE IF NOT EXISTS ccip_dispatched (
    id                   BIGSERIAL PRIMARY KEY,
    slot                 BIGINT NOT NULL,
    signature            TEXT NOT NULL DEFAULT '',
    provider_id          BYTEA NOT NULL,
    score                INTEGER NOT NULL,
    dest_chain_selector  BIGINT NOT NULL,
    receiver             BYTEA NOT NULL,
    payload              BYTEA NOT NULL,
    extra_args           BYTEA NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_ccip_dispatched_provider_id ON ccip_dispatched (provider_id);
