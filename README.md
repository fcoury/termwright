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

### Daemon Usage

The `daemon` subcommand runs a long-lived terminal session and exposes a local Unix socket for automation. This is useful when you want to keep an app running and interact with it incrementally (similar to how Playwright keeps a browser process alive).

Start a daemon (foreground; blocks until you close it):

```bash
termwright daemon -- vim test.txt
# prints a socket path like:
# /tmp/termwright-12345.sock
```

Start a daemon in the background (returns immediately):

```bash
SOCK=$(termwright daemon --background -- vim test.txt)
echo "$SOCK"
```

Stop a daemon (sends a `close` request):

```bash
printf '{"id":1,"method":"close","params":null}\n' | nc -U "$SOCK"
```

## CLI Reference

### `termwright fonts`

List available font families on the system (helpful for selecting a monospace font for screenshots).

```
termwright fonts
```

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

### `termwright daemon`

Run a single TUI session and expose it over a Unix socket.

```
termwright daemon [OPTIONS] -- <COMMAND> [ARGS]...

Options:
  --cols <COLS>          Terminal width [default: 80]
  --rows <ROWS>          Terminal height [default: 24]
  --socket <PATH>        Unix socket path (defaults to a temp path)
  --background           Start daemon in the background
```

The command prints the socket path to stdout.

## Daemon User Guide

### Connecting from Rust

```rust
use std::time::Duration;
use termwright::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = DaemonClient::connect_unix("/tmp/termwright-12345.sock").await?;

    // Sanity check: talk to the server
    let info = client.handshake().await?;
    println!("daemon pid={}", info.pid);

    // Wait and read screen
    client.wait_for_text("VIM", Some(Duration::from_secs(5))).await?;
    println!("{}", client.screen_text().await?);

    // Keyboard
    client.press("Escape").await?;
    client.hotkey_ctrl('r').await?;
    client.r#type(":q!\n").await?;

    // Mouse (row/col are 0-based cell coordinates)
    client.mouse_move(10, 10).await?;
    client.mouse_click(10, 10, MouseButton::Left).await?;

    // Shut down daemon + child process
    client.close().await?;
    Ok(())
}
```

### Notes / Caveats

- The daemon is local-only: it listens on a Unix socket you control.
- Mouse events are best-effort: many TUIs ignore mouse input unless they explicitly enable mouse reporting.
- Coordinate system for `mouse_move`/`mouse_click` is `row`/`col` in terminal cells (0-based).

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
