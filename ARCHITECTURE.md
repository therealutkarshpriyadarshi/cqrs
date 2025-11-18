# CQRS & Event Sourcing Order Processing System
## Architecture Design Document

---

## 🎯 System Overview

This project implements a production-grade order processing system using **Command Query Responsibility Segregation (CQRS)** and **Event Sourcing** patterns, similar to systems used by Netflix, Uber, and Amazon.

### Core Principles
- **CQRS**: Separate write (command) and read (query) models
- **Event Sourcing**: Immutable event log as the single source of truth
- **Eventual Consistency**: Accept temporary inconsistency for scalability
- **Event-Driven**: All state changes captured as domain events
- **Saga Pattern**: Distributed transactions with compensation

---

## 🏗️ System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        API Gateway / Load Balancer              │
└───────────────┬─────────────────────────────────────────────────┘
                │
    ┌───────────┴──────────────┐
    │                          │
┌───▼────────┐         ┌──────▼──────┐
│  Command   │         │    Query    │
│  Service   │         │   Service   │
│  (Write)   │         │    (Read)   │
└─────┬──────┘         └──────▲──────┘
      │                       │
      │ Commands              │ Projections
      │                       │
┌─────▼─────────────────────────────────────┐
│         Event Store (Source of Truth)     │
│  ┌──────────────────────────────────┐    │
│  │  Event Stream (Kafka/NATS)       │    │
│  └──────────────────────────────────┘    │
└─────┬─────────────────────────────────────┘
      │
      │ Event Subscribers
      │
  ┌───┴────────────────────┬──────────────────┬──────────────────┐
  │                        │                  │                  │
┌─▼────────────┐  ┌───────▼────────┐  ┌─────▼─────┐  ┌────────▼────────┐
│ Projection   │  │  Saga          │  │  Read     │  │  Notification   │
│ Engine       │  │  Orchestrator  │  │  Model    │  │  Service        │
│              │  │                │  │  Cache    │  │                 │
└──────────────┘  └────────────────┘  └───────────┘  └─────────────────┘
                                           (Redis)
```

---

## 📦 Technology Stack

### Recommended: **Go (Golang)** Implementation

#### Core Services
- **API Framework**: Gin (lightweight, fast HTTP routing)
- **Event Streaming**: Kafka with Confluent Schema Registry
- **Event Store**: PostgreSQL with dedicated event_store schema
- **Read Model Cache**: Redis (for materialized views)
- **Inter-Service Communication**: gRPC with Protocol Buffers
- **Database**: PostgreSQL (command & query databases)

#### Supporting Infrastructure
- **Message Broker**: Apache Kafka + Zookeeper
- **Service Discovery**: Consul or etcd
- **API Gateway**: Kong or Traefik
- **Monitoring**: Prometheus + Grafana
- **Tracing**: Jaeger (distributed tracing)
- **Logging**: ELK Stack (Elasticsearch, Logstash, Kibana)

#### Development Tools
- **Schema Management**: golang-migrate
- **API Documentation**: Swagger/OpenAPI 3.0
- **Testing**: Testify, GoMock
- **Container**: Docker + Docker Compose
- **Orchestration**: Kubernetes (production)

### Alternative: **Python (FastAPI)** Implementation
- FastAPI for async API endpoints
- Pydantic for data validation
- SQLAlchemy for ORM
- aiokafka for Kafka integration
- asyncpg for PostgreSQL

---

## 🗄️ Database Design

### Event Store Schema (PostgreSQL)

```sql
-- Core event store table
CREATE TABLE events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_version INT NOT NULL DEFAULT 1,
    event_data JSONB NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    sequence_number BIGSERIAL,

    -- Optimistic locking
    expected_version INT,

    -- Indexing for queries
    INDEX idx_aggregate (aggregate_id, sequence_number),
    INDEX idx_event_type (event_type),
    INDEX idx_created_at (created_at)
);

