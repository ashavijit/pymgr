use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    PythonNotFound,
    VersionMismatch,
    EnvAlreadyExists,
    EnvNotFound,
    PackageNotFound,
    ResolutionConflict,
    ChecksumMismatch,
    NetworkError,
    AuthRequired,
    LockStale,
    IoError,
    ConfigError,
    InstallError,
}

impl ErrorCode {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PythonNotFound => "E001",
            Self::VersionMismatch => "E002",
            Self::EnvAlreadyExists => "E100",
            Self::EnvNotFound => "E101",
            Self::PackageNotFound => "E200",
            Self::ResolutionConflict => "E201",
            Self::ChecksumMismatch => "E202",
            Self::NetworkError => "E300",
            Self::AuthRequired => "E301",
            Self::LockStale => "E400",
            Self::IoError => "E500",
            Self::ConfigError => "E501",
            Self::InstallError => "E502",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[derive(Debug, Error)]
pub enum PymgrError {
    #[error("Error [{code}]: {message}")]
    Coded {
        code: ErrorCode,
        message: String,
        suggestions: Vec<String>,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),

    #[error("{0}")]
    Other(String),
}

impl PymgrError {
    pub fn coded(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Coded {
            code,
            message: message.into(),
            suggestions: Vec::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        if let Self::Coded { suggestions, .. } = &mut self {
            suggestions.push(suggestion.into());
        }
        self
    }

    pub fn with_suggestions(mut self, hints: Vec<String>) -> Self {
        if let Self::Coded { suggestions, .. } = &mut self {
            *suggestions = hints;
        }
        self
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Coded {
                code,
                message,
                suggestions,
            } => serde_json::json!({
                "error": {
                    "code": code.code(),
                    "message": message,
                    "suggestions": suggestions,
                }
            }),
            _ => serde_json::json!({
                "error": {
                    "code": null,
                    "message": self.to_string(),
                    "suggestions": [],
                }
            }),
        }
    }

    pub fn format_human(&self) -> String {
        match self {
            Self::Coded {
                code,
                message,
                suggestions,
            } => {
                let mut out = format!("Error [{}]: {}", code.code(), message);
                if !suggestions.is_empty() {
                    out.push_str("\n\n  To fix, try:");
                    for s in suggestions {
                        out.push_str(&format!("\n    • {}", s));
                    }
                }
                out
            }
            _ => self.to_string(),
        }
    }
}

pub type PymgrResult<T> = Result<T, PymgrError>;
