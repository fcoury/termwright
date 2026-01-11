# Termwright

A Playwright-like automation framework for terminal TUI applications.

Termwright enables AI agents and integration tests to interact with and observe terminal user interfaces by wrapping applications in a pseudo-terminal (PTY).

## Features

- **PTY Wrapping**: Spawn and control any terminal application
- **Screen Reading**: Access text, colors, cursor position, and cell attributes
- **Wait Conditions**: Wait for text, regex patterns, screen stability, or process exit
- **Input Simulation**: Send keystrokes, special keys, and control sequences
- **Multiple Output Formats**: Plain text, JSON (for AI agents), and PNG screenshots
- **Box Detection**: Detect UI boundaries using box-drawing characters
- **Framework Agnostic**: Works with any TUI framework (ratatui, crossterm, ncurses, etc.)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
termwright = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Or install the CLI:

```bash
cargo install termwright
```

## Quick Start

### Library Usage

```rust
use termwright::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Spawn a terminal application
    let term = Terminal::builder()
        .size(80, 24)
        .spawn("vim", &["test.txt"])
        .await?;

    // Wait for the application to be ready
    term.expect("VIM")
        .timeout(Duration::from_secs(5))
        .await?;

    // Send input
    term.send_key(Key::Char('i')).await?;
    term.type_str("Hello, world!").await?;
    term.send_key(Key::Escape).await?;

    // Query screen state
    let screen = term.screen().await;
    assert!(screen.contains("Hello, world!"));

    // Get structured output for AI agents
    println!("{}", screen.to_json()?);

    // Take a screenshot
    term.screenshot().await.save("vim.png")?;

    // Quit the application
    term.type_str(":q!").await?;
    term.enter().await?;
    term.wait_exit().await?;

    Ok(())
}
```

### CLI Usage

Capture terminal output as text:

```bash
termwright run -- ls -la
```

Take a screenshot of a TUI application:

```bash
termwright screenshot --wait-for "VIM" -o vim.png -- vim test.txt
```

Get JSON output for AI processing:

```bash
termwright run --format json -- htop
```

## CLI Reference

### `termwright run`

Run a command and capture its output.

```
termwright run [OPTIONS] -- <COMMAND> [ARGS]...

Options:
  --cols <COLS>          Terminal width [default: 80]
  --rows <ROWS>          Terminal height [default: 24]
  --wait-for <TEXT>      Wait for this text to appear before capturing
  --delay <MS>           Delay in milliseconds before capturing [default: 500]
  --format <FORMAT>      Output format: text, json, json-compact [default: text]
  --timeout <SECS>       Timeout for wait conditions [default: 30]
```

### `termwright screenshot`

Take a PNG screenshot of a terminal application.

```
termwright screenshot [OPTIONS] -- <COMMAND> [ARGS]...

Options:
  --cols <COLS>          Terminal width [default: 80]
  --rows <ROWS>          Terminal height [default: 24]
  --wait-for <TEXT>      Wait for this text to appear before capturing
  --delay <MS>           Delay in milliseconds before capturing [default: 500]
  -o, --output <PATH>    Output file path (defaults to stdout)
  --font <NAME>          Font name for rendering
  --font-size <SIZE>     Font size in pixels [default: 14]
  --timeout <SECS>       Timeout for wait conditions [default: 30]
```

## API Overview

### Terminal

The main entry point for controlling terminal applications:

```rust
let term = Terminal::builder()
    .size(80, 24)
    .spawn("vim", &["file.txt"])
    .await?;

// Input
term.type_str("hello").await?;
term.send_key(Key::Enter).await?;
term.enter().await?;  // Shorthand for Enter key

// Screen access
let screen = term.screen().await;

// Wait conditions
term.expect("Ready").timeout(Duration::from_secs(5)).await?;
term.wait_exit().await?;

// Screenshots
term.screenshot().await.save("output.png")?;
```

### Screen

Query the terminal screen state:

```rust
let screen = term.screen().await;

// Text access
let text = screen.text();
let line = screen.line(0);
assert!(screen.contains("hello"));

// Cell-level access
let cell = screen.cell(0, 0);
println!("Char: {}, FG: {:?}, BG: {:?}", cell.char, cell.fg, cell.bg);

// Cursor position
let cursor = screen.cursor();
println!("Cursor at row={}, col={}", cursor.row, cursor.col);

// Region extraction
let region = screen.region(0..10, 0..5);

// Pattern matching
if let Some(pos) = screen.find_text("error") {
    println!("Found at row={}, col={}", pos.row, pos.col);
}

// Box detection (UI boundaries)
let boxes = screen.detect_boxes();

// Output formats
println!("{}", screen.to_json()?);        // Pretty JSON
println!("{}", screen.to_json_compact()?); // Compact JSON
```

### Keys

Available key types for input:

```rust
Key::Char('a')      // Regular characters
Key::Enter          // Enter/Return
Key::Tab            // Tab
Key::Escape         // Escape
Key::Backspace      // Backspace
Key::Up, Key::Down, Key::Left, Key::Right  // Arrow keys
Key::Home, Key::End
Key::PageUp, Key::PageDown
Key::Insert, Key::Delete
Key::F(1)..Key::F(12)  // Function keys
Key::Ctrl('c')      // Ctrl combinations
Key::Alt('x')       // Alt combinations
```

## Requirements

- Rust 1.85.0 or later (Edition 2024)
- macOS or Linux (Windows not supported)
- For screenshots: A monospace font (uses system fonts via font-kit)

## Use Cases

- **AI Agents**: Enable LLMs to observe and interact with terminal UIs via JSON output
- **Integration Testing**: Automated testing of TUI applications
- **Documentation**: Generate screenshots for documentation
- **Accessibility**: Extract text content from visual terminal applications

## License

MIT License - see [LICENSE](LICENSE) for details.
