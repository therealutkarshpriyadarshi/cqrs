# Development Guide

## Quick Start

### 1. Initial Setup

```bash
# Copy environment file
cp .env.example .env

# Start infrastructure (PostgreSQL, Kafka, Redis)
make dev

# This will:
# - Start Docker services
# - Run database migrations
# - Prepare development environment
```

### 2. Build & Test

```bash
# Build all crates
make build

# Run unit tests (fast, no dependencies)
make test-unit

# Run integration tests (requires Docker)
make test-int

# Run all tests
make test
```

### 3. Code Quality

```bash
# Format code
make fmt

# Run linter
make lint

# Check compilation
make check
```

## Development Workflow

### Working on a Feature

1. **Create a branch**:
```bash
git checkout -b feature/your-feature
```

2. **Start development environment**:
```bash
make dev
```

3. **Write code with tests**:
```bash
# Watch mode - auto-run tests on changes
make watch
```

4. **Verify changes**:
```bash
make fmt
make lint
make test
```

5. **Commit and push**:
```bash
git add .
git commit -m "feat: your feature description"
git push origin feature/your-feature
```

## Project Structure

```
cqrs/
├── crates/                  # Rust crates
│   ├── domain/             # Core business logic
│   ├── event-store/        # Event persistence
│   └── common/             # Shared utilities
├── migrations/             # Database migrations
├── tests/                  # Integration tests
│   └── integration/
├── docs/                   # Documentation
│   ├── PHASE1.md          # Phase 1 implementation
│   └── DEVELOPMENT.md     # This file
├── docker-compose.yml      # Infrastructure
├── Makefile               # Convenient commands
└── Cargo.toml             # Workspace configuration
```

## Available Make Targets

### Building
- `make build` - Build all crates
- `make build-release` - Build optimized release
- `make clean` - Clean build artifacts

### Testing
- `make test` - Run all tests
- `make test-unit` - Run unit tests only
- `make test-int` - Run integration tests
- `make watch` - Auto-run tests on changes

### Docker
- `make docker-up` - Start services
- `make docker-down` - Stop services
- `make docker-clean` - Stop and remove volumes
- `make docker-logs` - View logs

### Database
- `make migrate` - Run migrations
- `make db-create` - Create database
- `make db-drop` - Drop database
- `make db-reset` - Reset database

### Code Quality
- `make fmt` - Format code
- `make fmt-check` - Check formatting
- `make lint` - Run clippy
- `make check` - Run cargo check

### Development
- `make dev` - Start development environment
- `make doc` - Generate and open documentation

## Testing

### Unit Tests

Located within source files (`#[cfg(test)]`):

```bash
# Run all unit tests
cargo test --lib --all

# Run tests for specific crate
cargo test -p domain --lib

# Run specific test
cargo test test_create_order_success
```

### Integration Tests

Located in `tests/integration/`:

```bash
# Start required services
make docker-up
make migrate

# Run integration tests
cargo test --test '*' --all

# Run with output
cargo test --test event_store_tests -- --ignored --nocapture
```

### Writing Tests

**Unit Test Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_creation() {
        let items = vec![OrderItem::new(...)];
        let result = OrderAggregate::create(customer_id, items);
        assert!(result.is_ok());
    }
}
```

**Integration Test Example**:
```rust
#[tokio::test]
#[ignore] // Requires database
async fn test_event_store() {
    let pool = create_test_pool().await;
    let store = PostgresEventStore::new(pool);
    // ... test logic
}
```

## Database

### Accessing PostgreSQL

```bash
# Via Docker
docker exec -it cqrs-postgres psql -U postgres -d cqrs_events

# Via psql
psql postgresql://postgres:postgres@localhost:5432/cqrs_events
```

### Common Queries

```sql
-- View all events
SELECT * FROM events ORDER BY created_at DESC LIMIT 10;

-- View events for specific aggregate
SELECT * FROM events WHERE aggregate_id = 'your-uuid' ORDER BY version;

-- Count events by type
SELECT event_type, COUNT(*) FROM events GROUP BY event_type;

-- View event payload
SELECT event_type, payload FROM events;
```

### Migrations

Add new migration:
```bash
# Create new SQL file
cat > migrations/003_new_migration.sql << 'EOF'
-- Your SQL here
EOF

# Run migration
make migrate
```

## Debugging

### Enable Detailed Logging

```bash
# Set in .env
RUST_LOG=debug

# Or run with env var
RUST_LOG=debug cargo test test_name
```

### Logging Levels
- `error` - Errors only
- `warn` - Warnings and errors
- `info` - Info, warnings, errors (default)
- `debug` - Debug and above
- `trace` - Everything

### Module-Specific Logging

```bash
# Debug only event-store crate
RUST_LOG=event_store=debug cargo test

# Debug multiple modules
RUST_LOG=event_store=debug,domain=trace cargo test
```

### Database Debugging

```bash
# View event store operations
RUST_LOG=event_store=debug,sqlx=debug cargo test
```

## IDE Setup

### VSCode

Recommended extensions:
- rust-analyzer
- CodeLLDB
- Better TOML
- Even Better TOML

`.vscode/settings.json`:
```json
{
  "rust-analyzer.cargo.allFeatures": true,
  "rust-analyzer.checkOnSave.command": "clippy",
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  }
}
```

### IntelliJ IDEA / CLion

1. Install Rust plugin
2. Open project directory
3. Configure Rust toolchain in settings
4. Enable "Run clippy on save"

## Common Issues

### Port Already in Use

```bash
# Find process using port 5432
lsof -i :5432

# Stop conflicting service
brew services stop postgresql  # macOS
sudo systemctl stop postgresql # Linux
```

### Database Connection Failed

```bash
# Restart Docker services
make docker-down
make docker-up

# Wait for PostgreSQL to be ready
sleep 5

# Run migrations
make migrate
```

### Tests Hanging

```bash
# Check if database is accessible
docker ps | grep postgres

# Check logs
make docker-logs

# Reset environment
make docker-clean
make dev
```

### Cargo Build Slow

```bash
# Use cargo-watch for incremental builds
cargo install cargo-watch
make watch

# Or use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache
```

## Performance Tips

### Faster Compilation

1. **Use mold linker** (Linux):
```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

2. **Enable incremental compilation**:
```bash
export CARGO_INCREMENTAL=1
```

3. **Use cargo-watch**:
```bash
cargo watch -x test
```

### Faster Tests

```bash
# Run tests in parallel
cargo test -- --test-threads=4

# Skip integration tests during development
cargo test --lib

# Run specific test
cargo test test_name
```

## Git Workflow

### Commit Messages

Follow conventional commits:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `test:` - Tests
- `refactor:` - Code refactoring
- `perf:` - Performance improvement
- `chore:` - Maintenance

Examples:
```
feat: add order cancellation logic
fix: prevent concurrent event append
docs: update Phase 1 documentation
test: add integration tests for event store
```

### Pre-commit Checks

Create `.git/hooks/pre-commit`:
```bash
#!/bin/bash
make fmt-check
make lint
make test-unit
```

```bash
chmod +x .git/hooks/pre-commit
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [CQRS Pattern](https://martinfowler.com/bliki/CQRS.html)
- [Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html)

## Getting Help

1. Check documentation in `docs/`
2. Review test examples in `tests/`
3. Read inline code comments
4. Check GitHub issues
5. Ask the team

## Next Steps

After completing Phase 1:
1. Review [PHASE1.md](PHASE1.md)
2. Check [RUST_ROADMAP.md](../RUST_ROADMAP.md) for Phase 2
3. Start implementing command service
