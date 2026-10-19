use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    Validation,
    NotFound,
    Io,
    UnsupportedVersion,
    Busy,
    StaleConfirmation,
    QueueFull,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<crane_domain::ValidationError> for AppError {
    fn from(error: crane_domain::ValidationError) -> Self {
        Self::new(ErrorCode::Validation, error.to_string())
    }
}
