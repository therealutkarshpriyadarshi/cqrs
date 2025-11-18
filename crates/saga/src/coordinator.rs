use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::errors::{Result, SagaError};
use crate::repository::SagaRepository;
use crate::saga::{Saga, SagaState, SagaStatus};

/// Saga coordinator that orchestrates saga execution
pub struct SagaCoordinator<R: SagaRepository> {
    repository: Arc<R>,
}

impl<R: SagaRepository> SagaCoordinator<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Start a new saga
    pub async fn start_saga(
        &self,
        saga: &dyn Saga,
        saga_id: Uuid,
        data: serde_json::Value,
    ) -> Result<SagaState> {
        info!(
            saga_id = %saga_id,
            saga_type = saga.saga_type(),
            "Starting new saga"
        );

        let state = saga.create_state(saga_id, data).await?;
        self.repository.save(&state).await?;

        Ok(state)
    }

    /// Execute the next step of a saga
    pub async fn execute_step(
        &self,
        saga: &dyn Saga,
        mut state: SagaState,
    ) -> Result<SagaState> {
        if state.is_completed() {
            return Ok(state);
        }

        info!(
            saga_id = %state.saga_id,
            current_step = state.current_step,
            total_steps = state.steps.len(),
            "Executing saga step"
        );

        match saga.execute_next_step(&mut state).await {
            Ok(_) => {
                self.repository.update(&state).await?;

                if state.is_completed() {
                    info!(
                        saga_id = %state.saga_id,
                        "Saga completed successfully"
                    );
                } else {
                    info!(
                        saga_id = %state.saga_id,
                        current_step = state.current_step,
                        "Saga step completed, advancing to next step"
                    );
                }

                Ok(state)
            }
            Err(e) => {
                error!(
                    saga_id = %state.saga_id,
                    error = %e,
                    "Saga step failed, initiating compensation"
                );

                // Save failed state before compensation
                self.repository.update(&state).await?;

                // Initiate compensation
                self.compensate_saga(saga, state).await
            }
        }
    }

    /// Run a saga to completion (execute all steps)
    pub async fn run_saga(
        &self,
        saga: &dyn Saga,
        mut state: SagaState,
    ) -> Result<SagaState> {
        info!(
            saga_id = %state.saga_id,
            saga_type = %state.saga_type,
            total_steps = state.steps.len(),
            "Running saga to completion"
        );

        while state.has_more_steps() && !state.is_completed() {
            state = self.execute_step(saga, state).await?;

            // If saga failed and was compensated, return the compensated state
            if state.is_compensating() || state.is_failed() {
                break;
            }
        }

        // Mark as completed if all steps succeeded
        if state.has_more_steps() == false && !state.is_completed() && !state.is_failed() {
            state.mark_completed();
            self.repository.update(&state).await?;
        }

        Ok(state)
    }

    /// Compensate a saga (rollback all completed steps)
    pub async fn compensate_saga(
        &self,
        saga: &dyn Saga,
        mut state: SagaState,
    ) -> Result<SagaState> {
        warn!(
            saga_id = %state.saga_id,
            "Starting saga compensation"
        );

        match saga.compensate_all(&mut state).await {
            Ok(_) => {
                self.repository.update(&state).await?;
                info!(
                    saga_id = %state.saga_id,
                    "Saga compensated successfully"
                );
                Ok(state)
            }
            Err(e) => {
                error!(
                    saga_id = %state.saga_id,
                    error = %e,
                    "Saga compensation failed"
                );
                state.mark_failed();
                self.repository.update(&state).await?;
                Err(e)
            }
        }
    }

    /// Resume a saga from its current state
    pub async fn resume_saga(&self, saga: &dyn Saga, saga_id: Uuid) -> Result<SagaState> {
        info!(saga_id = %saga_id, "Resuming saga");

        let state = self.repository.load(saga_id).await?;

        if state.is_completed() {
            info!(saga_id = %saga_id, "Saga already completed");
            return Ok(state);
        }

        if state.is_failed() {
            warn!(saga_id = %saga_id, "Cannot resume failed saga");
            return Err(SagaError::InvalidStateTransition {
                from: "FAILED".to_string(),
                to: "RUNNING".to_string(),
            });
        }

        self.run_saga(saga, state).await
    }

    /// Get saga state by ID
    pub async fn get_saga_state(&self, saga_id: Uuid) -> Result<SagaState> {
        self.repository.load(saga_id).await
    }

    /// Find sagas by status
    pub async fn find_sagas_by_status(
        &self,
        status: SagaStatus,
        limit: i64,
    ) -> Result<Vec<SagaState>> {
        self.repository.find_by_status(status, limit).await
    }

    /// Retry failed sagas (finds failed sagas and retries them)
    pub async fn retry_failed_sagas(&self, saga: &dyn Saga, limit: i64) -> Result<usize> {
        let failed_sagas = self.find_sagas_by_status(SagaStatus::Running, limit).await?;

        let mut retried = 0;
        for state in failed_sagas {
            if let Some(current_step) = state.current_step() {
                if current_step.can_retry() {
                    info!(
                        saga_id = %state.saga_id,
                        step = %current_step.name,
                        "Retrying failed saga"
                    );

                    match self.run_saga(saga, state).await {
                        Ok(_) => retried += 1,
                        Err(e) => {
                            error!(error = %e, "Failed to retry saga");
                        }
                    }
                }
            }
        }

        info!(retried_count = retried, "Completed retry of failed sagas");
        Ok(retried)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::{SagaStep, StepContext, StepExecutor};
    use async_trait::async_trait;
    use std::collections::HashMap;

    struct MockRepository {
        states: std::sync::Mutex<HashMap<Uuid, SagaState>>,
    }

    impl MockRepository {
        fn new() -> Self {
            Self {
                states: std::sync::Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl SagaRepository for MockRepository {
        async fn save(&self, state: &SagaState) -> Result<()> {
            self.states.lock().unwrap().insert(state.saga_id, state.clone());
            Ok(())
        }

        async fn update(&self, state: &SagaState) -> Result<()> {
            self.states.lock().unwrap().insert(state.saga_id, state.clone());
            Ok(())
        }

        async fn load(&self, saga_id: Uuid) -> Result<SagaState> {
            self.states
                .lock()
                .unwrap()
                .get(&saga_id)
                .cloned()
                .ok_or_else(|| SagaError::SagaNotFound(saga_id.to_string()))
        }

        async fn find_by_status(&self, status: SagaStatus, _limit: i64) -> Result<Vec<SagaState>> {
            Ok(self
                .states
                .lock()
                .unwrap()
                .values()
                .filter(|s| s.status == status)
                .cloned()
                .collect())
        }

        async fn delete(&self, saga_id: Uuid) -> Result<()> {
            self.states.lock().unwrap().remove(&saga_id);
            Ok(())
        }
    }

    struct TestExecutor {
        should_fail: bool,
    }

    #[async_trait]
    impl StepExecutor for TestExecutor {
        async fn execute(&self, _context: &StepContext) -> Result<serde_json::Value> {
            if self.should_fail {
                Err(SagaError::StepExecutionFailed("test failure".to_string()))
            } else {
                Ok(serde_json::json!({"success": true}))
            }
        }

        async fn compensate(&self, _context: &StepContext) -> Result<()> {
            Ok(())
        }
    }

    struct TestSaga {
        executors: HashMap<String, Box<dyn StepExecutor>>,
    }

    impl TestSaga {
        fn new(should_fail: bool) -> Self {
            let mut executors: HashMap<String, Box<dyn StepExecutor>> = HashMap::new();
            executors.insert("step1".to_string(), Box::new(TestExecutor { should_fail: false }));
            executors.insert("step2".to_string(), Box::new(TestExecutor { should_fail }));

            Self { executors }
        }
    }

    #[async_trait]
    impl Saga for TestSaga {
        fn saga_type(&self) -> &str {
            "test_saga"
        }

        fn step_executors(&self) -> &HashMap<String, Box<dyn StepExecutor>> {
            &self.executors
        }

        async fn create_state(&self, saga_id: Uuid, data: serde_json::Value) -> Result<SagaState> {
            let steps = vec![
                SagaStep::new("step1".to_string(), 3),
                SagaStep::new("step2".to_string(), 3),
            ];
            Ok(SagaState::new(saga_id, self.saga_type().to_string(), steps, data))
        }
    }

    #[tokio::test]
    async fn test_start_saga() {
        let repo = Arc::new(MockRepository::new());
        let coordinator = SagaCoordinator::new(repo.clone());
        let saga = TestSaga::new(false);

        let saga_id = Uuid::new_v4();
        let data = serde_json::json!({"test": "data"});

        let state = coordinator.start_saga(&saga, saga_id, data).await.unwrap();

        assert_eq!(state.saga_id, saga_id);
        assert_eq!(state.saga_type, "test_saga");
        assert_eq!(state.status, SagaStatus::Running);

        // Verify it was saved
        let loaded = repo.load(saga_id).await.unwrap();
        assert_eq!(loaded.saga_id, saga_id);
    }

    #[tokio::test]
    async fn test_run_saga_success() {
        let repo = Arc::new(MockRepository::new());
        let coordinator = SagaCoordinator::new(repo);
        let saga = TestSaga::new(false);

        let saga_id = Uuid::new_v4();
        let state = coordinator
            .start_saga(&saga, saga_id, serde_json::json!({}))
            .await
            .unwrap();

        let final_state = coordinator.run_saga(&saga, state).await.unwrap();

        assert_eq!(final_state.status, SagaStatus::Completed);
        assert_eq!(final_state.current_step, 2);
    }
}
