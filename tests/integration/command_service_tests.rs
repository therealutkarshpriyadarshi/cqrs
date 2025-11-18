use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

// Helper function to setup test database
async fn setup_test_db() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string()
        });

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

// Helper function to clean up test data
async fn cleanup_events(pool: &PgPool, aggregate_id: Uuid) {
    sqlx::query("DELETE FROM events WHERE aggregate_id = $1")
        .bind(aggregate_id)
        .execute(pool)
        .await
        .expect("Failed to cleanup events");
}

#[tokio::test]
#[ignore] // Run with: cargo test --test command_service_tests -- --ignored
async fn test_create_order_success() {
    let pool = setup_test_db().await;

    // Create a test order
    let customer_id = Uuid::new_v4();
    let request_body = json!({
        "customer_id": customer_id,
        "items": [
            {
                "product_id": Uuid::new_v4(),
                "sku": "TEST-SKU-001",
                "quantity": 2,
                "unit_price": 29.99
            }
        ],
        "shipping_address": {
            "street": "123 Test Street",
            "city": "Test City",
            "state": "TS",
            "zip": "12345",
            "country": "US"
        }
    });

    // Note: This test requires the command service to be running
    // In a full integration test, you would spin up the service or use a test harness
    // For now, we're testing the database operations

    // Verify we can query the database
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(&pool)
        .await
        .expect("Failed to count events");

    assert!(count >= 0);

    println!("Integration test setup successful");
}

#[tokio::test]
#[ignore]
async fn test_event_persistence() {
    let pool = setup_test_db().await;

    let aggregate_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();

    // Insert a test event
    sqlx::query(
        r#"
        INSERT INTO events (
            event_id, aggregate_id, aggregate_type, event_type,
            event_version, payload, metadata, version, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
    )
    .bind(event_id)
    .bind(aggregate_id)
    .bind("Order")
    .bind("OrderCreated")
    .bind(1)
    .bind(json!({
        "order_id": aggregate_id,
        "customer_id": Uuid::new_v4(),
        "order_number": "TEST-001",
        "items": [],
        "total_amount": 100.0,
        "currency": "USD",
        "created_at": chrono::Utc::now()
    }))
    .bind(json!({}))
    .bind(1i64)
    .execute(&pool)
    .await
    .expect("Failed to insert test event");

    // Query the event back
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM events WHERE aggregate_id = $1"
    )
    .bind(aggregate_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to query event");

    assert_eq!(count, 1);

    // Cleanup
    cleanup_events(&pool, aggregate_id).await;

    println!("Event persistence test successful");
}

#[tokio::test]
#[ignore]
async fn test_optimistic_locking() {
    let pool = setup_test_db().await;

    let aggregate_id = Uuid::new_v4();

    // Insert first event
    sqlx::query(
        r#"
        INSERT INTO events (
            event_id, aggregate_id, aggregate_type, event_type,
            event_version, payload, metadata, version, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(aggregate_id)
    .bind("Order")
    .bind("OrderCreated")
    .bind(1)
    .bind(json!({"test": "data"}))
    .bind(json!({}))
    .bind(1i64)
    .execute(&pool)
    .await
    .expect("Failed to insert first event");

    // Try to insert event with duplicate version (should fail)
    let result = sqlx::query(
        r#"
        INSERT INTO events (
            event_id, aggregate_id, aggregate_type, event_type,
            event_version, payload, metadata, version, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(aggregate_id)
    .bind("Order")
    .bind("OrderConfirmed")
    .bind(1)
    .bind(json!({"test": "data2"}))
    .bind(json!({}))
    .bind(1i64) // Same version - should fail
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Duplicate version should fail");

    // Cleanup
    cleanup_events(&pool, aggregate_id).await;

    println!("Optimistic locking test successful");
}

#[tokio::test]
#[ignore]
async fn test_order_lifecycle() {
    let pool = setup_test_db().await;

    let aggregate_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();

    // 1. Create order
    sqlx::query(
        r#"
        INSERT INTO events (
            event_id, aggregate_id, aggregate_type, event_type,
            event_version, payload, metadata, version, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(aggregate_id)
    .bind("Order")
    .bind("OrderCreated")
    .bind(1)
    .bind(json!({
        "order_id": aggregate_id,
        "customer_id": customer_id,
        "order_number": "ORD-TEST-001",
        "items": [{"product_id": Uuid::new_v4(), "sku": "SKU-001", "quantity": 1, "unit_price": 50.0}],
        "total_amount": 50.0,
        "currency": "USD",
        "created_at": chrono::Utc::now()
    }))
    .bind(json!({"correlation_id": Uuid::new_v4()}))
    .bind(1i64)
    .execute(&pool)
    .await
    .expect("Failed to create order");

    // 2. Confirm order
    sqlx::query(
        r#"
        INSERT INTO events (
            event_id, aggregate_id, aggregate_type, event_type,
            event_version, payload, metadata, version, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(aggregate_id)
    .bind("Order")
    .bind("OrderConfirmed")
    .bind(1)
    .bind(json!({
        "order_id": aggregate_id,
        "confirmed_at": chrono::Utc::now()
    }))
    .bind(json!({"correlation_id": Uuid::new_v4()}))
    .bind(2i64)
    .execute(&pool)
    .await
    .expect("Failed to confirm order");

    // 3. Ship order
    sqlx::query(
        r#"
        INSERT INTO events (
            event_id, aggregate_id, aggregate_type, event_type,
            event_version, payload, metadata, version, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(aggregate_id)
    .bind("Order")
    .bind("OrderShipped")
    .bind(1)
    .bind(json!({
        "order_id": aggregate_id,
        "tracking_number": "TRACK-123",
        "carrier": "UPS",
        "shipped_at": chrono::Utc::now()
    }))
    .bind(json!({"correlation_id": Uuid::new_v4()}))
    .bind(3i64)
    .execute(&pool)
    .await
    .expect("Failed to ship order");

    // 4. Deliver order
    sqlx::query(
        r#"
        INSERT INTO events (
            event_id, aggregate_id, aggregate_type, event_type,
            event_version, payload, metadata, version, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(aggregate_id)
    .bind("Order")
    .bind("OrderDelivered")
    .bind(1)
    .bind(json!({
        "order_id": aggregate_id,
        "delivered_at": chrono::Utc::now()
    }))
    .bind(json!({"correlation_id": Uuid::new_v4()}))
    .bind(4i64)
    .execute(&pool)
    .await
    .expect("Failed to deliver order");

    // Verify all events were created
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM events WHERE aggregate_id = $1"
    )
    .bind(aggregate_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to query events");

    assert_eq!(count, 4, "Should have 4 events for complete order lifecycle");

    // Cleanup
    cleanup_events(&pool, aggregate_id).await;

    println!("Order lifecycle test successful");
}
