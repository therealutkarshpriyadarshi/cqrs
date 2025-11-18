use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),
}
