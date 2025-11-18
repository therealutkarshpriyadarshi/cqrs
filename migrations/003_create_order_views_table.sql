-- Read model table for order queries
CREATE TABLE IF NOT EXISTS order_views (
    order_id UUID PRIMARY KEY,
    customer_id UUID NOT NULL,
    order_number VARCHAR(50) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL,
    total_amount DECIMAL(12,2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    items JSONB NOT NULL,
    shipping_address JSONB,
    tracking_number VARCHAR(100),
    carrier VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL DEFAULT 1
);

-- Index for customer queries (most common query pattern)
CREATE INDEX idx_order_views_customer ON order_views(customer_id, created_at DESC);

-- Index for status queries
CREATE INDEX idx_order_views_status ON order_views(status, created_at DESC);

-- Index for order number lookups
CREATE INDEX idx_order_views_order_number ON order_views(order_number);

-- Index for created_at ordering
CREATE INDEX idx_order_views_created ON order_views(created_at DESC);

-- GIN index for JSONB items queries
CREATE INDEX idx_order_views_items ON order_views USING GIN (items);

-- Comments for documentation
COMMENT ON TABLE order_views IS 'Materialized view of orders optimized for queries (CQRS read model)';
COMMENT ON COLUMN order_views.order_id IS 'Unique identifier for the order (matches aggregate_id in events table)';
COMMENT ON COLUMN order_views.customer_id IS 'Customer who placed the order';
COMMENT ON COLUMN order_views.order_number IS 'Human-readable order number';
COMMENT ON COLUMN order_views.status IS 'Current order status (CREATED, CONFIRMED, CANCELLED, SHIPPED, DELIVERED)';
COMMENT ON COLUMN order_views.total_amount IS 'Total order amount';
COMMENT ON COLUMN order_views.currency IS 'Currency code (ISO 4217)';
COMMENT ON COLUMN order_views.items IS 'Order line items in JSON format';
COMMENT ON COLUMN order_views.shipping_address IS 'Shipping address in JSON format';
COMMENT ON COLUMN order_views.tracking_number IS 'Shipment tracking number';
COMMENT ON COLUMN order_views.carrier IS 'Shipping carrier name';
COMMENT ON COLUMN order_views.created_at IS 'Timestamp when order was created';
COMMENT ON COLUMN order_views.updated_at IS 'Timestamp when order was last updated';
COMMENT ON COLUMN order_views.version IS 'Version number for optimistic locking in read model';
