-- MonBeat simulation history table
-- Stores simulation results for the /api/simulations endpoint.
-- source_hash enables dedup and cache-key correlation with Redis.

CREATE TABLE IF NOT EXISTS simulations (
    id          TEXT PRIMARY KEY,
    source_hash TEXT NOT NULL,
    response    JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for pagination queries (ORDER BY created_at DESC)
CREATE INDEX IF NOT EXISTS idx_simulations_created_at ON simulations (created_at DESC);

-- Index for cache-key lookups by source hash
CREATE INDEX IF NOT EXISTS idx_simulations_source_hash ON simulations (source_hash);
