# Rust Implementation Roadmap - CQRS & Event Sourcing

**Complete step-by-step guide to building a production-grade CQRS/ES system in Rust**

---

## 🦀 Why Rust for CQRS/Event Sourcing?

### Advantages
✅ **Zero-cost abstractions** - Performance without compromise
✅ **Memory safety** - No null pointers, no data races (compile-time guaranteed)
✅ **Fearless concurrency** - Safe parallelism with `tokio`
✅ **Type safety** - Rich type system prevents event corruption
✅ **Pattern matching** - Perfect for event handling
✅ **Error handling** - `Result<T, E>` forces explicit error handling

### Challenges
⚠️ **Steep learning curve** - Ownership, borrowing, lifetimes
⚠️ **Longer development time** - 2-3x vs Go initially
⚠️ **Smaller ecosystem** - Fewer CQRS-specific libraries
⚠️ **Compilation time** - Can be slow for large projects

**Estimated Timeline**: 8-12 weeks (vs 4-6 weeks for Go)

---

## 📦 Rust Technology Stack

### Core Dependencies

```toml
[package]
name = "cqrs-order-system"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async Runtime
tokio = { version = "1.35", features = ["full"] }
tokio-stream = "0.1"

# Web Framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "uuid", "chrono", "json"] }
deadpool-postgres = "0.12"

# Kafka
rdkafka = { version = "0.36", features = ["cmake-build"] }

# Redis
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# UUID & Time
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Error Handling
thiserror = "1.0"
anyhow = "1.0"

# Logging & Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.22"
opentelemetry = { version = "0.21", features = ["trace"] }
opentelemetry-jaeger = "0.20"

# Metrics
prometheus = "0.13"

# Config
config = "0.14"
dotenv = "0.15"

# Validation
validator = { version = "0.18", features = ["derive"] }

# Utilities
once_cell = "1.19"
async-trait = "0.1"

[dev-dependencies]
# Testing
mockall = "0.12"
testcontainers = "0.15"
tokio-test = "0.4"
fake = "2.9"
```

---

## 🏗️ Project Structure

```
cqrs-order-system/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── docker-compose.yml
├── Makefile
│
├── crates/                           # Workspace crates
│   ├── domain/                       # Core domain logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── aggregates/
│   │       │   ├── mod.rs
│   │       │   ├── order.rs          # Order aggregate
│   │       │   ├── payment.rs
│   │       │   └── inventory.rs
│   │       ├── events/
│   │       │   ├── mod.rs
│   │       │   ├── order_events.rs
│   │       │   ├── payment_events.rs
│   │       │   └── event_envelope.rs
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   └── order_commands.rs
│   │       ├── value_objects/
│   │       │   ├── mod.rs
│   │       │   ├── money.rs
│   │       │   └── address.rs
│   │       └── errors.rs
│   │
│   ├── event-store/                  # Event persistence
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── postgres_event_store.rs
│   │       ├── event_repository.rs
│   │       ├── snapshot_repository.rs
│   │       └── optimistic_lock.rs
│   │
│   ├── messaging/                    # Kafka integration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── producer.rs
│   │       ├── consumer.rs
│   │       └── schema_registry.rs
│   │
│   ├── read-model/                   # Query side
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── repositories/
│   │       │   ├── mod.rs
│   │       │   └── order_view_repo.rs
│   │       ├── projections/
│   │       │   ├── mod.rs
│   │       │   ├── order_projection.rs
│   │       │   └── customer_summary.rs
│   │       └── cache/
│   │           ├── mod.rs
│   │           └── redis_cache.rs
│   │
│   └── common/                       # Shared utilities
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── config.rs
│           ├── telemetry.rs
│           └── errors.rs
│
├── services/                         # Microservices
│   ├── command-service/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── handlers/
│   │       │   ├── mod.rs
│   │       │   ├── create_order.rs
│   │       │   └── cancel_order.rs
│   │       ├── routes.rs
│   │       └── state.rs
│   │
│   ├── query-service/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── handlers/
│   │       │   ├── mod.rs
│   │       │   ├── get_order.rs
│   │       │   └── list_orders.rs
│   │       ├── routes.rs
│   │       └── state.rs
│   │
│   ├── projection-service/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── event_processor.rs
│   │       └── projectors/
│   │           ├── mod.rs
│   │           └── order_projector.rs
│   │
│   └── saga-orchestrator/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── sagas/
│           │   ├── mod.rs
│           │   └── order_saga.rs
│           └── coordinator.rs
│
├── migrations/                       # SQL migrations
│   ├── 001_create_events_table.sql
│   ├── 002_create_snapshots_table.sql
│   └── 003_create_order_views_table.sql
│
└── tests/                           # Integration tests
    ├── integration/
    └── e2e/
```

