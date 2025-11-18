# Phase 4 Implementation - Saga Orchestration

## Overview

Phase 4 implements the **Saga Pattern** for managing distributed transactions in the CQRS order processing system. This phase builds upon the event sourcing foundation (Phase 1), command processing (Phase 2), and query side (Phase 3) to add reliable distributed transaction coordination with automatic compensation.

## What is the Saga Pattern?

The **Saga Pattern** is a design pattern for managing distributed transactions across multiple microservices. Instead of using traditional ACID transactions (which don't scale in distributed systems), sagas break a transaction into a series of local transactions, with each local transaction updating a single service and publishing an event or message to trigger the next step.

### Key Characteristics

- **Eventual Consistency**: The system achieves consistency over time rather than immediately
- **Compensation**: Failed sagas are rolled back through compensating transactions
- **Atomicity**: Either all steps complete successfully, or all completed steps are compensated
- **Resilience**: The system can recover from failures and continue processing

### Saga vs Traditional Transactions

| Aspect | Traditional Transaction | Saga Pattern |
|--------|------------------------|--------------|
| **Scope** | Single database | Multiple services |
| **Consistency** | ACID | Eventual consistency |
| **Rollback** | Database rollback | Compensating transactions |
| **Locks** | Pessimistic locking | Optimistic locking |
| **Scalability** | Limited | Highly scalable |

## Architecture

### Saga Orchestration Pattern

This implementation uses the **orchestration** approach (as opposed to choreography), where a central coordinator manages the saga workflow.

```
┌─────────────────────────────────────────────────────────────┐
│                   Saga Orchestrator                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           Saga Coordinator                           │  │
│  │  - Manages saga lifecycle                            │  │
│  │  - Executes steps sequentially                       │  │
│  │  - Handles failures and compensation                 │  │
│  │  - Persists saga state                               │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ Events
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     Event Stream (Kafka)                    │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐   ┌───────────────┐   ┌──────────────┐
│  Inventory   │   │   Payment     │   │    Order     │
│  Service     │   │   Service     │   │   Service    │
│              │   │               │   │              │
│ - Reserve    │   │ - Authorize   │   │ - Confirm    │
│ - Release    │   │ - Void        │   │ - Cancel     │
└──────────────┘   └───────────────┘   └──────────────┘
```

## Implemented Components

### 1. Saga Core Crate (`crates/saga`)

A dedicated crate providing reusable saga infrastructure for any distributed transaction workflow.

#### Core Types

**SagaStatus** - Lifecycle states of a saga:
```rust
pub enum SagaStatus {
    Running,      // Saga is executing forward steps
    Completed,    // All steps completed successfully
    Compensating, // Saga failed, rolling back
    Compensated,  // All compensations completed
    Failed,       // Compensation failed (manual intervention needed)
}
```

**StepStatus** - Status of individual saga steps:
```rust
pub enum StepStatus {
    Pending,             // Not yet executed
    Running,             // Currently executing
    Completed,           // Executed successfully
    Failed,              // Execution failed
    Compensating,        // Compensation in progress
    Compensated,         // Compensation completed
    CompensationFailed,  // Compensation failed
}
```

**SagaState** - Complete state of a saga instance:
```rust
pub struct SagaState {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub status: SagaStatus,
    pub current_step: usize,
    pub steps: Vec<SagaStep>,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**SagaStep** - Individual step in a saga:
```rust
pub struct SagaStep {
    pub name: String,
    pub status: StepStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
```

#### Traits

**StepExecutor** - Interface for saga step execution:
```rust
#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn execute(&self, context: &StepContext) -> Result<serde_json::Value>;
    async fn compensate(&self, context: &StepContext) -> Result<()>;
}
```

**Saga** - Interface for saga implementations:
```rust
#[async_trait]
pub trait Saga: Send + Sync {
    fn saga_type(&self) -> &str;
    fn step_executors(&self) -> &HashMap<String, Box<dyn StepExecutor>>;
    async fn create_state(&self, saga_id: Uuid, data: serde_json::Value) -> Result<SagaState>;
    async fn execute_next_step(&self, state: &mut SagaState) -> Result<()>;
    async fn compensate_step(&self, state: &mut SagaState, step_index: usize) -> Result<()>;
    async fn compensate_all(&self, state: &mut SagaState) -> Result<()>;
}
```

#### Saga Coordinator

The coordinator orchestrates saga execution:

**Key Methods**:
- `start_saga()`: Initialize a new saga instance
- `execute_step()`: Execute the next step in the saga
- `run_saga()`: Execute all steps to completion
- `compensate_saga()`: Rollback all completed steps
- `resume_saga()`: Resume a saga from its current state
- `retry_failed_sagas()`: Retry sagas that can be retried

**Features**:
- ✅ Automatic state persistence after each step
- ✅ Automatic compensation on failure
- ✅ Retry logic for transient failures
- ✅ Saga resumption after restarts
- ✅ Comprehensive error handling

#### Saga Repository

PostgreSQL-based persistence for saga state:

**Features**:
- ✅ Save new saga instances
- ✅ Update saga state
- ✅ Load saga by ID
- ✅ Query sagas by status
- ✅ Delete completed sagas
- ✅ JSONB storage for flexible state

### 2. Order Processing Saga (`services/saga-orchestrator/src/sagas/order_saga.rs`)

A concrete implementation of the Saga pattern for order processing.

#### Saga Steps

The order processing saga consists of three steps:

**Step 1: Reserve Inventory**
- **Action**: Reserve items from inventory
- **Event Published**: `InventoryReservedEvent`
- **Compensation**: Release reserved inventory
- **Compensation Event**: `InventoryReleasedEvent`

**Step 2: Authorize Payment**
- **Action**: Authorize payment (but don't capture)
- **Event Published**: `PaymentAuthorizedEvent`
- **Compensation**: Void the payment authorization
- **Compensation Event**: `PaymentVoidedEvent`

**Step 3: Confirm Order**
- **Action**: Confirm the order
- **Event Published**: `OrderConfirmedEvent`
- **Compensation**: Cancel the order (if needed)
- **Compensation Event**: `OrderCancelledEvent`

#### Execution Flow

**Happy Path** (all steps succeed):
```
OrderCreated
    ↓
Reserve Inventory → InventoryReservedEvent
    ↓
Authorize Payment → PaymentAuthorizedEvent
    ↓
Confirm Order → OrderConfirmedEvent
    ↓
Saga COMPLETED
```

**Failure Path** (e.g., payment fails):
```
OrderCreated
    ↓
Reserve Inventory → InventoryReservedEvent ✓
    ↓
Authorize Payment → PaymentFailed ✗
    ↓
COMPENSATION TRIGGERED
    ↓
Void Payment (N/A - never authorized)
    ↓
Release Inventory → InventoryReleasedEvent ✓
    ↓
Saga COMPENSATED
```

#### OrderSagaData

The saga carries context data through all steps:

```rust
pub struct OrderSagaData {
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub currency: String,
    pub payment_method: String,
    pub correlation_id: Uuid,
}
```

### 3. New Domain Events

#### Inventory Events (`crates/domain/src/events/inventory_events.rs`)

- **InventoryReservedEvent**: Items reserved for an order
- **InventoryReleasedEvent**: Reserved items released (compensation)
- **InventoryReservationFailedEvent**: Reservation failed (not enough stock)
- **StockReplenishedEvent**: New stock added

#### Payment Events (`crates/domain/src/events/payment_events.rs`)

- **PaymentAuthorizedEvent**: Payment authorized (not captured)
- **PaymentCapturedEvent**: Payment funds captured
- **PaymentVoidedEvent**: Authorization voided (compensation)
- **PaymentFailedEvent**: Payment failed
- **PaymentRefundedEvent**: Payment refunded

### 4. Saga Orchestrator Service (`services/saga-orchestrator`)

A dedicated microservice that:

- **Listens** for `OrderCreatedEvent` from Kafka
- **Starts** the OrderProcessingSaga for each new order
- **Executes** saga steps sequentially
- **Publishes** events for each step (inventory, payment, order)
- **Handles** failures with automatic compensation
- **Persists** saga state to PostgreSQL

**Service Flow**:
```
1. Kafka Consumer receives OrderCreatedEvent
2. Create OrderSagaData from event
3. Start OrderProcessingSaga with coordinator
4. Coordinator executes each step:
   a. Execute step logic
   b. Publish event
   c. Save saga state
   d. Move to next step
5. If failure: compensate completed steps
6. Saga completes (COMPLETED or COMPENSATED)
```

### 5. Database Schema

**saga_instances table**:
```sql
CREATE TABLE saga_instances (
    saga_id UUID PRIMARY KEY,
    saga_type VARCHAR(100) NOT NULL,
    current_step INT NOT NULL DEFAULT 0,
    state JSONB NOT NULL,
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**saga_event_log table**:
```sql
CREATE TABLE saga_event_log (
    event_id UUID PRIMARY KEY,
    saga_id UUID NOT NULL REFERENCES saga_instances(saga_id),
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Indexes**:
- `idx_saga_status`: Fast queries by status
- `idx_saga_type`: Fast queries by saga type
- `idx_saga_status_created`: Compound index for finding stuck sagas

## Testing

### Unit Tests

The saga crate includes comprehensive unit tests:

**Step Tests** (`crates/saga/src/step.rs`):
- Step lifecycle transitions
- Retry logic
- Compensation tracking

**Saga State Tests** (`crates/saga/src/saga.rs`):
- State progression
- Compensation step ordering
- Status transitions

**Coordinator Tests** (`crates/saga/src/coordinator.rs`):
- Saga start and execution
- Failure handling
- Compensation logic

### Integration Tests

Integration tests (`tests/integration/saga_tests.rs`) verify:

- ✅ Successful saga execution (all steps complete)
- ✅ Saga failure with compensation
- ✅ Saga state persistence
- ✅ Saga resumption after interruption
- ✅ Finding sagas by status
- ✅ Retry logic for transient failures

**Run tests**:
```bash
# Unit tests (all crates)
cargo test --workspace

# Integration tests
cargo test --test saga_tests

# Specific saga tests
cargo test -p saga --lib

# With output
cargo test -- --nocapture
```

## Key Features

### 1. Automatic Compensation

When a step fails, the saga automatically compensates all previously completed steps **in reverse order**:

```rust
// Compensation logic
let compensation_steps = state.get_compensation_steps(); // Reverse order!
for (index, _) in compensation_steps {
    saga.compensate_step(state, index).await?;
}
```

### 2. Retry Logic

Each step can be configured with retry attempts:

```rust
SagaStep::new("reserve_inventory".to_string(), 3) // Max 3 retries
```

If a step fails but hasn't exceeded max retries, it can be retried.

### 3. State Persistence

Saga state is persisted after every step, ensuring recovery after crashes:

```rust
// After each step
self.repository.update(&state).await?;
```

### 4. Event-Driven Communication

Each saga step publishes events to Kafka, enabling:
- Loose coupling between services
- Event replay capabilities
- Audit trail of all actions
- Integration with other services

### 5. Idempotency

The saga event log table tracks processed events, preventing duplicate processing.

### 6. Observability

Comprehensive tracing with correlation IDs:

```rust
info!(
    saga_id = %saga_id,
    step = %step_name,
    correlation_id = %correlation_id,
    "Executing saga step"
);
```

## Running the Saga Orchestrator

### Prerequisites

```bash
# Ensure infrastructure is running
docker-compose up -d

# Run database migrations (includes saga tables)
make migrate
```

### Start the Service

```bash
# Run saga orchestrator
cargo run --bin saga-orchestrator

# Or use make
make run-saga-orchestrator
```

### Configuration

Environment variables (`.env`):
```bash
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/cqrs
KAFKA_BROKERS=localhost:9093
RUST_LOG=info
```

## Monitoring Sagas

### Query Saga State

```sql
-- Get all running sagas
SELECT saga_id, saga_type, current_step, status, created_at
FROM saga_instances
WHERE status = 'RUNNING'
ORDER BY created_at DESC;

-- Get saga details
SELECT *
FROM saga_instances
WHERE saga_id = 'your-saga-id';

-- Find stuck sagas (running > 5 minutes)
SELECT saga_id, saga_type, current_step, created_at
FROM saga_instances
WHERE status = 'RUNNING'
  AND updated_at < NOW() - INTERVAL '5 minutes';
```

### Saga Metrics

Key metrics to track:

- **Saga completion rate**: % of sagas that complete successfully
- **Compensation rate**: % of sagas that require compensation
- **Step failure rate**: Failures per step type
- **Saga duration**: Time from start to completion
- **Stuck sagas**: Sagas not progressing

## Error Handling

### Saga Error Types

```rust
pub enum SagaError {
    StepExecutionFailed(String),      // Step failed to execute
    CompensationFailed(String),       // Compensation failed
    AlreadyCompleted,                 // Saga already done
    InvalidStateTransition,           // Invalid state change
    DatabaseError(sqlx::Error),       // DB error
    SagaNotFound(String),             // Saga doesn't exist
    StepNotFound(String),             // Step doesn't exist
    InternalError(String),            // Other errors
}
```

### Compensation Failure

If compensation fails, the saga is marked as `FAILED` and requires **manual intervention**:

```sql
-- Find failed sagas
SELECT *
FROM saga_instances
WHERE status = 'FAILED';
```

## Production Considerations

### 1. Saga Timeout

Implement timeout for long-running sagas:
```rust
// Pseudo-code
if saga.updated_at < now - timeout {
    compensate_saga(saga);
}
```

### 2. Dead Letter Queue

Failed sagas that can't be compensated should go to a DLQ for manual review.

### 3. Saga Cleanup

Completed sagas should be archived/deleted periodically:
```sql
DELETE FROM saga_instances
WHERE status = 'COMPLETED'
  AND created_at < NOW() - INTERVAL '30 days';
```

### 4. Concurrent Saga Execution

The current implementation executes sagas sequentially per order. For higher throughput, consider:
- Parallel saga execution with worker pools
- Partitioning sagas by order ID
- Multiple saga orchestrator instances

### 5. Saga Versioning

As sagas evolve, version them to support backward compatibility:
```rust
pub struct SagaState {
    pub saga_version: i32,  // Add version field
    // ...
}
```

## Comparison with Other Patterns

### Saga vs Two-Phase Commit (2PC)

| Aspect | Saga | Two-Phase Commit |
|--------|------|------------------|
| **Coordinator** | Lightweight | Resource-intensive |
| **Locks** | None (optimistic) | Distributed locks |
| **Failure handling** | Compensation | Rollback |
| **Availability** | High | Low (coordinator SPOF) |
| **Use case** | Microservices | Distributed databases |

### Saga vs Event Choreography

| Aspect | Saga (Orchestration) | Event Choreography |
|--------|---------------------|---------------------|
| **Control** | Central coordinator | Distributed |
| **Complexity** | Easier to reason about | Harder to debug |
| **Coupling** | Coordinator knows all steps | Services independent |
| **Observability** | Better (central view) | Harder (distributed) |
| **Flexibility** | Easier to change workflow | Harder to change flow |

## Future Enhancements

### Phase 5 Will Add:

- ✅ Distributed tracing (Jaeger) for saga execution
- ✅ Prometheus metrics for saga monitoring
- ✅ Circuit breakers for external service calls
- ✅ Saga timeout and automatic recovery
- ✅ Saga event replay for debugging

## Real-World Usage

The Saga pattern is used by:

| Company | Use Case | Scale |
|---------|----------|-------|
| **Uber** | Trip booking | Millions of trips/day |
| **Netflix** | Subscription management | 200M+ subscribers |
| **Amazon** | Order fulfillment | Billions of orders |
| **Airbnb** | Booking workflow | Complex multi-step bookings |

## Learning Resources

### Papers & Articles
- [Sagas (Original Paper)](https://www.cs.cornell.edu/andru/cs711/2002fa/reading/sagas.pdf) - Hector Garcia-Molina, 1987
- [Microservices.io - Saga Pattern](https://microservices.io/patterns/data/saga.html)
- [GOTO 2015 - Applying the Saga Pattern](https://www.youtube.com/watch?v=xDuwrtwYHu8) - Caitie McCaffrey

### Books
- **Designing Data-Intensive Applications** - Martin Kleppmann (Chapter 9: Consistency and Consensus)
- **Microservices Patterns** - Chris Richardson (Chapter 4: Sagas)

## Summary

Phase 4 adds production-ready distributed transaction management to the CQRS system:

- ✅ **Saga Pattern Implementation**: Reliable distributed transactions
- ✅ **Automatic Compensation**: Rollback on failures
- ✅ **State Persistence**: Saga recovery after crashes
- ✅ **Event-Driven**: Integration with event sourcing
- ✅ **Testable**: Comprehensive unit and integration tests
- ✅ **Production-Ready**: Error handling, logging, monitoring

The saga orchestrator handles complex order processing workflows while maintaining eventual consistency and providing automatic recovery from failures.

**Next**: Phase 5 will add production features including distributed tracing, metrics, circuit breakers, and advanced monitoring.

---

## Quick Reference

### Create a New Saga

```rust
// 1. Define saga data
#[derive(Serialize, Deserialize)]
struct MySagaData { /* ... */ }

// 2. Implement step executors
struct MyStep { /* ... */ }

#[async_trait]
impl StepExecutor for MyStep {
    async fn execute(&self, ctx: &StepContext) -> Result<Value> { /* ... */ }
    async fn compensate(&self, ctx: &StepContext) -> Result<()> { /* ... */ }
}

// 3. Implement Saga trait
struct MySaga { executors: HashMap<String, Box<dyn StepExecutor>> }

#[async_trait]
impl Saga for MySaga {
    fn saga_type(&self) -> &str { "MySaga" }
    // ... implement other methods
}

// 4. Use coordinator
let coordinator = SagaCoordinator::new(repository);
let state = coordinator.start_saga(&saga, saga_id, data).await?;
let result = coordinator.run_saga(&saga, state).await?;
```

### Query Sagas

```rust
// Get saga state
let state = coordinator.get_saga_state(saga_id).await?;

// Find running sagas
let running = coordinator.find_sagas_by_status(SagaStatus::Running, 100).await?;

// Resume saga
let result = coordinator.resume_saga(&saga, saga_id).await?;
```
