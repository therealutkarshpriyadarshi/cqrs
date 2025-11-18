.PHONY: help build test clean docker-up docker-down migrate dev check fmt lint

# Default target
help:
	@echo "Available targets:"
	@echo "  build        - Build all crates"
	@echo "  test         - Run all tests"
	@echo "  test-unit    - Run unit tests only"
	@echo "  test-int     - Run integration tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  docker-up    - Start Docker services"
	@echo "  docker-down  - Stop Docker services"
	@echo "  migrate      - Run database migrations"
	@echo "  dev          - Start development environment"
	@echo "  check        - Run cargo check"
	@echo "  fmt          - Format code"
	@echo "  lint         - Run clippy linter"

# Build all crates
build:
	cargo build --all

# Build in release mode
build-release:
	cargo build --all --release

# Run all tests
test:
	cargo test --all

# Run unit tests only
test-unit:
	cargo test --lib --all

# Run integration tests
test-int:
	cargo test --test '*' --all

# Run tests with coverage
test-coverage:
	cargo tarpaulin --all --out Html --output-dir coverage

# Clean build artifacts
clean:
	cargo clean

# Start Docker services
docker-up:
	docker-compose up -d

# Stop Docker services
docker-down:
	docker-compose down

# Stop Docker services and remove volumes
docker-clean:
	docker-compose down -v

# View Docker logs
docker-logs:
	docker-compose logs -f

# Run database migrations
migrate:
	@echo "Waiting for PostgreSQL to be ready..."
	@sleep 2
	@echo "Running migrations..."
	psql postgresql://postgres:postgres@localhost:5432/cqrs_events -f migrations/001_create_events_table.sql
	psql postgresql://postgres:postgres@localhost:5432/cqrs_events -f migrations/002_create_snapshots_table.sql
	@echo "Migrations completed!"

# Start development environment
dev: docker-up
	@echo "Waiting for services to be ready..."
	@sleep 5
	@make migrate
	@echo "Development environment is ready!"

# Run cargo check
check:
	cargo check --all

# Format code
fmt:
	cargo fmt --all

# Check formatting
fmt-check:
	cargo fmt --all -- --check

# Run clippy linter
lint:
	cargo clippy --all -- -D warnings

# Watch for changes and run tests
watch:
	cargo watch -x test

# Watch for changes and run specific crate tests
watch-crate:
	@read -p "Enter crate name: " crate; \
	cargo watch -x "test -p $$crate"

# Generate documentation
doc:
	cargo doc --all --no-deps --open

# Install development dependencies
install-dev:
	cargo install cargo-watch
	cargo install cargo-tarpaulin
	cargo install sqlx-cli

# Database commands
db-create:
	sqlx database create --database-url postgresql://postgres:postgres@localhost:5432/cqrs_events

db-drop:
	sqlx database drop --database-url postgresql://postgres:postgres@localhost:5432/cqrs_events

db-reset: db-drop db-create migrate

# Benchmark
bench:
	cargo bench --all
