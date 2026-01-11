//! Convenient re-exports for common usage.
//!
//! ```rust
//! use termwright::prelude::*;
//! ```

pub use crate::error::{Result, TermwrightError};
pub use crate::input::Key;
pub use crate::screen::{
    BoxStyle, Cell, CellAttributes, Color, DetectedBox, Position, Region, Screen, Size, TextMatch,
};
pub use crate::output::{Screenshot, ScreenshotConfig};
pub use crate::terminal::{Terminal, TerminalBuilder, TerminalConfig};
pub use crate::wait::{DurationExt, WaitCondition};
