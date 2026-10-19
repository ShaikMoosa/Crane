use std::fmt;

pub mod backup;
pub mod markdown;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransferError {
    Markdown(String),
    Json(String),
    BackupFormat,
    BackupVersion(u32),
    StoreVersion(u32),
    Validation(String),
    BackupSize,
    Io(String),
}

impl TransferError {
    pub(crate) fn markdown(message: impl Into<String>) -> Self {
        Self::Markdown(message.into())
    }
}

impl fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Markdown(message)
            | Self::Json(message)
            | Self::Validation(message)
            | Self::Io(message) => formatter.write_str(message),
            Self::BackupFormat => formatter.write_str("This is not a Crane backup."),
            Self::BackupVersion(version) => {
                write!(formatter, "Unsupported Crane backup version {version}.")
            }
            Self::StoreVersion(version) => {
                write!(formatter, "Unsupported store version {version}.")
            }
            Self::BackupSize => formatter.write_str("The backup is larger than the 256 MB limit."),
        }
    }
}

impl std::error::Error for TransferError {}
