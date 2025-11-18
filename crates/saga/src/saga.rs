use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

use crate::errors::{Result, SagaError};
use crate::step::{SagaStep, StepContext, StepExecutor};

/// Status of the entire saga
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaStatus {
    /// Saga is running forward
    Running,
    /// Saga completed successfully
    Completed,
    /// Saga failed and is compensating
    Compensating,
    /// Saga compensation completed (rolled back)
    Compensated,
    /// Saga failed completely (compensation also failed)
    Failed,
}

impl fmt::Display for SagaStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SagaStatus::Running => write!(f, "RUNNING"),
            SagaStatus::Completed => write!(f, "COMPLETED"),
            SagaStatus::Compensating => write!(f, "COMPENSATING"),
            SagaStatus::Compensated => write!(f, "COMPENSATED"),
            SagaStatus::Failed => write!(f, "FAILED"),
        }
    }
}

/// State of a saga instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaState {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub status: SagaStatus,
    pub current_step: usize,
    pub steps: Vec<SagaStep>,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SagaState {
    pub fn new(saga_id: Uuid, saga_type: String, steps: Vec<SagaStep>, data: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            saga_id,
            saga_type,
            status: SagaStatus::Running,
            current_step: 0,
            steps,
            data,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_completed(&self) -> bool {
        self.status == SagaStatus::Completed
    }

    pub fn is_compensating(&self) -> bool {
        self.status == SagaStatus::Compensating
    }

    pub fn is_failed(&self) -> bool {
        self.status == SagaStatus::Failed
    }

    pub fn has_more_steps(&self) -> bool {
        self.current_step < self.steps.len()
    }

    pub fn current_step(&self) -> Option<&SagaStep> {
        self.steps.get(self.current_step)
    }

    pub fn current_step_mut(&mut self) -> Option<&mut SagaStep> {
        self.steps.get_mut(self.current_step)
    }

    pub fn advance_step(&mut self) {
        self.current_step += 1;
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self) {
        self.status = SagaStatus::Completed;
        self.updated_at = Utc::now();
    }

    pub fn mark_compensating(&mut self) {
        self.status = SagaStatus::Compensating;
        self.updated_at = Utc::now();
    }

    pub fn mark_compensated(&mut self) {
        self.status = SagaStatus::Compensated;
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self) {
        self.status = SagaStatus::Failed;
        self.updated_at = Utc::now();
    }

    /// Get steps that need compensation (completed steps in reverse order)
    pub fn get_compensation_steps(&self) -> Vec<(usize, &SagaStep)> {
        self.steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.is_completed())
            .rev()
            .collect()
    }
}

/// Trait for saga implementations
#[async_trait]
pub trait Saga: Send + Sync {
    /// Get saga type name
    fn saga_type(&self) -> &str;

    /// Get step executors
    fn step_executors(&self) -> &HashMap<String, Box<dyn StepExecutor>>;

    /// Create initial saga state
    async fn create_state(&self, saga_id: Uuid, data: serde_json::Value) -> Result<SagaState>;

    /// Execute the next step
    async fn execute_next_step(&self, state: &mut SagaState) -> Result<()> {
        if state.is_completed() {
            return Err(SagaError::AlreadyCompleted);
        }

        if !state.has_more_steps() {
            state.mark_completed();
            return Ok(());
        }

        // Get step information before borrowing mutably
        let (saga_id, step_name, data) = {
            let step = state.current_step()
                .ok_or_else(|| SagaError::StepNotFound("current step".to_string()))?;
            (state.saga_id, step.name.clone(), state.data.clone())
        };

        // Now mark step as running
        let step = state.current_step_mut()
            .ok_or_else(|| SagaError::StepNotFound("current step".to_string()))?;
        step.mark_running();

        let context = StepContext {
            saga_id,
            step_name: step_name.clone(),
            data,
        };

        let executor = self.step_executors()
            .get(&step_name)
            .ok_or_else(|| SagaError::StepNotFound(step_name))?;

        match executor.execute(&context).await {
            Ok(result) => {
                let step = state.current_step_mut().unwrap();
                step.mark_completed(result);
                state.advance_step();
                Ok(())
            }
            Err(e) => {
                let step = state.current_step_mut().unwrap();
                step.mark_failed(e.to_string());
                Err(e)
            }
        }
    }

