use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::{Result, SagaError};
use crate::saga::{SagaState, SagaStatus};

/// Saga instance as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SagaInstance {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub current_step: i32,
    pub state: serde_json::Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SagaInstance {
    pub fn from_saga_state(state: &SagaState) -> Result<Self> {
        Ok(Self {
            saga_id: state.saga_id,
            saga_type: state.saga_type.clone(),
            current_step: state.current_step as i32,
            state: serde_json::to_value(state)?,
            status: state.status.to_string(),
            created_at: state.created_at,
            updated_at: state.updated_at,
        })
    }

    pub fn to_saga_state(&self) -> Result<SagaState> {
        Ok(serde_json::from_value(self.state.clone())?)
    }
}

/// Repository for persisting saga state
#[async_trait]
pub trait SagaRepository: Send + Sync {
    /// Save a new saga instance
    async fn save(&self, state: &SagaState) -> Result<()>;

    /// Update an existing saga instance
    async fn update(&self, state: &SagaState) -> Result<()>;

    /// Load a saga instance by ID
    async fn load(&self, saga_id: Uuid) -> Result<SagaState>;

    /// Find sagas by status
    async fn find_by_status(&self, status: SagaStatus, limit: i64) -> Result<Vec<SagaState>>;

    /// Delete a saga instance
    async fn delete(&self, saga_id: Uuid) -> Result<()>;
}

/// PostgreSQL implementation of SagaRepository
pub struct PostgresSagaRepository {
    pool: PgPool,
}

impl PostgresSagaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SagaRepository for PostgresSagaRepository {
    async fn save(&self, state: &SagaState) -> Result<()> {
        let instance = SagaInstance::from_saga_state(state)?;

        sqlx::query(
            r#"
            INSERT INTO saga_instances (
                saga_id, saga_type, current_step, state, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(instance.saga_id)
        .bind(&instance.saga_type)
        .bind(instance.current_step)
        .bind(&instance.state)
        .bind(&instance.status)
        .bind(instance.created_at)
        .bind(instance.updated_at)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            saga_id = %state.saga_id,
            saga_type = %state.saga_type,
            "Saga instance saved"
        );

        Ok(())
    }

    async fn update(&self, state: &SagaState) -> Result<()> {
        let instance = SagaInstance::from_saga_state(state)?;

        let result = sqlx::query(
            r#"
            UPDATE saga_instances
            SET current_step = $2, state = $3, status = $4, updated_at = $5
            WHERE saga_id = $1
            "#,
        )
        .bind(instance.saga_id)
        .bind(instance.current_step)
        .bind(&instance.state)
        .bind(&instance.status)
        .bind(instance.updated_at)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(SagaError::SagaNotFound(state.saga_id.to_string()));
        }

        tracing::debug!(
            saga_id = %state.saga_id,
            status = %state.status,
            current_step = state.current_step,
            "Saga instance updated"
        );

        Ok(())
    }

    async fn load(&self, saga_id: Uuid) -> Result<SagaState> {
        let instance: SagaInstance = sqlx::query_as(
            r#"
            SELECT saga_id, saga_type, current_step, state, status, created_at, updated_at
            FROM saga_instances
            WHERE saga_id = $1
            "#,
        )
        .bind(saga_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| SagaError::SagaNotFound(saga_id.to_string()))?;

        instance.to_saga_state()
    }

    async fn find_by_status(&self, status: SagaStatus, limit: i64) -> Result<Vec<SagaState>> {
        let instances: Vec<SagaInstance> = sqlx::query_as(
            r#"
            SELECT saga_id, saga_type, current_step, state, status, created_at, updated_at
            FROM saga_instances
            WHERE status = $1
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(status.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        instances
            .iter()
            .map(|i| i.to_saga_state())
            .collect()
    }

    async fn delete(&self, saga_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM saga_instances WHERE saga_id = $1")
            .bind(saga_id)
            .execute(&self.pool)
            .await?;

        tracing::info!(saga_id = %saga_id, "Saga instance deleted");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::SagaStep;

    #[test]
    fn test_saga_instance_conversion() {
        let saga_id = Uuid::new_v4();
        let steps = vec![
            SagaStep::new("step1".to_string(), 3),
            SagaStep::new("step2".to_string(), 3),
        ];
        let data = serde_json::json!({"order_id": "123"});

        let state = SagaState::new(saga_id, "test_saga".to_string(), steps, data);
        let instance = SagaInstance::from_saga_state(&state).unwrap();

        assert_eq!(instance.saga_id, saga_id);
        assert_eq!(instance.saga_type, "test_saga");
        assert_eq!(instance.status, "RUNNING");
        assert_eq!(instance.current_step, 0);

        let restored_state = instance.to_saga_state().unwrap();
        assert_eq!(restored_state.saga_id, state.saga_id);
        assert_eq!(restored_state.saga_type, state.saga_type);
        assert_eq!(restored_state.status, state.status);
    }
}
