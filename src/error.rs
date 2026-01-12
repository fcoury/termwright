//! Error types for termwright.

use std::time::Duration;

/// Result type alias using TermwrightError.
pub type Result<T> = std::result::Result<T, TermwrightError>;

/// Errors that can occur when using termwright.
#[derive(Debug, thiserror::Error)]
pub enum TermwrightError {
    /// PTY-related I/O error.
    #[error("PTY error: {0}")]
    Pty(#[from] std::io::Error),

    /// Timeout waiting for a condition.
    #[error("Timeout after {timeout:?} waiting for: {condition}")]
    Timeout {
        /// The condition that was being waited for.
        condition: String,
        /// How long we waited.
        timeout: Duration,
    },

    /// The spawned process exited unexpectedly.
    #[error("Process exited with code: {code:?}")]
    ProcessExited {
        /// Exit code, if available.
        code: Option<i32>,
    },

    /// A text pattern was not found on screen.
    #[error("Pattern not found: {pattern}")]
    PatternNotFound {
        /// The pattern that wasn't found.
        pattern: String,
    },

    /// Invalid region specification.
    #[error("Invalid region: {0}")]
    InvalidRegion(String),

    /// Failed to spawn the command.
    #[error("Failed to spawn command: {0}")]
    SpawnFailed(String),

    /// Terminal not running.
    #[error("Terminal is not running")]
    NotRunning,

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Regex compilation error.
    #[error("Invalid regex: {0}")]
    Regex(#[from] regex::Error),

    /// Image rendering error.
    #[error("Image error: {0}")]
    Image(String),

    /// Font loading error.
    #[error("Font error: {0}")]
    Font(String),

    /// IPC transport error (daemon).
    #[error("IPC error: {0}")]
    Ipc(String),

    /// Protocol/serialization error (daemon).
    #[error("Protocol error: {0}")]
    Protocol(String),
}
