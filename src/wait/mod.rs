//! Wait conditions for terminal state changes.

use std::time::Duration;

use regex::Regex;

use crate::error::TermwrightError;
use crate::screen::{Position, Screen};

/// Default timeout for wait operations.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default poll interval for checking conditions.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// A condition to wait for.
#[derive(Debug, Clone)]
pub enum WaitCondition {
    /// Wait for specific text to appear on screen.
    TextAppears(String),
    /// Wait for specific text to disappear from screen.
    TextDisappears(String),
    /// Wait for a regex pattern to match.
    PatternMatches(String),
    /// Wait for the cursor to reach a specific position.
    CursorAt(Position),
    /// Wait for the screen to stabilize (no changes for duration).
    ScreenStable(Duration),
    /// Wait for the process to exit.
    ProcessExit,
}

impl WaitCondition {
    /// Check if this condition is satisfied by the given screen state.
    pub fn is_satisfied(&self, screen: &Screen, prev_screen: Option<&Screen>) -> bool {
        match self {
            WaitCondition::TextAppears(text) => screen.contains(text),
            WaitCondition::TextDisappears(text) => !screen.contains(text),
            WaitCondition::PatternMatches(pattern) => {
                if let Ok(re) = Regex::new(pattern) {
                    re.is_match(&screen.text())
                } else {
                    false
                }
            }
            WaitCondition::CursorAt(pos) => screen.cursor() == *pos,
            WaitCondition::ScreenStable(_) => {
                // Screen stability is checked by comparing with previous screen
                if let Some(prev) = prev_screen {
                    screen.text() == prev.text()
                } else {
                    false
                }
            }
            WaitCondition::ProcessExit => {
                // This is handled specially by the terminal
                false
            }
        }
    }

    /// Get a human-readable description of this condition.
    pub fn description(&self) -> String {
        match self {
            WaitCondition::TextAppears(text) => format!("text '{}' to appear", text),
            WaitCondition::TextDisappears(text) => format!("text '{}' to disappear", text),
            WaitCondition::PatternMatches(pattern) => {
                format!("pattern '{}' to match", pattern)
            }
            WaitCondition::CursorAt(pos) => {
                format!("cursor at row={}, col={}", pos.row, pos.col)
            }
            WaitCondition::ScreenStable(duration) => {
                format!("screen stable for {:?}", duration)
            }
            WaitCondition::ProcessExit => "process to exit".to_string(),
        }
    }
}

/// Builder for wait operations with fluent API.
pub struct WaitBuilder {
    condition: WaitCondition,
    timeout: Duration,
    poll_interval: Duration,
}

impl WaitBuilder {
    /// Create a new wait builder for the given condition.
    pub fn new(condition: WaitCondition) -> Self {
        Self {
            condition,
            timeout: DEFAULT_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }

    /// Set the timeout for this wait operation.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the poll interval for checking the condition.
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Get the condition.
    pub fn condition(&self) -> &WaitCondition {
        &self.condition
    }

    /// Get the timeout.
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }

    /// Get the poll interval.
    pub fn get_poll_interval(&self) -> Duration {
        self.poll_interval
    }

    /// Create a timeout error for this wait.
    pub fn timeout_error(&self) -> TermwrightError {
        TermwrightError::Timeout {
            condition: self.condition.description(),
            timeout: self.timeout,
        }
    }
}

/// Extension trait for Duration to create durations more ergonomically.
pub trait DurationExt {
    /// Create a duration from this value in seconds.
    fn seconds(self) -> Duration;
    /// Create a duration from this value in milliseconds.
    fn millis(self) -> Duration;
}

impl DurationExt for u64 {
    fn seconds(self) -> Duration {
        Duration::from_secs(self)
    }

    fn millis(self) -> Duration {
        Duration::from_millis(self)
    }
}

impl DurationExt for i32 {
    fn seconds(self) -> Duration {
        Duration::from_secs(self as u64)
    }

    fn millis(self) -> Duration {
        Duration::from_millis(self as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_ext() {
        assert_eq!(5.seconds(), Duration::from_secs(5));
        assert_eq!(100.millis(), Duration::from_millis(100));
    }

    #[test]
    fn test_condition_description() {
        let cond = WaitCondition::TextAppears("hello".to_string());
        assert!(cond.description().contains("hello"));
    }
}