    /// Compensate a specific step
    async fn compensate_step(&self, state: &mut SagaState, step_index: usize) -> Result<()> {
        // Get step information first
        let (saga_id, step_name, data, is_completed) = {
            let step = state.steps.get(step_index)
                .ok_or_else(|| SagaError::StepNotFound(format!("step {}", step_index)))?;
            (state.saga_id, step.name.clone(), state.data.clone(), step.is_completed())
        };

        if !is_completed {
            return Ok(()); // Only compensate completed steps
        }

        // Mark step as compensating
        let step = state.steps.get_mut(step_index).unwrap();
        step.mark_compensating();

        let context = StepContext {
            saga_id,
            step_name: step_name.clone(),
            data,
        };

        let executor = self.step_executors()
            .get(&step_name)
            .ok_or_else(|| SagaError::StepNotFound(step_name))?;

        match executor.compensate(&context).await {
            Ok(_) => {
                let step = state.steps.get_mut(step_index).unwrap();
                step.mark_compensated();
                Ok(())
            }
            Err(e) => {
                let step = state.steps.get_mut(step_index).unwrap();
                step.mark_compensation_failed(e.to_string());
                Err(e)
            }
        }
    }

    /// Compensate all completed steps (in reverse order)
    async fn compensate_all(&self, state: &mut SagaState) -> Result<()> {
        if state.is_compensating() {
            return Err(SagaError::AlreadyCompensating);
        }

        state.mark_compensating();

        // Collect indices before iterating to avoid borrow issues
        let compensation_indices: Vec<usize> = state
            .get_compensation_steps()
            .into_iter()
            .map(|(index, _)| index)
            .collect();

        for index in compensation_indices {
            if let Err(e) = self.compensate_step(state, index).await {
                tracing::error!(
                    saga_id = %state.saga_id,
                    step_index = index,
                    error = %e,
                    "Compensation failed for step"
                );
                state.mark_failed();
                return Err(e);
            }
        }

        state.mark_compensated();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saga_state_creation() {
        let saga_id = Uuid::new_v4();
        let steps = vec![
            SagaStep::new("step1".to_string(), 3),
            SagaStep::new("step2".to_string(), 3),
        ];
        let data = serde_json::json!({"order_id": "123"});

        let state = SagaState::new(saga_id, "test_saga".to_string(), steps, data);

        assert_eq!(state.saga_id, saga_id);
        assert_eq!(state.saga_type, "test_saga");
        assert_eq!(state.status, SagaStatus::Running);
        assert_eq!(state.current_step, 0);
        assert_eq!(state.steps.len(), 2);
    }

    #[test]
    fn test_saga_state_progression() {
        let saga_id = Uuid::new_v4();
        let steps = vec![
            SagaStep::new("step1".to_string(), 3),
            SagaStep::new("step2".to_string(), 3),
        ];
        let data = serde_json::json!({});

        let mut state = SagaState::new(saga_id, "test".to_string(), steps, data);

        assert!(state.has_more_steps());
        assert_eq!(state.current_step().unwrap().name, "step1");

        state.advance_step();
        assert_eq!(state.current_step, 1);
        assert_eq!(state.current_step().unwrap().name, "step2");

        state.advance_step();
        assert_eq!(state.current_step, 2);
        assert!(!state.has_more_steps());
    }

    #[test]
    fn test_compensation_steps() {
        let saga_id = Uuid::new_v4();
        let mut steps = vec![
            SagaStep::new("step1".to_string(), 3),
            SagaStep::new("step2".to_string(), 3),
            SagaStep::new("step3".to_string(), 3),
        ];

        steps[0].mark_completed(serde_json::json!({}));
        steps[1].mark_completed(serde_json::json!({}));
        // step3 is not completed

        let data = serde_json::json!({});
        let state = SagaState::new(saga_id, "test".to_string(), steps, data);

        let compensation_steps = state.get_compensation_steps();
        assert_eq!(compensation_steps.len(), 2);
        // Should be in reverse order
        assert_eq!(compensation_steps[0].1.name, "step2");
        assert_eq!(compensation_steps[1].1.name, "step1");
    }
}
