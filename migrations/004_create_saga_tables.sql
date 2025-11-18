-- Saga instances table for tracking saga state
CREATE TABLE IF NOT EXISTS saga_instances (
    saga_id UUID PRIMARY KEY,
    saga_type VARCHAR(100) NOT NULL,
    current_step INT NOT NULL DEFAULT 0,
    state JSONB NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying sagas by status
CREATE INDEX idx_saga_status ON saga_instances(status);

-- Index for querying sagas by type
CREATE INDEX idx_saga_type ON saga_instances(saga_type);

-- Index for querying sagas by created time
CREATE INDEX idx_saga_created ON saga_instances(created_at);

-- Index for querying sagas by updated time (for finding stuck sagas)
CREATE INDEX idx_saga_updated ON saga_instances(updated_at);

-- Composite index for status + created_at (common query pattern)
CREATE INDEX idx_saga_status_created ON saga_instances(status, created_at);

-- Idempotency tracking for saga events
CREATE TABLE IF NOT EXISTS saga_event_log (
    event_id UUID PRIMARY KEY,
    saga_id UUID NOT NULL REFERENCES saga_instances(saga_id) ON DELETE CASCADE,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying events by saga
CREATE INDEX idx_saga_event_saga_id ON saga_event_log(saga_id);

-- Index for querying events by type
CREATE INDEX idx_saga_event_type ON saga_event_log(event_type);

-- Index for checking idempotency
CREATE INDEX idx_saga_event_id ON saga_event_log(event_id);

COMMENT ON TABLE saga_instances IS 'Stores the state of running sagas for distributed transaction coordination';
COMMENT ON TABLE saga_event_log IS 'Tracks all events processed by sagas for idempotency and audit trail';
COMMENT ON COLUMN saga_instances.saga_id IS 'Unique identifier for the saga instance';
COMMENT ON COLUMN saga_instances.saga_type IS 'Type of saga (e.g., OrderProcessingSaga)';
COMMENT ON COLUMN saga_instances.current_step IS 'Current step index in the saga execution';
COMMENT ON COLUMN saga_instances.state IS 'Complete saga state including steps, data, and results';
COMMENT ON COLUMN saga_instances.status IS 'Saga status: RUNNING, COMPLETED, COMPENSATING, COMPENSATED, FAILED';
