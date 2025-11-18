use chrono::Utc;
use saga::coordinator::SagaCoordinator;
use saga::repository::SagaRepository;
use saga::saga::{Saga, SagaState, SagaStatus};
use saga::step::{SagaStep, StepContext, StepExecutor};
use saga::errors::{Result, SagaError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use async_trait::async_trait;

// Mock repository for testing
struct MockSagaRepository {
    states: Arc<Mutex<HashMap<Uuid, SagaState>>>,
}

impl MockSagaRepository {
    fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SagaRepository for MockSagaRepository {
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

// Mock step executor that succeeds
struct SuccessExecutor {
    name: String,
    compensation_count: Arc<Mutex<usize>>,
}

impl SuccessExecutor {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            compensation_count: Arc::new(Mutex::new(0)),
        }
    }

    fn compensation_count(&self) -> usize {
        *self.compensation_count.lock().unwrap()
    }
}

#[async_trait]
impl StepExecutor for SuccessExecutor {
    async fn execute(&self, _context: &StepContext) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "step": self.name,
            "success": true
        }))
    }

    async fn compensate(&self, _context: &StepContext) -> Result<()> {
        *self.compensation_count.lock().unwrap() += 1;
        Ok(())
    }
}

// Mock step executor that fails
struct FailingExecutor {
    fail_on_attempt: usize,
    attempt_count: Arc<Mutex<usize>>,
}

impl FailingExecutor {
    fn new(fail_on_attempt: usize) -> Self {
        Self {
            fail_on_attempt,
            attempt_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl StepExecutor for FailingExecutor {
    async fn execute(&self, _context: &StepContext) -> Result<serde_json::Value> {
        let mut count = self.attempt_count.lock().unwrap();
        *count += 1;

        if *count == self.fail_on_attempt {
            Err(SagaError::StepExecutionFailed("Simulated failure".to_string()))
        } else {
            Ok(serde_json::json!({"success": true}))
        }
    }

    async fn compensate(&self, _context: &StepContext) -> Result<()> {
        Ok(())
    }
}

// Test saga with configurable executors
struct TestSaga {
    executors: HashMap<String, Box<dyn StepExecutor>>,
}

impl TestSaga {
    fn with_success_steps() -> Self {
        let mut executors: HashMap<String, Box<dyn StepExecutor>> = HashMap::new();
        executors.insert("step1".to_string(), Box::new(SuccessExecutor::new("step1")));
        executors.insert("step2".to_string(), Box::new(SuccessExecutor::new("step2")));
        executors.insert("step3".to_string(), Box::new(SuccessExecutor::new("step3")));

        Self { executors }
    }

    fn with_failing_step(fail_at_step: usize) -> Self {
        let mut executors: HashMap<String, Box<dyn StepExecutor>> = HashMap::new();

        for i in 1..=3 {
            let executor: Box<dyn StepExecutor> = if i == fail_at_step {
                Box::new(FailingExecutor::new(1))
            } else {
                Box::new(SuccessExecutor::new(&format!("step{}", i)))
            };
            executors.insert(format!("step{}", i), executor);
        }

        Self { executors }
    }
}

#[async_trait]
impl Saga for TestSaga {
    fn saga_type(&self) -> &str {
        "TestSaga"
    }

    fn step_executors(&self) -> &HashMap<String, Box<dyn StepExecutor>> {
        &self.executors
    }

    async fn create_state(&self, saga_id: Uuid, data: serde_json::Value) -> Result<SagaState> {
        let steps = vec![
            SagaStep::new("step1".to_string(), 3),
            SagaStep::new("step2".to_string(), 3),
            SagaStep::new("step3".to_string(), 3),
        ];
        Ok(SagaState::new(saga_id, self.saga_type().to_string(), steps, data))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_saga_successful_execution() {
    let repo = Arc::new(MockSagaRepository::new());
    let coordinator = SagaCoordinator::new(repo.clone());
    let saga = TestSaga::with_success_steps();

    let saga_id = Uuid::new_v4();
    let data = serde_json::json!({"test": "data"});

    // Start saga
    let state = coordinator.start_saga(&saga, saga_id, data).await.unwrap();
    assert_eq!(state.status, SagaStatus::Running);

    // Run to completion
    let final_state = coordinator.run_saga(&saga, state).await.unwrap();

    assert_eq!(final_state.status, SagaStatus::Completed);
    assert_eq!(final_state.current_step, 3);

    // Verify all steps are completed
    for step in &final_state.steps {
        assert!(step.is_completed(), "Step {} should be completed", step.name);
    }
}

#[tokio::test]
async fn test_saga_with_failure_and_compensation() {
    let repo = Arc::new(MockSagaRepository::new());
    let coordinator = SagaCoordinator::new(repo.clone());
    let saga = TestSaga::with_failing_step(2); // Fail on step 2

    let saga_id = Uuid::new_v4();
    let data = serde_json::json!({"test": "data"});

    // Start saga
    let state = coordinator.start_saga(&saga, saga_id, data).await.unwrap();

    // Run saga (will fail and compensate)
    let final_state = coordinator.run_saga(&saga, state).await.unwrap();

    // Should be compensated after failure
    assert!(
        final_state.status == SagaStatus::Compensated || final_state.status == SagaStatus::Failed,
        "Saga should be compensated or failed, got {:?}",
        final_state.status
    );

    // First step should be completed and then compensated
    assert!(final_state.steps[0].is_compensated() || final_state.steps[0].is_completed());
}

#[tokio::test]
async fn test_saga_state_persistence() {
    let repo = Arc::new(MockSagaRepository::new());
    let coordinator = SagaCoordinator::new(repo.clone());
    let saga = TestSaga::with_success_steps();

    let saga_id = Uuid::new_v4();
    let data = serde_json::json!({"test": "data"});

    // Start and run saga
    let state = coordinator.start_saga(&saga, saga_id, data).await.unwrap();
    let _final_state = coordinator.run_saga(&saga, state).await.unwrap();

    // Load from repository
    let loaded_state = repo.load(saga_id).await.unwrap();
    assert_eq!(loaded_state.saga_id, saga_id);
    assert_eq!(loaded_state.status, SagaStatus::Completed);
}

#[tokio::test]
async fn test_saga_resume() {
    let repo = Arc::new(MockSagaRepository::new());
    let coordinator = SagaCoordinator::new(repo.clone());
    let saga = TestSaga::with_success_steps();

    let saga_id = Uuid::new_v4();
    let data = serde_json::json!({"test": "data"});

    // Start saga
    let mut state = coordinator.start_saga(&saga, saga_id, data).await.unwrap();

    // Execute only first step
    state = coordinator.execute_step(&saga, state).await.unwrap();
    assert_eq!(state.current_step, 1);

    // Resume saga
    let final_state = coordinator.resume_saga(&saga, saga_id).await.unwrap();
    assert_eq!(final_state.status, SagaStatus::Completed);
    assert_eq!(final_state.current_step, 3);
}

#[tokio::test]
async fn test_find_sagas_by_status() {
    let repo = Arc::new(MockSagaRepository::new());
    let coordinator = SagaCoordinator::new(repo.clone());
    let saga = TestSaga::with_success_steps();

    // Create multiple sagas
    for _ in 0..3 {
        let saga_id = Uuid::new_v4();
        let data = serde_json::json!({});
        coordinator.start_saga(&saga, saga_id, data).await.unwrap();
    }

    // Find running sagas
    let running_sagas = coordinator.find_sagas_by_status(SagaStatus::Running, 10).await.unwrap();
    assert_eq!(running_sagas.len(), 3);
}