---

## 📅 Implementation Roadmap

---

## **Phase 1: Foundation & Setup** (Week 1-2)

### Goals
- Set up Rust workspace
- Configure async runtime (Tokio)
- Set up PostgreSQL event store
- Basic domain types

### Tasks

#### 1.1 Initialize Workspace
```bash
# Create workspace
cargo new --lib cqrs-order-system
cd cqrs-order-system

# Create workspace Cargo.toml
```

**Workspace Cargo.toml**:
```toml
[workspace]
members = [
    "crates/domain",
    "crates/event-store",
    "crates/messaging",
    "crates/read-model",
    "crates/common",
    "services/command-service",
    "services/query-service",
    "services/projection-service",
]

resolver = "2"

[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
```

---

#### 1.2 Domain Layer - Event Types

**`crates/domain/src/events/mod.rs`**:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Base event envelope wrapping all domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub event_type: String,
    pub event_version: i32,
    pub payload: serde_json::Value,
    pub metadata: EventMetadata,
    pub timestamp: DateTime<Utc>,
    pub sequence_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub correlation_id: Uuid,
    pub causation_id: Uuid,
    pub user_id: Option<Uuid>,
}

/// Trait for all domain events
pub trait DomainEvent: Serialize + for<'de> Deserialize<'de> {
    fn event_type() -> &'static str;
    fn event_version() -> i32 {
        1
    }
}
```

---

#### 1.3 Order Events

**`crates/domain/src/events/order_events.rs`**:
```rust
use super::DomainEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreatedEvent {
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub order_number: String,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

impl DomainEvent for OrderCreatedEvent {
    fn event_type() -> &'static str {
        "OrderCreated"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: Uuid,
    pub sku: String,
    pub quantity: u32,
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderConfirmedEvent {
    pub order_id: Uuid,
    pub confirmed_at: DateTime<Utc>,
}

impl DomainEvent for OrderConfirmedEvent {
    fn event_type() -> &'static str {
        "OrderConfirmed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelledEvent {
    pub order_id: Uuid,
    pub reason: String,
    pub cancelled_at: DateTime<Utc>,
}

impl DomainEvent for OrderCancelledEvent {
    fn event_type() -> &'static str {
        "OrderCancelled"
    }
}
```

---

#### 1.4 Order Aggregate

**`crates/domain/src/aggregates/order.rs`**:
```rust
use crate::events::order_events::*;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Created,
    Confirmed,
    Cancelled,
    Shipped,
    Delivered,
}

#[derive(Debug, Clone)]
pub struct OrderAggregate {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub order_number: String,
    pub status: OrderStatus,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub version: i64,
}

impl OrderAggregate {
    /// Create new order aggregate from command
    pub fn create(
        customer_id: Uuid,
        items: Vec<OrderItem>,
    ) -> Result<(Self, OrderCreatedEvent), OrderError> {
        if items.is_empty() {
            return Err(OrderError::NoItems);
        }

        let order_id = Uuid::new_v4();
        let total_amount = items.iter().map(|i| i.unit_price * i.quantity as f64).sum();
        let order_number = format!("ORD-{}", Uuid::new_v4().simple());

        let event = OrderCreatedEvent {
            order_id,
            customer_id,
            order_number: order_number.clone(),
            items: items.clone(),
            total_amount,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        };

        let aggregate = Self {
            id: order_id,
            customer_id,
            order_number,
            status: OrderStatus::Created,
            items,
            total_amount,
            version: 0,
        };

        Ok((aggregate, event))
    }

