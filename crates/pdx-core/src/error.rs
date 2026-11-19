//! Typed, fail-closed error taxonomy shared by every crate.
//!
//! No parser may return a guessed value. Every validation failure carries a
//! kind, a message, and — where a byte position is known — an offset with
//! expected/actual values.

use serde::Serialize;
use std::fmt;

pub type Result<T> = std::result::Result<T, PdxError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Io,
    BadInput,
    UnsupportedFormatVersion,
    VerificationFailed,
    CapabilityFailure,
    Cancelled,
    Internal,
}

impl ErrorKind {
    /// Typed CLI exit codes from section 7 of the specification.
    pub fn exit_code(self) -> i32 {
        match self {
            ErrorKind::BadInput => 2,
            ErrorKind::UnsupportedFormatVersion => 3,
            ErrorKind::VerificationFailed => 4,
            ErrorKind::CapabilityFailure => 5,
            ErrorKind::Cancelled => 6,
            // IO and Internal have no dedicated code in the contract; 1 is
            // reserved for an unmapped failure.
            ErrorKind::Io | ErrorKind::Internal => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PdxError {
    pub kind: ErrorKind,
    pub message: String,
    pub offset: Option<u64>,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl PdxError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            offset: None,
            expected: None,
            actual: None,
        }
    }

    pub fn bad_input(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::BadInput, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Io, message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::UnsupportedFormatVersion, message)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::VerificationFailed, message)
    }

    pub fn capability(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::CapabilityFailure, message)
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Cancelled, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub fn at(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn expected(mut self, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            "{\"kind\":\"internal\",\"message\":\"error serialization failed\"}".to_string()
        })
    }
}

impl fmt::Display for PdxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)?;
        if let Some(offset) = self.offset {
            write!(f, " [offset=0x{offset:X}")?;
            if let (Some(expected), Some(actual)) = (&self.expected, &self.actual) {
                write!(f, ", expected={expected}, actual={actual}")?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl std::error::Error for PdxError {}

impl From<std::io::Error> for PdxError {
    fn from(error: std::io::Error) -> Self {
        Self::io(format!("io error: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes() {
        assert_eq!(ErrorKind::BadInput.exit_code(), 2);
        assert_eq!(ErrorKind::UnsupportedFormatVersion.exit_code(), 3);
        assert_eq!(ErrorKind::VerificationFailed.exit_code(), 4);
        assert_eq!(ErrorKind::CapabilityFailure.exit_code(), 5);
        assert_eq!(ErrorKind::Cancelled.exit_code(), 6);
    }

    #[test]
    fn error_carries_offset_and_expectation() {
        let e = PdxError::bad_input("truncated header")
            .at(0x40)
            .expected("4 bytes", "2 bytes");
        let text = e.to_string();
        assert!(text.contains("offset=0x40"));
        assert!(text.contains("expected=4 bytes"));
        assert!(text.contains("actual=2 bytes"));
    }

    #[test]
    fn json_is_typed() {
        let e = PdxError::unsupported("version 9");
        let v: serde_json::Value = serde_json::from_str(&e.to_json()).unwrap();
        assert_eq!(v["kind"], "unsupported_format_version");
    }
}
