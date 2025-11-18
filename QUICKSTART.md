# Quick Start Guide - CQRS & Event Sourcing Order System

## 🚀 Get Up and Running in 15 Minutes

---

## Prerequisites Check

Before starting, ensure you have:

```bash
# Check Go version (1.21+)
go version

# Check Docker
docker --version
docker-compose --version

# Check Make
make --version

# Check PostgreSQL client (optional but helpful)
psql --version
```

**Don't have these?** See [Installation Guide](#installation-guide) below.

---

## Option 1: Quick Setup (Automated)

```bash
# Clone and setup
git clone <your-repo>
cd cqrs

# Run automated setup
make setup-all

# Start all services
make up

# Verify services are running
make health-check
```

**That's it!** Your CQRS system is running on:
- Command API: http://localhost:8080
- Query API: http://localhost:8081
- Admin API: http://localhost:8082

---

## Option 2: Manual Setup (Step by Step)

### Step 1: Infrastructure Setup

```bash
# Start PostgreSQL, Kafka, Redis
docker-compose up -d

# Wait for services to be ready (30 seconds)
docker-compose ps
```

**Expected output**:
```
NAME                SERVICE    STATUS        PORTS
cqrs-postgres-1    postgres   Up (healthy)  5432->5432
cqrs-kafka-1       kafka      Up            9092->9092
cqrs-redis-1       redis      Up            6379->6379
cqrs-zookeeper-1   zookeeper  Up            2181->2181
```

---

### Step 2: Database Initialization

```bash
# Run migrations
make migrate-up

# Verify tables created
make db-verify

# (Optional) Seed test data
make seed-data
```

---

### Step 3: Create Kafka Topics

```bash
# Create event topics
make kafka-topics-create

# Verify topics
make kafka-topics-list
```

**Topics created**:
- `order-events` - Order domain events
- `payment-events` - Payment events
- `inventory-events` - Inventory events
- `saga-events` - Saga orchestration events

---

### Step 4: Start Services

#### Terminal 1: Command Service (Write)
```bash
make run-command-service
```

Output:
```
[GIN] Listening on :8080
Command service started successfully
Connected to event store
Kafka producer ready
```

---

#### Terminal 2: Query Service (Read)
```bash
make run-query-service
```

Output:
```
[GIN] Listening on :8081
Query service started successfully
Connected to read database
Redis cache connected
```

---

#### Terminal 3: Projection Service (Event Processing)
```bash
make run-projection-service
```

Output:
```
Projection service started
Subscribed to order-events
Event consumer ready
```

---

### Step 5: Test the System

#### Create an Order (Command)

```bash
curl -X POST http://localhost:8080/api/v1/orders \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "550e8400-e29b-41d4-a716-446655440000",
    "items": [
      {
        "product_id": "660e8400-e29b-41d4-a716-446655440000",
        "quantity": 2,
        "price": 29.99
      }
    ],
    "shipping_address": {
      "street": "123 Main St",
      "city": "San Francisco",
      "state": "CA",
      "zip": "94105"
    }
  }'
```

**Response**:
```json
{
  "order_id": "770e8400-e29b-41d4-a716-446655440000",
  "order_number": "ORD-2024-001",
  "status": "CREATED",
  "message": "Order created successfully"
}
```

---

#### Query the Order (Read)

```bash
# Wait 1-2 seconds for projection to complete

curl http://localhost:8081/api/v1/orders/770e8400-e29b-41d4-a716-446655440000
```

**Response**:
```json
{
  "order_id": "770e8400-e29b-41d4-a716-446655440000",
  "order_number": "ORD-2024-001",
  "customer_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "CREATED",
  "total_amount": 59.98,
  "items": [...],
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

---

#### View Event Stream (Admin)

```bash
curl http://localhost:8082/api/v1/admin/events/770e8400-e29b-41d4-a716-446655440000
```

**Response**:
```json
{
  "aggregate_id": "770e8400-e29b-41d4-a716-446655440000",
  "events": [
    {
      "event_id": "880e8400-e29b-41d4-a716-446655440000",
      "event_type": "OrderCreated",
      "version": 1,
      "timestamp": "2024-01-15T10:30:00Z",
      "data": {...}
    }
  ]
}
```

---

## 🎯 Verify CQRS/ES Principles

### 1. **Separate Write and Read Models**

```bash
# Check command database (event store)
docker exec -it cqrs-postgres-1 psql -U postgres -d eventstore -c \
  "SELECT event_type, aggregate_id FROM events ORDER BY created_at DESC LIMIT 5;"

