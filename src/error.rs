use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Unsupported,
    Timeout,
    Adb,
    Internal,
}

impl ErrorKind {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Unsupported => 4,
            Self::Timeout => 3,
            Self::Adb => 5,
            Self::Internal => 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unsupported, message)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Timeout, message)
    }

    pub fn adb(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Adb, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub fn exit_code(&self) -> i32 {
        self.kind.exit_code()
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}