-- Snapshots for performance optimization
CREATE TABLE snapshots (
    snapshot_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_id UUID NOT NULL UNIQUE,
    aggregate_type VARCHAR(100) NOT NULL,
    version INT NOT NULL,
    state JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Saga state tracking
CREATE TABLE saga_instances (
    saga_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    saga_type VARCHAR(100) NOT NULL,
    current_step INT NOT NULL DEFAULT 0,
    state JSONB NOT NULL,
    status VARCHAR(50) NOT NULL, -- RUNNING, COMPLETED, COMPENSATING, FAILED
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Idempotency tracking
CREATE TABLE processed_messages (
    message_id UUID PRIMARY KEY,
    processed_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### Read Model Schema (PostgreSQL)

```sql
-- Denormalized order view
CREATE TABLE order_views (
    order_id UUID PRIMARY KEY,
    customer_id UUID NOT NULL,
    order_number VARCHAR(50) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL,
    total_amount DECIMAL(10,2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    items JSONB NOT NULL,
    shipping_address JSONB,
    payment_info JSONB,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    version INT NOT NULL,

    INDEX idx_customer (customer_id),
    INDEX idx_status (status),
    INDEX idx_created (created_at)
);

-- Customer order history (optimized for queries)
CREATE TABLE customer_order_summary (
    customer_id UUID PRIMARY KEY,
    total_orders INT DEFAULT 0,
    total_spent DECIMAL(10,2) DEFAULT 0,
    last_order_date TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Product inventory view
CREATE TABLE inventory_views (
    product_id UUID PRIMARY KEY,
    sku VARCHAR(100) UNIQUE NOT NULL,
    available_quantity INT NOT NULL,
    reserved_quantity INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

---

## 📋 Event Types & Schema

### Domain Events

```go
// Base Event
type Event struct {
    EventID       string                 `json:"event_id"`
    AggregateID   string                 `json:"aggregate_id"`
    AggregateType string                 `json:"aggregate_type"`
    EventType     string                 `json:"event_type"`
    EventVersion  int                    `json:"event_version"`
    Data          map[string]interface{} `json:"data"`
    Metadata      EventMetadata          `json:"metadata"`
    Timestamp     time.Time              `json:"timestamp"`
}

type EventMetadata struct {
    CorrelationID string `json:"correlation_id"`
    CausationID   string `json:"causation_id"`
    UserID        string `json:"user_id,omitempty"`
}
```

### Order Events

1. **OrderCreated**
   ```json
   {
     "customer_id": "uuid",
     "order_number": "ORD-2024-001",
     "items": [
       {"product_id": "uuid", "quantity": 2, "price": 29.99}
     ],
     "total_amount": 59.98
   }
   ```

2. **OrderConfirmed**
   ```json
   {
     "confirmed_at": "2024-01-15T10:30:00Z"
   }
   ```

3. **PaymentProcessed**
   ```json
   {
     "payment_id": "uuid",
     "amount": 59.98,
     "payment_method": "credit_card"
   }
   ```

4. **OrderShipped**
   ```json
   {
     "tracking_number": "TRK123456",
     "carrier": "FedEx",
     "shipped_at": "2024-01-16T14:00:00Z"
   }
   ```

5. **OrderCancelled**
   ```json
   {
     "reason": "Customer request",
     "cancelled_at": "2024-01-15T12:00:00Z"
   }
   ```

### Inventory Events

1. **InventoryReserved**
2. **InventoryReleased**
3. **StockReplenished**

### Payment Events

1. **PaymentAuthorized**
2. **PaymentCaptured**
3. **PaymentRefunded**

---

## 🔄 Command Handlers

### Order Commands

```go
type CreateOrderCommand struct {
    CommandID    string
    CustomerID   string
    Items        []OrderItem
    ShippingAddr Address
}

type CancelOrderCommand struct {
    CommandID string
    OrderID   string
    Reason    string
}

type ConfirmPaymentCommand struct {
    CommandID     string
    OrderID       string
    PaymentID     string
    PaymentMethod string
}
```

### Command Handler Interface

```go
type CommandHandler interface {
    Handle(ctx context.Context, cmd Command) error
    Validate(cmd Command) error
}
```

---

## 🔍 Query Handlers

### Order Queries

```go
type GetOrderByIDQuery struct {
    OrderID string
}

type GetOrdersByCustomerQuery struct {
    CustomerID string
    Limit      int
    Offset     int
}

type GetOrdersByStatusQuery struct {
    Status string
    From   time.Time
    To     time.Time
}
```

### Query Handler Interface

```go
type QueryHandler interface {
    Handle(ctx context.Context, query Query) (interface{}, error)
}
```

---

## 🎭 Saga Pattern - Order Processing Saga

### Saga Steps

```
CreateOrder Saga:
1. Reserve Inventory       → Compensate: Release Inventory
2. Authorize Payment       → Compensate: Void Authorization
3. Create Shipping Label   → Compensate: Cancel Shipment
4. Confirm Order           → Compensate: Cancel Order
```

### Saga Implementation

```go
type OrderSaga struct {
    SagaID    string
    OrderID   string
    State     SagaState
    Steps     []SagaStep
}

type SagaStep struct {
    Name           string
    Execute        func(context.Context) error
    Compensate     func(context.Context) error
    Status         StepStatus
}
```

---

## 🚀 API Design

### Command API (Port 8080)

```
POST   /api/v1/orders              - Create order
POST   /api/v1/orders/{id}/cancel  - Cancel order
POST   /api/v1/orders/{id}/confirm - Confirm order
POST   /api/v1/payments            - Process payment
```

### Query API (Port 8081)

```
GET    /api/v1/orders/{id}                    - Get order details
GET    /api/v1/orders                         - List orders (paginated)
GET    /api/v1/customers/{id}/orders          - Customer order history
GET    /api/v1/orders/status/{status}         - Orders by status
GET    /api/v1/analytics/sales                - Sales analytics
```

### Admin API (Port 8082)

```
POST   /api/v1/admin/events/replay            - Replay events
GET    /api/v1/admin/events/{aggregate_id}    - View event stream
POST   /api/v1/admin/projections/rebuild      - Rebuild read models
GET    /api/v1/admin/health                   - System health
```

---

## 📁 Project Structure

```
cqrs/
├── cmd/
│   ├── command-service/
│   │   └── main.go
│   ├── query-service/
│   │   └── main.go
│   ├── projection-service/
│   │   └── main.go
│   └── saga-orchestrator/
│       └── main.go
├── internal/
│   ├── domain/
│   │   ├── order/
│   │   │   ├── aggregate.go
│   │   │   ├── events.go
│   │   │   ├── commands.go
│   │   │   └── repository.go
│   │   ├── payment/
│   │   └── inventory/
│   ├── application/
│   │   ├── commands/
│   │   │   └── handlers/
│   │   ├── queries/
│   │   │   └── handlers/
│   │   └── sagas/
│   ├── infrastructure/
│   │   ├── eventstore/
│   │   │   ├── postgres/
│   │   │   └── eventstore.go
│   │   ├── messaging/
│   │   │   ├── kafka/
│   │   │   └── publisher.go
│   │   ├── cache/
│   │   │   └── redis/
│   │   └── persistence/
│   │       └── postgres/
│   └── interfaces/
│       ├── http/
│       │   ├── command_api.go
│       │   └── query_api.go
│       └── grpc/
├── pkg/
│   ├── events/
│   ├── common/
│   └── errors/
├── deployments/
│   ├── docker/
│   │   ├── docker-compose.yml
│   │   └── Dockerfile
│   └── kubernetes/
├── scripts/
│   ├── db/
│   │   └── migrations/
│   └── setup/
├── tests/
│   ├── integration/
│   └── e2e/
├── docs/
│   └── api/
├── go.mod
├── go.sum
├── Makefile
├── README.md
└── ARCHITECTURE.md
```

---

## 🔧 Implementation Phases

### **Phase 1: Foundation** (Week 1-2)
- [ ] Project setup and directory structure
- [ ] Docker Compose for local development
- [ ] PostgreSQL event store schema
- [ ] Basic event store implementation
- [ ] Kafka setup with topics

### **Phase 2: Command Side** (Week 2-3)
- [ ] Order aggregate implementation
- [ ] Command handlers (CreateOrder, CancelOrder)
- [ ] Event publishing to Kafka
- [ ] Command API with Gin
- [ ] Unit tests for aggregates

### **Phase 3: Query Side** (Week 3-4)
- [ ] Event subscribers/consumers
- [ ] Projection engine
- [ ] Read model database
- [ ] Query handlers
- [ ] Query API with caching (Redis)

### **Phase 4: Saga Orchestration** (Week 4-5)
- [ ] Saga coordinator implementation
- [ ] Order processing saga
- [ ] Compensation logic
- [ ] Saga state persistence
- [ ] Integration tests

### **Phase 5: Advanced Features** (Week 5-6)
- [ ] Event replay mechanism
- [ ] Snapshot optimization
- [ ] Event versioning/upcasting
- [ ] Idempotency handling
- [ ] Circuit breakers

### **Phase 6: Production Readiness** (Week 6-7)
- [ ] gRPC inter-service communication
- [ ] Distributed tracing (Jaeger)
- [ ] Metrics (Prometheus)
- [ ] API documentation (Swagger)
- [ ] E2E tests
- [ ] Kubernetes manifests

### **Phase 7: Monitoring & Operations** (Week 7-8)
- [ ] Grafana dashboards
- [ ] Alert rules
- [ ] Log aggregation
- [ ] Performance testing
- [ ] Documentation

---

## 🚦 Getting Started - Prerequisites

### Required Software
- **Go 1.21+** or **Python 3.11+**
- **Docker** & **Docker Compose**
- **PostgreSQL 15+**
- **Apache Kafka 3.x**
- **Redis 7+**
- **Make** (build automation)

### Infrastructure Setup

```bash
# Start all infrastructure services
docker-compose up -d postgres kafka redis

# Run database migrations
make migrate-up

# Start services
make run-command-service
make run-query-service
make run-projection-service
```

---

## 📊 Key Metrics to Track

1. **Command Processing Time** - Latency from command to event persistence
2. **Event Processing Lag** - Time between event creation and projection update
3. **Query Response Time** - Read model query performance
4. **Saga Completion Rate** - Success rate of distributed transactions
5. **Event Replay Speed** - Time to rebuild read models from events

---

## 🎓 Learning Outcomes

After completing this project, you will understand:

✅ **CQRS Pattern** - Separating reads and writes for scalability
✅ **Event Sourcing** - Using events as the source of truth
✅ **Eventual Consistency** - Managing consistency in distributed systems
✅ **Saga Pattern** - Distributed transactions with compensation
✅ **Event-Driven Architecture** - Asynchronous, loosely-coupled systems
✅ **Domain-Driven Design** - Aggregate boundaries and ubiquitous language
✅ **Temporal Queries** - Querying system state at any point in time
✅ **Event Schema Evolution** - Versioning and backward compatibility

---

## 📚 References

- **Martin Fowler**: CQRS Pattern - https://martinfowler.com/bliki/CQRS.html
- **Greg Young**: Event Sourcing - https://www.eventstore.com/blog/what-is-event-sourcing
- **Microservices.io**: Saga Pattern - https://microservices.io/patterns/data/saga.html
- **Confluent**: Event Sourcing with Kafka - https://www.confluent.io/blog/event-sourcing-cqrs-stream-processing-apache-kafka-whats-connection/

---

**Next Steps**: Review this architecture, then we'll start implementing Phase 1!
