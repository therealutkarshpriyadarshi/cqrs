# CQRS & Event Sourcing Order Processing System

**Production-grade order processing system demonstrating Command Query Responsibility Segregation (CQRS) and Event Sourcing patterns used by Netflix, Uber, and Amazon.**

[![Go Version](https://img.shields.io/badge/Go-1.21+-00ADD8?style=flat&logo=go)](https://go.dev)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

---

## 🎯 What is This?

This project implements a complete **event-sourced microservices architecture** for order processing, demonstrating:

- **CQRS Pattern**: Separate write (command) and read (query) models for independent scaling
- **Event Sourcing**: Immutable event log as the single source of truth
- **Saga Pattern**: Distributed transactions with automatic compensation
- **Event-Driven Architecture**: Asynchronous, loosely-coupled services
- **Eventual Consistency**: Handling consistency in distributed systems

**Why build this?** These patterns power the core systems at:
- Netflix (230M+ subscribers, viewing history)
- Uber (millions of events/day, ride matching)
- Amazon (order processing pipeline)
- Capital One (financial transactions, compliance)

---

## 🏗️ System Architecture

```
┌─────────────┐         ┌─────────────┐
│   Command   │         │    Query    │
│   Service   │         │   Service   │
│   (Write)   │         │    (Read)   │
└──────┬──────┘         └──────▲──────┘
       │                       │
       │ Events                │ Projections
       │                       │
┌──────▼───────────────────────┴──────┐
│         Event Store (Kafka)         │
│      PostgreSQL Event Persistence   │
└──────┬──────────────────────────────┘
       │
       │ Event Subscribers
       │
   ┌───┴────┬──────────┬─────────┐
   │        │          │         │
┌──▼──┐ ┌──▼───┐ ┌────▼──┐ ┌───▼────┐
│Proj │ │ Saga │ │ Read  │ │ Notify │
│     │ │      │ │ Model │ │        │
└─────┘ └──────┘ └───────┘ └────────┘
                    (Redis)
```

---

## ✨ Key Features

### Core Patterns
- ✅ **Separate Command/Query Models** - Independent scaling and optimization
- ✅ **Immutable Event Log** - Complete audit trail, temporal queries
- ✅ **Event Replay** - Rebuild read models from events
- ✅ **Saga Orchestration** - Distributed transactions with compensation
- ✅ **Event Versioning** - Schema evolution without breaking changes

### Production Features
- ✅ **gRPC** - High-performance inter-service communication
- ✅ **Distributed Tracing** - Jaeger integration
- ✅ **Metrics & Monitoring** - Prometheus + Grafana
- ✅ **API Documentation** - Auto-generated Swagger docs
- ✅ **Idempotency** - Handle duplicate message processing
- ✅ **Circuit Breakers** - Fault tolerance and resilience

---

## 🚀 Quick Start

### Prerequisites
- **Go 1.21+** (or Python 3.11+)
- **Docker** & **Docker Compose**
- **Make**

### Get Running in 3 Commands

```bash
# 1. Start infrastructure (PostgreSQL, Kafka, Redis)
docker-compose up -d

# 2. Run database migrations & create Kafka topics
make setup

# 3. Start all services
make run
```

**Services running on**:
- Command API: http://localhost:8080
- Query API: http://localhost:8081
- Admin API: http://localhost:8082
- Grafana: http://localhost:3000
- Jaeger: http://localhost:16686

### Test It Out

```bash
# Create an order (Command)
curl -X POST http://localhost:8080/api/v1/orders \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "550e8400-e29b-41d4-a716-446655440000",
    "items": [{"product_id": "...", "quantity": 2, "price": 29.99}]
  }'

# Query the order (Read) - notice separate endpoint!
curl http://localhost:8081/api/v1/orders/{order_id}

# View event stream (Event Sourcing)
curl http://localhost:8082/api/v1/admin/events/{order_id}
```

**See full guide**: [QUICKSTART.md](QUICKSTART.md)

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| **[ARCHITECTURE.md](ARCHITECTURE.md)** | Complete system design, event schemas, database design |
| **[REQUIREMENTS.md](REQUIREMENTS.md)** | Comprehensive checklist of components needed |
| **[TECH_COMPARISON.md](TECH_COMPARISON.md)** | Go vs Python detailed comparison for CQRS/ES |
| **[QUICKSTART.md](QUICKSTART.md)** | Step-by-step setup and testing guide |

---

## 🛠️ Technology Stack

### Core Services (Go)
- **Web Framework**: Gin
- **Event Streaming**: Apache Kafka
- **Event Store**: PostgreSQL
- **Read Model Cache**: Redis
- **Inter-Service**: gRPC + Protocol Buffers

### Infrastructure
- **Message Broker**: Kafka + Zookeeper
- **Databases**: PostgreSQL (event store + read models)
- **Monitoring**: Prometheus + Grafana
- **Tracing**: Jaeger
- **Containers**: Docker + Kubernetes

---

## 📁 Project Structure

```
cqrs/
├── cmd/                          # Service entry points
│   ├── command-service/          # Write operations
│   ├── query-service/            # Read operations
│   ├── projection-service/       # Event projection engine
│   └── saga-orchestrator/        # Distributed transactions
├── internal/
│   ├── domain/                   # Business logic & aggregates
│   │   ├── order/                # Order aggregate, events
│   │   ├── payment/              # Payment aggregate
│   │   └── inventory/            # Inventory aggregate
│   ├── application/
│   │   ├── commands/handlers/    # Command handlers
│   │   ├── queries/handlers/     # Query handlers
│   │   └── sagas/                # Saga coordinators
│   ├── infrastructure/
│   │   ├── eventstore/           # Event persistence
│   │   ├── messaging/kafka/      # Event publishing
│   │   └── cache/redis/          # Read model caching
│   └── interfaces/
│       ├── http/                 # REST APIs
│       └── grpc/                 # gRPC services
├── deployments/
│   ├── docker/                   # Docker Compose
│   └── kubernetes/               # K8s manifests
├── docs/                         # Additional documentation
└── tests/                        # Integration & E2E tests
```

---

## 🎓 Learning Outcomes

After completing this project, you'll master:

1. **CQRS Pattern**
   - Separating reads and writes
   - Independent scaling strategies
   - Optimizing for different access patterns

2. **Event Sourcing**
   - Event as source of truth
   - Event replay and time travel
   - Snapshot optimization

3. **Event-Driven Architecture**
   - Asynchronous processing
   - Eventual consistency
   - Event choreography vs orchestration

4. **Saga Pattern**
   - Distributed transactions
   - Compensation logic
   - Failure handling

5. **Domain-Driven Design**
   - Aggregate boundaries
   - Ubiquitous language
   - Bounded contexts

6. **Production Practices**
   - Distributed tracing
   - Metrics and monitoring
   - API versioning
   - Schema evolution

---

## 🏢 Real-World Usage

These patterns are used by:

| Company | Use Case | Scale |
|---------|----------|-------|
| **Netflix** | Viewing history, recommendations | 230M+ subscribers |
| **Uber** | Ride matching, trip events | Millions of events/day |
| **Amazon** | Order processing | Billions of orders |
| **Capital One** | Financial transactions | Compliance & audit |

---

## 🧪 Testing

```bash
# Unit tests
make test-unit

# Integration tests (requires Docker)
make test-integration

# E2E tests
make test-e2e

# Load testing
make load-test

# All tests with coverage
make test-all
```

---

## 📊 Monitoring

### Metrics (Grafana)
- Command processing latency
- Event publishing rate
- Projection lag time
- Saga success rate
- Cache hit ratio

### Tracing (Jaeger)
- End-to-end request tracing
- Service dependency visualization
- Performance bottleneck identification

---

## 🚢 Deployment

### Local Development
```bash
docker-compose up -d
make run
```

### Production (Kubernetes)
```bash
# Build images
make docker-build

# Deploy to K8s
kubectl apply -f deployments/kubernetes/

# Monitor
kubectl get pods -n cqrs-system
```

---

## 🔧 Development

### Add New Feature
```bash
# 1. Create new event type
# internal/domain/order/events.go

# 2. Add command handler
# internal/application/commands/handlers/

# 3. Add query handler
# internal/application/queries/handlers/

# 4. Add projection
# cmd/projection-service/

# 5. Update API
# internal/interfaces/http/
```

See: `docs/guides/adding_new_feature.md`

---

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md)

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing`)
5. Open Pull Request

---

## 📖 Further Reading

### CQRS & Event Sourcing
- [Martin Fowler - CQRS](https://martinfowler.com/bliki/CQRS.html)
- [Greg Young - Event Sourcing](https://www.eventstore.com/blog/what-is-event-sourcing)
- [Microservices.io - Saga Pattern](https://microservices.io/patterns/data/saga.html)

### Kafka & Event Streaming
- [Confluent - Event Sourcing with Kafka](https://www.confluent.io/blog/event-sourcing-cqrs-stream-processing-apache-kafka-whats-connection/)
- [Building Event-Driven Microservices](https://www.oreilly.com/library/view/building-event-driven-microservices/9781492057888/)

---

## 📄 License

MIT License - see [LICENSE](LICENSE)

---

## 🎯 Next Steps

1. **Read the docs**: Start with [ARCHITECTURE.md](ARCHITECTURE.md)
2. **Choose your stack**: Review [TECH_COMPARISON.md](TECH_COMPARISON.md)
3. **Get started**: Follow [QUICKSTART.md](QUICKSTART.md)
4. **Build features**: Implement your own aggregates and sagas!

**Questions?** Open an issue or start a discussion!

---

**Built with ❤️ using CQRS, Event Sourcing, and modern microservices patterns**
