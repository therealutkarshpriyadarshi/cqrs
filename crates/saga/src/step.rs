use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::errors::Result;

/// Status of a saga step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step is pending execution
    Pending,
    /// Step is currently executing
    Running,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Step is being compensated
    Compensating,
    /// Step compensation completed
    Compensated,
    /// Step compensation failed
    CompensationFailed,
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "PENDING"),
            StepStatus::Running => write!(f, "RUNNING"),
            StepStatus::Completed => write!(f, "COMPLETED"),
            StepStatus::Failed => write!(f, "FAILED"),
            StepStatus::Compensating => write!(f, "COMPENSATING"),
            StepStatus::Compensated => write!(f, "COMPENSATED"),
            StepStatus::CompensationFailed => write!(f, "COMPENSATION_FAILED"),
        }
    }
}

/// Context passed to step execution and compensation functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepContext {
    pub saga_id: uuid::Uuid,
    pub step_name: String,
    pub data: serde_json::Value,
}

/// Trait for executing saga steps
#[async_trait]
pub trait StepExecutor: Send + Sync {
    /// Execute the step
    async fn execute(&self, context: &StepContext) -> Result<serde_json::Value>;

    /// Compensate the step (undo its effects)
    async fn compensate(&self, context: &StepContext) -> Result<()>;
}

/// A step in a saga
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    pub name: String,
    pub status: StepStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl SagaStep {
    pub fn new(name: String, max_retries: u32) -> Self {
        Self {
            name,
            status: StepStatus::Pending,
            retry_count: 0,
            max_retries,
            result: None,
            error: None,
        }
    }

    pub fn mark_running(&mut self) {
        self.status = StepStatus::Running;
    }

    pub fn mark_completed(&mut self, result: serde_json::Value) {
        self.status = StepStatus::Completed;
        self.result = Some(result);
        self.error = None;
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = StepStatus::Failed;
        self.error = Some(error);
        self.retry_count += 1;
    }

    pub fn mark_compensating(&mut self) {
        self.status = StepStatus::Compensating;
    }

    pub fn mark_compensated(&mut self) {
        self.status = StepStatus::Compensated;
        self.error = None;
    }

    pub fn mark_compensation_failed(&mut self, error: String) {
        self.status = StepStatus::CompensationFailed;
        self.error = Some(error);
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn is_completed(&self) -> bool {
        self.status == StepStatus::Completed
    }

    pub fn is_failed(&self) -> bool {
        self.status == StepStatus::Failed
    }

    pub fn is_compensated(&self) -> bool {
        self.status == StepStatus::Compensated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_step() {
        let step = SagaStep::new("test_step".to_string(), 3);
        assert_eq!(step.name, "test_step");
        assert_eq!(step.status, StepStatus::Pending);
        assert_eq!(step.retry_count, 0);
        assert_eq!(step.max_retries, 3);
    }

    #[test]
    fn test_step_lifecycle() {
        let mut step = SagaStep::new("test".to_string(), 3);

        step.mark_running();
        assert_eq!(step.status, StepStatus::Running);

        step.mark_completed(serde_json::json!({"success": true}));
        assert_eq!(step.status, StepStatus::Completed);
        assert!(step.is_completed());
    }

    #[test]
    fn test_step_retry() {
        let mut step = SagaStep::new("test".to_string(), 3);

        assert!(step.can_retry());
        step.mark_failed("error1".to_string());
        assert_eq!(step.retry_count, 1);
        assert!(step.can_retry());

        step.mark_failed("error2".to_string());
        step.mark_failed("error3".to_string());
        assert_eq!(step.retry_count, 3);
        assert!(!step.can_retry());
    }

    #[test]
    fn test_compensation() {
        let mut step = SagaStep::new("test".to_string(), 3);

        step.mark_completed(serde_json::json!({"success": true}));
        step.mark_compensating();
        assert_eq!(step.status, StepStatus::Compensating);

        step.mark_compensated();
        assert_eq!(step.status, StepStatus::Compensated);
        assert!(step.is_compensated());
    }
}