    /// Apply event to rebuild state
    pub fn apply_event(&mut self, event: &OrderCreatedEvent) {
        self.id = event.order_id;
        self.customer_id = event.customer_id;
        self.order_number = event.order_number.clone();
        self.items = event.items.clone();
        self.total_amount = event.total_amount;
        self.status = OrderStatus::Created;
        self.version += 1;
    }

    /// Confirm order
    pub fn confirm(&self) -> Result<OrderConfirmedEvent, OrderError> {
        match self.status {
            OrderStatus::Created => Ok(OrderConfirmedEvent {
                order_id: self.id,
                confirmed_at: Utc::now(),
            }),
            _ => Err(OrderError::InvalidStatus),
        }
    }

    /// Cancel order
    pub fn cancel(&self, reason: String) -> Result<OrderCancelledEvent, OrderError> {
        match self.status {
            OrderStatus::Shipped | OrderStatus::Delivered => {
                Err(OrderError::CannotCancel)
            }
            _ => Ok(OrderCancelledEvent {
                order_id: self.id,
                reason,
                cancelled_at: Utc::now(),
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("Order must have at least one item")]
    NoItems,
    #[error("Invalid order status for this operation")]
    InvalidStatus,
    #[error("Cannot cancel shipped or delivered order")]
    CannotCancel,
}
```

---

#### 1.5 Event Store Interface

**`crates/event-store/src/lib.rs`**:
```rust
use async_trait::async_trait;
use uuid::Uuid;

pub mod postgres_event_store;

#[derive(Debug, Clone)]
pub struct Event {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub event_type: String,
    pub event_version: i32,
    pub payload: serde_json::Value,
    pub metadata: serde_json::Value,
    pub sequence_number: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to the store
    async fn append_events(
        &self,
        aggregate_id: Uuid,
        expected_version: i64,
        events: Vec<Event>,
    ) -> Result<(), EventStoreError>;

    /// Load all events for an aggregate
    async fn load_events(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<Event>, EventStoreError>;

    /// Load events from specific version
    async fn load_events_from_version(
        &self,
        aggregate_id: Uuid,
        from_version: i64,
    ) -> Result<Vec<Event>, EventStoreError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EventStoreError {
    #[error("Concurrency conflict: expected version {expected}, got {actual}")]
    ConcurrencyConflict { expected: i64, actual: i64 },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
```

---

#### 1.6 PostgreSQL Event Store Implementation

**`crates/event-store/src/postgres_event_store.rs`**:
```rust
use super::{Event, EventStore, EventStoreError};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventStore for PostgresEventStore {
    async fn append_events(
        &self,
        aggregate_id: Uuid,
        expected_version: i64,
        events: Vec<Event>,
    ) -> Result<(), EventStoreError> {
        let mut tx = self.pool.begin().await?;

        // Check current version (optimistic locking)
        let current_version: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM events WHERE aggregate_id = $1"
        )
        .bind(aggregate_id)
        .fetch_optional(&mut *tx)
        .await?;

        let current = current_version.unwrap_or(0);
        if current != expected_version {
            return Err(EventStoreError::ConcurrencyConflict {
                expected: expected_version,
                actual: current,
            });
        }

        // Insert events
        for (i, event) in events.iter().enumerate() {
            let version = expected_version + i as i64 + 1;

            sqlx::query(
                r#"
                INSERT INTO events (
                    event_id, aggregate_id, aggregate_type, event_type,
                    event_version, payload, metadata, version, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#
            )
            .bind(event.event_id)
            .bind(aggregate_id)
            .bind(&event.aggregate_type)
            .bind(&event.event_type)
            .bind(event.event_version)
            .bind(&event.payload)
            .bind(&event.metadata)
            .bind(version)
            .bind(event.created_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_events(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<Event>, EventStoreError> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT event_id, aggregate_id, aggregate_type, event_type,
                   event_version, payload, metadata, version as sequence_number, created_at
            FROM events
            WHERE aggregate_id = $1
            ORDER BY version ASC
            "#
        )
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    async fn load_events_from_version(
        &self,
        aggregate_id: Uuid,
        from_version: i64,
    ) -> Result<Vec<Event>, EventStoreError> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT event_id, aggregate_id, aggregate_type, event_type,
                   event_version, payload, metadata, version as sequence_number, created_at
            FROM events
            WHERE aggregate_id = $1 AND version > $2
            ORDER BY version ASC
            "#
        )
        .bind(aggregate_id)
        .bind(from_version)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }
}
```

---

#### 1.7 Database Migrations

**`migrations/001_create_events_table.sql`**:
```sql
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

    -- Ensure version uniqueness per aggregate
    UNIQUE(aggregate_id, version)
);

-- Index for aggregate queries
CREATE INDEX idx_events_aggregate ON events(aggregate_id, version);

-- Index for event type queries
CREATE INDEX idx_events_type ON events(event_type);

-- Index for timestamp queries
CREATE INDEX idx_events_created ON events(created_at);

-- GIN index for JSONB queries
CREATE INDEX idx_events_payload ON events USING GIN (payload);
```

---

### Deliverables - Phase 1
- ✅ Workspace setup with multiple crates
- ✅ Core domain events defined
- ✅ Order aggregate with business logic
- ✅ Event store trait and PostgreSQL implementation
- ✅ Database migrations for event store
- ✅ Basic error handling with `thiserror`

---

## **Phase 2: Command Side (Write Model)** (Week 3-4)

### Goals
- Implement command handlers
- Set up Kafka event publishing
- Build HTTP API with Axum
- Add validation

### Tasks

#### 2.1 Command Types

**`crates/domain/src/commands/order_commands.rs`**:
```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateOrderCommand {
    #[validate(length(min = 1))]
    pub customer_id: Uuid,

    #[validate(length(min = 1, message = "Order must have at least one item"))]
    pub items: Vec<CreateOrderItem>,

    pub shipping_address: ShippingAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateOrderItem {
    pub product_id: Uuid,
    pub sku: String,

    #[validate(range(min = 1))]
    pub quantity: u32,

    #[validate(range(min = 0.01))]
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ShippingAddress {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderCommand {
    pub order_id: Uuid,
    pub reason: String,
}
```

---

#### 2.2 Kafka Event Publisher

**`crates/messaging/src/producer.rs`**:
```rust
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

pub struct EventPublisher {
    producer: FutureProducer,
    topic: String,
}

impl EventPublisher {
    pub fn new(brokers: &str, topic: String) -> Result<Self, Box<dyn std::error::Error>> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "snappy")
            .set("acks", "all")
            .create()?;

        Ok(Self { producer, topic })
    }

    pub async fn publish<T: Serialize>(
        &self,
        key: Uuid,
        event: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_string(event)?;
        let key_str = key.to_string();

        let record = FutureRecord::to(&self.topic)
            .key(&key_str)
            .payload(&payload);

        self.producer
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await
            .map_err(|(err, _)| err)?;

        Ok(())
    }
}
```

---

#### 2.3 Command Handler

**`services/command-service/src/handlers/create_order.rs`**:
```rust
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use domain::aggregates::order::OrderAggregate;
use domain::commands::order_commands::CreateOrderCommand;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct CreateOrderResponse {
    pub order_id: Uuid,
    pub order_number: String,
    pub status: String,
}

pub async fn create_order_handler(
    State(state): State<AppState>,
    Json(cmd): Json<CreateOrderCommand>,
) -> Result<Json<CreateOrderResponse>, (StatusCode, String)> {
    // Validate command
    cmd.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Create aggregate and generate event
    let items = cmd.items.iter().map(|i| domain::events::order_events::OrderItem {
        product_id: i.product_id,
        sku: i.sku.clone(),
        quantity: i.quantity,
        unit_price: i.unit_price,
    }).collect();

    let (aggregate, event) = OrderAggregate::create(cmd.customer_id, items)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Persist event to event store
    let event_envelope = create_event_envelope(&aggregate.id, event);
    state.event_store
        .append_events(aggregate.id, 0, vec![event_envelope.clone()])
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Publish to Kafka
    state.event_publisher
        .publish(aggregate.id, &event_envelope)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CreateOrderResponse {
        order_id: aggregate.id,
        order_number: aggregate.order_number,
        status: "CREATED".to_string(),
    }))
}
```

---

#### 2.4 Axum HTTP Server

**`services/command-service/src/main.rs`**:
```rust
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
mod routes;
mod state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize state (DB pool, Kafka, etc.)
    let state = state::AppState::new().await?;

    // Build router
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/v1/orders", post(handlers::create_order::create_order_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Command service listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

### Deliverables - Phase 2
- ✅ Command types with validation
- ✅ Kafka event publisher
- ✅ Command handlers (CreateOrder, CancelOrder)
- ✅ Axum HTTP API
- ✅ Distributed tracing setup

---

## **Phase 3: Query Side (Read Model)** (Week 5-6)

### Goals
- Build read model projections
- Implement Redis caching
- Create query handlers
- Set up Kafka consumers

### Tasks

#### 3.1 Read Model Schema

**`migrations/003_create_order_views_table.sql`**:
```sql
CREATE TABLE order_views (
    order_id UUID PRIMARY KEY,
    customer_id UUID NOT NULL,
    order_number VARCHAR(50) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL,
    total_amount DECIMAL(10,2) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    items JSONB NOT NULL,
    shipping_address JSONB,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    version BIGINT NOT NULL,

    INDEX idx_customer (customer_id),
    INDEX idx_status (status),
    INDEX idx_created (created_at)
);
```

---

#### 3.2 Order Projection

**`crates/read-model/src/projections/order_projection.rs`**:
```rust
use async_trait::async_trait;
use domain::events::order_events::*;
use sqlx::PgPool;
use uuid::Uuid;

pub struct OrderProjection {
    pool: PgPool,
}

impl OrderProjection {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn handle_order_created(
        &self,
        event: &OrderCreatedEvent,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO order_views (
                order_id, customer_id, order_number, status,
                total_amount, currency, items, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)
            "#
        )
        .bind(event.order_id)
        .bind(event.customer_id)
        .bind(&event.order_number)
        .bind("CREATED")
        .bind(event.total_amount)
        .bind(&event.currency)
        .bind(serde_json::to_value(&event.items).unwrap())
        .bind(event.created_at)
        .bind(event.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn handle_order_confirmed(
        &self,
        event: &OrderConfirmedEvent,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE order_views
            SET status = 'CONFIRMED', updated_at = $1, version = version + 1
            WHERE order_id = $2
            "#
        )
        .bind(event.confirmed_at)
        .bind(event.order_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
```

---

#### 3.3 Kafka Event Consumer

**`services/projection-service/src/event_processor.rs`**:
```rust
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::time::Duration;
use tracing::{error, info};

pub struct EventProcessor {
    consumer: StreamConsumer,
    projection: OrderProjection,
}

impl EventProcessor {
    pub fn new(brokers: &str, group_id: &str, projection: OrderProjection) -> Result<Self, Box<dyn std::error::Error>> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&["order-events"])?;

        Ok(Self { consumer, projection })
    }

    pub async fn start(&self) {
        info!("Starting event processor...");

        loop {
            match self.consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        if let Err(e) = self.process_message(payload).await {
                            error!("Error processing message: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Kafka error: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn process_message(&self, payload: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let event_envelope: EventEnvelope = serde_json::from_slice(payload)?;

        match event_envelope.event_type.as_str() {
            "OrderCreated" => {
                let event: OrderCreatedEvent = serde_json::from_value(event_envelope.payload)?;
                self.projection.handle_order_created(&event).await?;
            }
            "OrderConfirmed" => {
                let event: OrderConfirmedEvent = serde_json::from_value(event_envelope.payload)?;
                self.projection.handle_order_confirmed(&event).await?;
            }
            _ => info!("Unknown event type: {}", event_envelope.event_type),
        }

        Ok(())
    }
}
```

---

#### 3.4 Query Handlers

**`services/query-service/src/handlers/get_order.rs`**:
```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct OrderView {
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub order_number: String,
    pub status: String,
    pub total_amount: f64,
    pub items: serde_json::Value,
}

pub async fn get_order_handler(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderView>, StatusCode> {
    // Try cache first
    if let Some(cached) = state.cache.get(&order_id).await {
        return Ok(Json(cached));
    }

    // Query database
    let order = sqlx::query_as::<_, OrderView>(
        r#"
        SELECT order_id, customer_id, order_number, status, total_amount, items
        FROM order_views
        WHERE order_id = $1
        "#
    )
    .bind(order_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Update cache
    state.cache.set(&order_id, &order, 300).await;

    Ok(Json(order))
}
```

### Deliverables - Phase 3
- ✅ Read model database schema
- ✅ Event projections
- ✅ Kafka event consumer
- ✅ Query handlers with Redis caching
- ✅ Query service API

---

## **Phase 4: Saga Orchestration** (Week 7-8)

### Goals
- Implement Saga pattern
- Handle distributed transactions
- Add compensation logic
- Persist saga state

**See ARCHITECTURE.md for full Saga implementation details**

---

## **Phase 5: Production Features** (Week 9-10)

### Goals
- Add distributed tracing (Jaeger)
- Implement metrics (Prometheus)
- Event replay functionality
- Idempotency handling
- Circuit breakers

---

## **Phase 6: Testing & Deployment** (Week 11-12)

### Goals
- Unit tests with `mockall`
- Integration tests with `testcontainers`
- Load testing
- Docker deployment
- Kubernetes manifests

---

## 🎓 Learning Resources for Rust CQRS/ES

### Essential Reading
1. **The Rust Book** - https://doc.rust-lang.org/book/
2. **Async Rust** - https://rust-lang.github.io/async-book/
3. **CQRS Documents** - https://cqrs.files.wordpress.com/2010/11/cqrs_documents.pdf
4. **Event Sourcing with Rust** - https://github.com/johnbywater/eventsourcing-rust

### Video Tutorials
- **Rust Tokio Tutorial** - Async programming fundamentals
- **Building Microservices in Rust** - Production patterns
- **Event Sourcing Explained** - Greg Young

### Example Projects
- https://github.com/serverlesstechnology/cqrs-demo
- https://github.com/tokio-rs/axum/tree/main/examples
- https://github.com/eventstore/EventStore

---

## ⚠️ Common Rust Pitfalls & Solutions

### 1. **Lifetime Hell**
**Problem**: Fighting with lifetime annotations
```rust
// ❌ This won't compile
pub struct Handler<'a> {
    repo: &'a Repository,
}
```

**Solution**: Use `Arc` for shared ownership
```rust
use std::sync::Arc;

pub struct Handler {
    repo: Arc<Repository>,
}
```

---

### 2. **Async Trait Methods**
**Problem**: Traits with async methods don't work natively
```rust
// ❌ This won't compile
trait EventStore {
    async fn save(&self, event: Event);
}
```

**Solution**: Use `async-trait` crate
```rust
use async_trait::async_trait;

#[async_trait]
trait EventStore {
    async fn save(&self, event: Event);
}
```

---

### 3. **Cloning Large Structs**
**Problem**: Performance hit from excessive cloning
```rust
// ❌ Expensive
let event_copy = event.clone();
process(event_copy);
```

**Solution**: Use references or `Arc`
```rust
// ✅ Better
process(&event);

// ✅ Or for shared ownership
let event = Arc::new(event);
process(Arc::clone(&event));
```

---

## 📊 Estimated Effort

| Phase | Duration | Difficulty | Key Challenge |
|-------|----------|-----------|---------------|
| Phase 1: Foundation | 2 weeks | ⭐⭐⭐ | Learning Rust ownership |
| Phase 2: Commands | 2 weeks | ⭐⭐⭐⭐ | Async/await patterns |
| Phase 3: Queries | 2 weeks | ⭐⭐⭐ | Event projections |
| Phase 4: Sagas | 2 weeks | ⭐⭐⭐⭐⭐ | Distributed state |
| Phase 5: Production | 2 weeks | ⭐⭐⭐⭐ | Observability |
| Phase 6: Testing | 2 weeks | ⭐⭐⭐ | Integration tests |

**Total**: 10-12 weeks (vs 4-6 weeks for Go)

---

## 🚀 Getting Started

Ready to start? Here's what I'll implement first:

1. ✅ Workspace structure
2. ✅ Domain events and aggregates
3. ✅ PostgreSQL event store
4. ✅ Basic command handler
5. ✅ Docker Compose setup

**Shall I begin implementing Phase 1 in Rust?** 🦀
