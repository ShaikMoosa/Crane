use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationCode {
    MissingId,
    TitleTooLong,
    BodyTooLarge,
    TimestampOutOfRange,
    UnsupportedSchema,
    DuplicateId,
    UnsupportedExtension,
    InvalidUtf8,
    BatchTooLarge,
    MissingImportId,
    BackupFormat,
    UnsupportedBackupVersion,
    BackupTooLarge,
    InvalidBackup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub code: ValidationCode,
    pub message: String,
}

impl ValidationError {
    pub(crate) fn new(code: ValidationCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for ValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ValidationError {}