# Check query database (read model)
docker exec -it cqrs-postgres-1 psql -U postgres -d readmodel -c \
  "SELECT order_id, status, total_amount FROM order_views ORDER BY created_at DESC LIMIT 5;"
```

**Notice**: Different data structures optimized for their purpose!

---

### 2. **Event Sourcing - Immutable Log**

```bash
# Events are append-only (try to modify - should fail)
docker exec -it cqrs-postgres-1 psql -U postgres -d eventstore -c \
  "UPDATE events SET event_type = 'MODIFIED' WHERE event_id = '...'"

# Error: Permission denied (read-only for application)
```

---

### 3. **Event Replay - Rebuild Read Models**

```bash
# Delete read model
curl -X DELETE http://localhost:8082/api/v1/admin/projections/orders

# Replay events to rebuild
curl -X POST http://localhost:8082/api/v1/admin/events/replay

# Query again - data is back!
curl http://localhost:8081/api/v1/orders/770e8400-e29b-41d4-a716-446655440000
```

**This proves**: Event store is the source of truth!

---

## 📊 Monitoring & Observability

### View Metrics (Prometheus)

```bash
# Open Prometheus UI
open http://localhost:9090

# Query examples:
# - command_processing_duration_seconds
# - event_publishing_total
# - projection_lag_seconds
```

---

### View Dashboards (Grafana)

```bash
# Open Grafana
open http://localhost:3000

# Default credentials: admin/admin

# Pre-built dashboards:
# - CQRS System Overview
# - Event Processing Metrics
# - Saga Success Rate
```

---

### Distributed Tracing (Jaeger)

```bash
# Open Jaeger UI
open http://localhost:16686

# Search for traces:
# - Service: command-service
# - Operation: POST /api/v1/orders
```

---

## 🧪 Testing the Saga Pattern

### Scenario: Order with Payment Failure

```bash
# Create order that will fail payment (test card)
curl -X POST http://localhost:8080/api/v1/orders \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "550e8400-e29b-41d4-a716-446655440000",
    "items": [...],
    "payment_method": "test_card_fail"
  }'

# Response:
{
  "order_id": "990e8400-e29b-41d4-a716-446655440000",
  "status": "PAYMENT_FAILED",
  "message": "Order cancelled due to payment failure"
}

# Check event stream - you'll see compensation events:
# 1. OrderCreated
# 2. InventoryReserved
# 3. PaymentFailed
# 4. InventoryReleased (COMPENSATION)
# 5. OrderCancelled (COMPENSATION)
```

**This demonstrates**: Saga pattern with automatic compensation!

---

## 🔧 Development Workflow

### Run Tests

```bash
# Unit tests
make test-unit

# Integration tests
make test-integration

# E2E tests
make test-e2e

# All tests with coverage
make test-all
```

---

### Code Generation

```bash
# Generate gRPC stubs from proto files
make proto-gen

# Generate mocks for testing
make mocks-gen

# Generate API docs (Swagger)
make docs-gen
```

---

### Database Migrations

```bash
# Create new migration
make migrate-create name=add_customer_table

# Apply migrations
make migrate-up

# Rollback last migration
make migrate-down

# Check migration status
make migrate-status
```

---

## 📚 API Documentation

### Swagger UI

```bash
# Open interactive API docs
open http://localhost:8080/swagger/index.html

# Or query API
open http://localhost:8081/swagger/index.html
```

---

### Postman Collection

```bash
# Import collection
open ./docs/api/CQRS_Order_System.postman_collection.json
```

**Includes**:
- Create Order
- Query Order
- Cancel Order
- Process Payment
- Replay Events
- View Metrics

---

## 🐛 Troubleshooting

### Services Not Starting

```bash
# Check container logs
docker-compose logs postgres
docker-compose logs kafka
docker-compose logs redis

# Restart services
docker-compose restart

