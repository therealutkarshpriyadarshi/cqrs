use thiserror::Error;

#[derive(Debug, Error)]
pub enum SagaError {
    #[error("Step execution failed: {0}")]
    StepExecutionFailed(String),

    #[error("Compensation failed: {0}")]
    CompensationFailed(String),

    #[error("Saga already completed")]
    AlreadyCompleted,

    #[error("Saga already compensating")]
    AlreadyCompensating,

    #[error("Invalid saga state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: String,
        to: String,
    },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Saga not found: {0}")]
    SagaNotFound(String),

    #[error("Step not found: {0}")]
    StepNotFound(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, SagaError>;
