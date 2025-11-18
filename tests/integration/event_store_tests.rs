use event_store::{Event, EventStore, PostgresEventStore};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper function to create a test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/cqrs_events".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Helper function to clean up test data
async fn cleanup_aggregate(pool: &PgPool, aggregate_id: Uuid) {
    sqlx::query("DELETE FROM events WHERE aggregate_id = $1")
        .bind(aggregate_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore] // Run with: cargo test --test event_store_tests -- --ignored
async fn test_append_and_load_single_event() {
    let pool = create_test_pool().await;
    let store = PostgresEventStore::new(pool.clone());
    let aggregate_id = Uuid::new_v4();

    // Create a test event
    let event = Event::new(
        aggregate_id,
        "Order".to_string(),
        "OrderCreated".to_string(),
        1,
        json!({
            "order_id": aggregate_id.to_string(),
            "customer_id": Uuid::new_v4().to_string(),
            "total_amount": 100.50
        }),
        json!({
            "correlation_id": Uuid::new_v4().to_string()
        }),
    );

    // Append event
    let result = store.append_events(aggregate_id, 0, vec![event.clone()]).await;
    assert!(result.is_ok());

    // Load events
    let loaded_events = store.load_events(aggregate_id).await.unwrap();
    assert_eq!(loaded_events.len(), 1);
    assert_eq!(loaded_events[0].aggregate_id, aggregate_id);
    assert_eq!(loaded_events[0].event_type, "OrderCreated");

    // Cleanup
    cleanup_aggregate(&pool, aggregate_id).await;
}

#[tokio::test]
#[ignore]
async fn test_append_multiple_events() {
    let pool = create_test_pool().await;
    let store = PostgresEventStore::new(pool.clone());
    let aggregate_id = Uuid::new_v4();

    // Create multiple events
    let events = vec![
        Event::new(
            aggregate_id,
            "Order".to_string(),
            "OrderCreated".to_string(),
            1,
            json!({"status": "created"}),
            json!({}),
        ),
        Event::new(
            aggregate_id,
            "Order".to_string(),
            "OrderConfirmed".to_string(),
            1,
            json!({"status": "confirmed"}),
            json!({}),
        ),
    ];

    // Append events
    let result = store.append_events(aggregate_id, 0, events).await;
    assert!(result.is_ok());

    // Load events
    let loaded_events = store.load_events(aggregate_id).await.unwrap();
    assert_eq!(loaded_events.len(), 2);
    assert_eq!(loaded_events[0].event_type, "OrderCreated");
    assert_eq!(loaded_events[1].event_type, "OrderConfirmed");
    assert_eq!(loaded_events[0].sequence_number, 1);
    assert_eq!(loaded_events[1].sequence_number, 2);

    // Cleanup
    cleanup_aggregate(&pool, aggregate_id).await;
}

#[tokio::test]
#[ignore]
async fn test_optimistic_concurrency_control() {
    let pool = create_test_pool().await;
    let store = PostgresEventStore::new(pool.clone());
    let aggregate_id = Uuid::new_v4();

    // Append first event
    let event1 = Event::new(
        aggregate_id,
        "Order".to_string(),
        "OrderCreated".to_string(),
        1,
        json!({"status": "created"}),
        json!({}),
    );
    store.append_events(aggregate_id, 0, vec![event1]).await.unwrap();

    // Try to append with wrong expected version (should fail)
    let event2 = Event::new(
        aggregate_id,
        "Order".to_string(),
        "OrderConfirmed".to_string(),
        1,
        json!({"status": "confirmed"}),
        json!({}),
    );
    let result = store.append_events(aggregate_id, 0, vec![event2]).await;
    assert!(result.is_err());

    // Append with correct version (should succeed)
    let event3 = Event::new(
        aggregate_id,
        "Order".to_string(),
        "OrderConfirmed".to_string(),
        1,
        json!({"status": "confirmed"}),
        json!({}),
    );
    let result = store.append_events(aggregate_id, 1, vec![event3]).await;
    assert!(result.is_ok());

    // Verify we have 2 events
    let loaded_events = store.load_events(aggregate_id).await.unwrap();
    assert_eq!(loaded_events.len(), 2);

    // Cleanup
    cleanup_aggregate(&pool, aggregate_id).await;
}

#[tokio::test]
#[ignore]
async fn test_load_events_from_version() {
    let pool = create_test_pool().await;
    let store = PostgresEventStore::new(pool.clone());
    let aggregate_id = Uuid::new_v4();

    // Append 3 events
    let events = vec![
        Event::new(
            aggregate_id,
            "Order".to_string(),
            "OrderCreated".to_string(),
            1,
            json!({"status": "created"}),
            json!({}),
        ),
        Event::new(
            aggregate_id,
            "Order".to_string(),
            "OrderConfirmed".to_string(),
            1,
            json!({"status": "confirmed"}),
            json!({}),
        ),
        Event::new(
            aggregate_id,
            "Order".to_string(),
            "OrderShipped".to_string(),
            1,
            json!({"status": "shipped"}),
            json!({}),
        ),
    ];
    store.append_events(aggregate_id, 0, events).await.unwrap();

    // Load events from version 1
    let loaded_events = store
        .load_events_from_version(aggregate_id, 1)
        .await
        .unwrap();
    assert_eq!(loaded_events.len(), 2);
    assert_eq!(loaded_events[0].event_type, "OrderConfirmed");
    assert_eq!(loaded_events[1].event_type, "OrderShipped");

    // Cleanup
    cleanup_aggregate(&pool, aggregate_id).await;
}

#[tokio::test]
#[ignore]
async fn test_get_current_version() {
    let pool = create_test_pool().await;
    let store = PostgresEventStore::new(pool.clone());
    let aggregate_id = Uuid::new_v4();

    // Initially, version should be 0
    let version = store.get_current_version(aggregate_id).await.unwrap();
    assert_eq!(version, 0);

    // Append one event
    let event = Event::new(
        aggregate_id,
        "Order".to_string(),
        "OrderCreated".to_string(),
        1,
        json!({"status": "created"}),
        json!({}),
    );
    store.append_events(aggregate_id, 0, vec![event]).await.unwrap();

    // Version should be 1
    let version = store.get_current_version(aggregate_id).await.unwrap();
    assert_eq!(version, 1);

    // Cleanup
    cleanup_aggregate(&pool, aggregate_id).await;
}

#[tokio::test]
#[ignore]
async fn test_event_ordering() {
    let pool = create_test_pool().await;
    let store = PostgresEventStore::new(pool.clone());
    let aggregate_id = Uuid::new_v4();

    // Append events in order
    for i in 0..5 {
        let event = Event::new(
            aggregate_id,
            "Order".to_string(),
            format!("Event{}", i),
            1,
            json!({"index": i}),
            json!({}),
        );
        store.append_events(aggregate_id, i, vec![event]).await.unwrap();
    }

    // Load and verify ordering
    let loaded_events = store.load_events(aggregate_id).await.unwrap();
    assert_eq!(loaded_events.len(), 5);

    for (i, event) in loaded_events.iter().enumerate() {
        assert_eq!(event.event_type, format!("Event{}", i));
        assert_eq!(event.sequence_number, (i + 1) as i64);
    }

    // Cleanup
    cleanup_aggregate(&pool, aggregate_id).await;
}
