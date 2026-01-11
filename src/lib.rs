//! # Termwright
//!
//! A Playwright-like automation framework for terminal TUI applications.
//!
//! Termwright enables AI agents and integration tests to interact with and observe
//! terminal user interfaces by wrapping applications in a pseudo-terminal (PTY).
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use termwright::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Spawn a terminal application
//!     let mut term = Terminal::builder()
//!         .size(80, 24)
//!         .spawn("vim", &["test.txt"])
//!         .await?;
//!
//!     // Wait for the application to be ready
//!     term.expect("test.txt").timeout(5.seconds()).await?;
//!
//!     // Send input
//!     term.send_key(Key::Char('i')).await?;
//!     term.type_str("Hello, world!").await?;
//!     term.send_key(Key::Escape).await?;
//!
//!     // Query screen state
//!     let screen = term.screen().await;
//!     assert!(screen.contains("Hello, world!"));
//!
//!     // Get JSON output for AI agents
//!     println!("{}", screen.to_json()?);
//!
//!     // Quit the application
//!     term.type_str(":q!").await?;
//!     term.enter().await?;
//!     term.wait_exit().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **PTY Wrapping**: Spawn and control terminal applications
//! - **Screen Reading**: Access text, colors, cursor position, and cell attributes
//! - **Wait Conditions**: Wait for text, patterns, screen stability, or process exit
//! - **Input Simulation**: Send keystrokes, special keys, and control sequences
//! - **Multiple Output Formats**: Plain text, JSON, and PNG screenshots
//!
//! ## Modules
//!
//! - [`terminal`]: Main Terminal struct and builder
//! - [`screen`]: Screen state representation and querying
//! - [`input`]: Key definitions and escape sequences
//! - [`wait`]: Wait conditions and duration helpers
//! - [`error`]: Error types
//! - [`prelude`]: Convenient re-exports

pub mod error;
pub mod input;
pub mod output;
pub mod screen;
pub mod terminal;
pub mod wait;

pub mod prelude;

// Re-export main types at crate root
pub use error::{Result, TermwrightError};
pub use input::Key;
pub use screen::Screen;
pub use terminal::Terminal;
