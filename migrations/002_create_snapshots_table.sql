-- Snapshots table for performance optimization
CREATE TABLE IF NOT EXISTS snapshots (
    aggregate_id UUID PRIMARY KEY,
    aggregate_type VARCHAR(100) NOT NULL,
    version BIGINT NOT NULL,
    state JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for aggregate type queries
CREATE INDEX idx_snapshots_type ON snapshots(aggregate_type);

-- Index for version queries
CREATE INDEX idx_snapshots_version ON snapshots(version);

-- Comments for documentation
COMMENT ON TABLE snapshots IS 'Aggregate snapshots for performance optimization - reduces event replay time';
COMMENT ON COLUMN snapshots.aggregate_id IS 'Identifier of the aggregate';
COMMENT ON COLUMN snapshots.aggregate_type IS 'Type of aggregate (e.g., Order, Payment)';
COMMENT ON COLUMN snapshots.version IS 'Version of the aggregate at the time of snapshot';
COMMENT ON COLUMN snapshots.state IS 'Complete state of the aggregate in JSON format';
COMMENT ON COLUMN snapshots.created_at IS 'Timestamp when the snapshot was first created';
COMMENT ON COLUMN snapshots.updated_at IS 'Timestamp when the snapshot was last updated';
