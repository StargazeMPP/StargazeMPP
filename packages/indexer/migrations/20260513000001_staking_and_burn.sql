-- Staking-pool and burn-ladder event projection. Mirrors the conventions in
-- `20260513000000_init.sql`: one table per Anchor event variant, keyed by
-- (slot, signature) for replay idempotency, with a `provider_id` index on
-- every per-provider table. Global config-style events (StakingInitialized,
-- StakeMintSet, RoutingFeeProcessed) don't carry a provider_id and so get
-- no provider_id index.

CREATE TABLE IF NOT EXISTS staked (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    owner           BYTEA NOT NULL,
    amount          BIGINT NOT NULL,
    total           BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_staked_provider_id ON staked (provider_id);

CREATE TABLE IF NOT EXISTS unstake_requested (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    owner           BYTEA NOT NULL,
    amount          BIGINT NOT NULL,
    cooldown_until  BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_unstake_requested_provider_id ON unstake_requested (provider_id);

CREATE TABLE IF NOT EXISTS unstaked (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    owner           BYTEA NOT NULL,
    amount          BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_unstaked_provider_id ON unstaked (provider_id);

CREATE TABLE IF NOT EXISTS slashed (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    provider_id     BYTEA NOT NULL,
    owner           BYTEA NOT NULL,
    amount          BIGINT NOT NULL,
    destination     BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_slashed_provider_id ON slashed (provider_id);

CREATE TABLE IF NOT EXISTS staking_initialized (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    stake_mint      BYTEA NOT NULL,
    min_stake       BIGINT NOT NULL,
    verified_stake  BIGINT NOT NULL,
    cooldown_secs   BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);

CREATE TABLE IF NOT EXISTS stake_mint_set (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    stake_mint      BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);

CREATE TABLE IF NOT EXISTS routing_fee_processed (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    burned          BIGINT NOT NULL,
    to_stakers      BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);

CREATE TABLE IF NOT EXISTS reputation_vote_burned (
    id              BIGSERIAL PRIMARY KEY,
    slot            BIGINT NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    voter           BYTEA NOT NULL,
    provider_id     BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_reputation_vote_burned_provider_id ON reputation_vote_burned (provider_id);
CREATE INDEX IF NOT EXISTS idx_reputation_vote_burned_voter ON reputation_vote_burned (voter);

CREATE TABLE IF NOT EXISTS stake_dispatched (
    id                   BIGSERIAL PRIMARY KEY,
    slot                 BIGINT NOT NULL,
    signature            TEXT NOT NULL DEFAULT '',
    provider_id          BYTEA NOT NULL,
    owner                BYTEA NOT NULL,
    amount               BIGINT NOT NULL,
    dest_chain_selector  BIGINT NOT NULL,
    receiver             BYTEA NOT NULL,
    payload              BYTEA NOT NULL,
    extra_args           BYTEA NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slot, signature)
);
CREATE INDEX IF NOT EXISTS idx_stake_dispatched_provider_id ON stake_dispatched (provider_id);