# Nuclear option: reset everything
make clean && make setup-all
```

---

### Kafka Connection Issues

```bash
# Verify Kafka is healthy
docker exec cqrs-kafka-1 kafka-broker-api-versions --bootstrap-server localhost:9092

# Check topics
docker exec cqrs-kafka-1 kafka-topics --list --bootstrap-server localhost:9092

# View consumer lag
docker exec cqrs-kafka-1 kafka-consumer-groups --bootstrap-server localhost:9092 --describe --group projection-service
```

---

### Database Connection Issues

```bash
# Test connection
docker exec cqrs-postgres-1 psql -U postgres -c "SELECT version();"

# Check database exists
docker exec cqrs-postgres-1 psql -U postgres -c "\l"

# Reset databases
make db-reset
```

---

### Projection Lag (Events Not Appearing in Read Model)

```bash
# Check projection service logs
docker logs cqrs-projection-service-1

# Check consumer lag
curl http://localhost:8082/api/v1/admin/health

# Manually trigger projection rebuild
curl -X POST http://localhost:8082/api/v1/admin/projections/rebuild
```

---

## 📖 Next Steps

### 1. **Explore the Code**
```bash
# Start with domain layer
cat internal/domain/order/aggregate.go

# Then command handlers
cat internal/application/commands/handlers/create_order.go

# Then query handlers
cat internal/application/queries/handlers/get_order.go
```

---

### 2. **Add a New Feature**

Try implementing:
- [ ] Order modification (edit items before confirmation)
- [ ] Order rating/review after delivery
- [ ] Customer loyalty points

Guides in `docs/guides/adding_new_feature.md`

---

### 3. **Performance Testing**

```bash
# Load test with k6
make load-test

# Stress test
make stress-test

# View results in Grafana
```

---

### 4. **Deploy to Production**

```bash
# Build Docker images
make docker-build

# Deploy to Kubernetes
kubectl apply -f deployments/kubernetes/

# Monitor deployment
kubectl get pods -n cqrs-system
```

---

## 🎓 Learning Path

1. **Week 1**: Understand CQRS separation
   - Study `ARCHITECTURE.md`
   - Modify command handlers
   - Add new queries

2. **Week 2**: Master Event Sourcing
   - Study event store implementation
   - Practice event replay
   - Implement new aggregate

3. **Week 3**: Implement Sagas
   - Study saga orchestrator
   - Add compensation logic
   - Handle failure scenarios

4. **Week 4**: Production Readiness
   - Add monitoring
   - Implement circuit breakers
   - Performance optimization

---

## 📞 Getting Help

### Documentation
- Architecture: `ARCHITECTURE.md`
- Requirements: `REQUIREMENTS.md`
- Tech Comparison: `TECH_COMPARISON.md`
- API Docs: http://localhost:8080/swagger

### Common Issues
- Check `docs/troubleshooting.md`
- View FAQ: `docs/faq.md`

### Community
- GitHub Issues: [Report bugs]
- Discussions: [Ask questions]

---

## Installation Guide

### Install Go (1.21+)

#### macOS
```bash
brew install go
```

#### Linux
```bash
wget https://go.dev/dl/go1.21.0.linux-amd64.tar.gz
sudo tar -C /usr/local -xzf go1.21.0.linux-amd64.tar.gz
export PATH=$PATH:/usr/local/go/bin
```

#### Windows
Download from: https://go.dev/dl/

---

### Install Docker

#### macOS
```bash
brew install --cask docker
```

#### Linux
```bash
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
```

#### Windows
Download Docker Desktop: https://www.docker.com/products/docker-desktop

---

### Install Make

#### macOS
```bash
brew install make
```

#### Linux
```bash
sudo apt-get install build-essential  # Ubuntu/Debian
sudo yum install make                 # CentOS/RHEL
```

#### Windows
Use Git Bash or WSL2

---

## 🎉 Congratulations!

You now have a fully functional CQRS/Event Sourcing system running locally!

**What you've learned**:
✅ CQRS pattern with separate command/query models
✅ Event sourcing with immutable event log
✅ Event replay and projection rebuilding
✅ Saga pattern with compensation
✅ Distributed system monitoring

**Ready to build production systems at Netflix/Uber scale!** 🚀
