-- Event store table
CREATE TABLE IF NOT EXISTS events (
    event_id UUID PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_version INT NOT NULL DEFAULT 1,
    payload JSONB NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    version BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure version uniqueness per aggregate (optimistic locking)
    UNIQUE(aggregate_id, version)
);

-- Index for aggregate queries (most common query pattern)
CREATE INDEX idx_events_aggregate ON events(aggregate_id, version);

-- Index for event type queries (useful for projections)
CREATE INDEX idx_events_type ON events(event_type);

-- Index for timestamp queries (useful for temporal queries)
CREATE INDEX idx_events_created ON events(created_at);

-- GIN index for JSONB queries on payload
CREATE INDEX idx_events_payload ON events USING GIN (payload);

-- GIN index for JSONB queries on metadata
CREATE INDEX idx_events_metadata ON events USING GIN (metadata);

-- Comments for documentation
COMMENT ON TABLE events IS 'Event store table for CQRS/Event Sourcing - stores all domain events';
COMMENT ON COLUMN events.event_id IS 'Unique identifier for the event';
COMMENT ON COLUMN events.aggregate_id IS 'Identifier of the aggregate that produced this event';
COMMENT ON COLUMN events.aggregate_type IS 'Type of aggregate (e.g., Order, Payment)';
COMMENT ON COLUMN events.event_type IS 'Type of event (e.g., OrderCreated, OrderCancelled)';
COMMENT ON COLUMN events.event_version IS 'Schema version of the event for handling event evolution';
COMMENT ON COLUMN events.payload IS 'Event data in JSON format';
COMMENT ON COLUMN events.metadata IS 'Event metadata (correlation_id, causation_id, user_id, etc.)';
COMMENT ON COLUMN events.version IS 'Aggregate version - used for optimistic concurrency control';
COMMENT ON COLUMN events.created_at IS 'Timestamp when the event was created';
