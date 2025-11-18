use chrono::Utc;
use domain::events::order_events::*;
use read_model::{OrderProjection, OrderView, OrderViewRepository, PostgresOrderViewRepository};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
#[ignore] // Requires database to be running
async fn test_order_projection_created() {
    // Setup database connection
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string()
        });
    let pool = PgPool::connect(&database_url).await.unwrap();

    // Create projection
    let projection = OrderProjection::new(pool.clone());
    let repository = PostgresOrderViewRepository::new(pool.clone());

    // Create test event
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let event = OrderCreatedEvent {
        order_id,
        customer_id,
        order_number: format!("ORD-{}", Uuid::new_v4().simple()),
        items: vec![OrderItem {
            product_id: Uuid::new_v4(),
            sku: "TEST-SKU-001".to_string(),
            quantity: 2,
            unit_price: 50.0,
        }],
        total_amount: 100.0,
        currency: "USD".to_string(),
        created_at: Utc::now(),
    };

    // Handle event
    projection.handle_order_created(&event).await.unwrap();

    // Verify projection was created
    let order = repository.get_by_id(order_id).await.unwrap();
    assert!(order.is_some());

    let order = order.unwrap();
    assert_eq!(order.order_id, order_id);
    assert_eq!(order.customer_id, customer_id);
    assert_eq!(order.status, "CREATED");
    assert_eq!(order.total_amount, 100.0);

    // Cleanup
    sqlx::query("DELETE FROM order_views WHERE order_id = $1")
        .bind(order_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore] // Requires database to be running
async fn test_order_projection_lifecycle() {
    // Setup database connection
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string()
        });
    let pool = PgPool::connect(&database_url).await.unwrap();

    let projection = OrderProjection::new(pool.clone());
    let repository = PostgresOrderViewRepository::new(pool.clone());

    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();

    // 1. Create order
    let created_event = OrderCreatedEvent {
        order_id,
        customer_id,
        order_number: format!("ORD-{}", Uuid::new_v4().simple()),
        items: vec![],
        total_amount: 100.0,
        currency: "USD".to_string(),
        created_at: Utc::now(),
    };
    projection.handle_order_created(&created_event).await.unwrap();

    let order = repository.get_by_id(order_id).await.unwrap().unwrap();
    assert_eq!(order.status, "CREATED");

    // 2. Confirm order
    let confirmed_event = OrderConfirmedEvent {
        order_id,
        confirmed_at: Utc::now(),
    };
    projection.handle_order_confirmed(&confirmed_event).await.unwrap();

    let order = repository.get_by_id(order_id).await.unwrap().unwrap();
    assert_eq!(order.status, "CONFIRMED");

    // 3. Ship order
    let shipped_event = OrderShippedEvent {
        order_id,
        tracking_number: "TRACK123".to_string(),
        carrier: "UPS".to_string(),
        shipped_at: Utc::now(),
    };
    projection.handle_order_shipped(&shipped_event).await.unwrap();

    let order = repository.get_by_id(order_id).await.unwrap().unwrap();
    assert_eq!(order.status, "SHIPPED");
    assert_eq!(order.tracking_number.as_deref(), Some("TRACK123"));
    assert_eq!(order.carrier.as_deref(), Some("UPS"));

    // 4. Deliver order
    let delivered_event = OrderDeliveredEvent {
        order_id,
        delivered_at: Utc::now(),
    };
    projection.handle_order_delivered(&delivered_event).await.unwrap();

    let order = repository.get_by_id(order_id).await.unwrap().unwrap();
    assert_eq!(order.status, "DELIVERED");

    // Cleanup
    sqlx::query("DELETE FROM order_views WHERE order_id = $1")
        .bind(order_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore] // Requires database to be running
async fn test_repository_list_by_customer() {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string()
        });
    let pool = PgPool::connect(&database_url).await.unwrap();

    let projection = OrderProjection::new(pool.clone());
    let repository = PostgresOrderViewRepository::new(pool.clone());

    let customer_id = Uuid::new_v4();

    // Create 3 orders for the same customer
    let mut order_ids = vec![];
    for i in 0..3 {
        let order_id = Uuid::new_v4();
        order_ids.push(order_id);

        let event = OrderCreatedEvent {
            order_id,
            customer_id,
            order_number: format!("ORD-{}", i),
            items: vec![],
            total_amount: (i as f64 + 1.0) * 100.0,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        };

        projection.handle_order_created(&event).await.unwrap();
    }

    // List orders
    let orders = repository.list_by_customer(customer_id, 10, 0).await.unwrap();
    assert_eq!(orders.len(), 3);

    // Count orders
    let count = repository.count_by_customer(customer_id).await.unwrap();
    assert_eq!(count, 3);

    // Cleanup
    for order_id in order_ids {
        sqlx::query("DELETE FROM order_views WHERE order_id = $1")
            .bind(order_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore] // Requires database to be running
async fn test_repository_list_by_status() {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string()
        });
    let pool = PgPool::connect(&database_url).await.unwrap();

    let projection = OrderProjection::new(pool.clone());
    let repository = PostgresOrderViewRepository::new(pool.clone());

    // Create orders
    let mut order_ids = vec![];
    for i in 0..2 {
        let order_id = Uuid::new_v4();
        order_ids.push(order_id);

        let event = OrderCreatedEvent {
            order_id,
            customer_id: Uuid::new_v4(),
            order_number: format!("ORD-STATUS-{}", i),
            items: vec![],
            total_amount: 100.0,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        };

        projection.handle_order_created(&event).await.unwrap();
    }

    // List by status
    let orders = repository.list_by_status("CREATED", 10, 0).await.unwrap();
    assert!(orders.len() >= 2);

    // Cleanup
    for order_id in order_ids {
        sqlx::query("DELETE FROM order_views WHERE order_id = $1")
            .bind(order_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore] // Requires database to be running
async fn test_repository_search_by_order_number() {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string()
        });
    let pool = PgPool::connect(&database_url).await.unwrap();

    let projection = OrderProjection::new(pool.clone());
    let repository = PostgresOrderViewRepository::new(pool.clone());

    let order_id = Uuid::new_v4();
    let order_number = format!("ORD-SEARCH-{}", Uuid::new_v4().simple());

    let event = OrderCreatedEvent {
        order_id,
        customer_id: Uuid::new_v4(),
        order_number: order_number.clone(),
        items: vec![],
        total_amount: 100.0,
        currency: "USD".to_string(),
        created_at: Utc::now(),
    };

    projection.handle_order_created(&event).await.unwrap();

    // Search by order number
    let order = repository
        .search_by_order_number(&order_number)
        .await
        .unwrap();
    assert!(order.is_some());
    assert_eq!(order.unwrap().order_number, order_number);

    // Cleanup
    sqlx::query("DELETE FROM order_views WHERE order_id = $1")
        .bind(order_id)
        .execute(&pool)
        .await
        .unwrap();
}
