---
name: termwright-tui-testing
description: Automates end-to-end testing and debugging of terminal UIs using Termwright. Use when testing TUI apps (ratatui, crossterm, ncurses), automating terminal interactions, capturing screenshots, or debugging terminal-based interfaces. Handles step files, daemon control, and artifact analysis.
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
  - Glob
  - Grep
---

# Termwright TUI Testing

Automate end-to-end testing of terminal user interfaces with Termwright's daemon-based architecture.

## Quick Start

### Run a Step File Test

```bash
# Execute YAML steps file with trace output
termwright run-steps --trace test.yaml

# Connect to existing daemon instead of spawning
termwright run-steps --connect /tmp/termwright-123.sock steps.yaml
```

### Interactive Daemon Session

```bash
# Start daemon in background
SOCK=$(termwright daemon --background -- vim test.txt)

# Send commands via exec
termwright exec --socket "$SOCK" --method handshake
termwright exec --socket "$SOCK" --method wait_for_text --params '{"text":"VIM","timeout_ms":5000}'
termwright exec --socket "$SOCK" --method press --params '{"key":"i"}'
termwright exec --socket "$SOCK" --method type --params '{"text":"Hello"}'
termwright exec --socket "$SOCK" --method screen --params '{"format":"text"}'
termwright exec --socket "$SOCK" --method close
```

### Quick Screenshot

```bash
termwright screenshot --wait-for "Ready" -o app.png -- ./my-tui-app
```

## Core Workflows

### 1. Writing Step Files

Create YAML files for reproducible E2E tests:

```yaml
session:
  command: vim
  args: [test.txt]
  cols: 120
  rows: 40

steps:
  - waitForText: {text: "VIM", timeoutMs: 5000}
  - press: {key: i}
  - type: {text: "Hello, world!"}
  - press: {key: Escape}
  - expectText: {text: "Hello, world!"}
  - screenshot: {name: vim-content}
  - type: {text: ":q!"}
  - press: {key: Enter}

artifacts:
  mode: always
  dir: ./test-artifacts
```

**Available Steps:**
| Step | Purpose | Example |
|------|---------|---------|
| `waitForText` | Wait for text to appear | `{text: "Ready", timeoutMs: 5000}` |
| `waitForPattern` | Wait for regex match | `{pattern: "error\|warning"}` |
| `waitForIdle` | Wait for screen stability | `{idleMs: 500, timeoutMs: 5000}` |
| `press` | Press a key | `{key: "Enter"}` |
| `type` | Type text | `{text: "hello"}` |
| `hotkey` | Modifier combo | `{ctrl: true, ch: "c"}` |
| `expectText` | Assert text exists | `{text: "Success"}` |
| `expectPattern` | Assert pattern matches | `{pattern: "completed"}` |
| `screenshot` | Capture PNG | `{name: "result"}` |

Note: `screenshot` steps require artifacts mode `onFailure` or `always` (not `off`).

For full schema details, see [step-schema.md](references/step-schema.md).

### 2. Daemon Protocol

The daemon uses JSON-over-Unix-socket communication:

**Request format:**
```json
{"id": 1, "method": "screen", "params": {"format": "text"}}
```

**Response format:**
```json
{"id": 1, "result": "screen content...", "error": null}
```

**Key Methods:**
- `handshake` - Initialize connection, get version info
- `screen` - Get screen content (text/json/json_compact)
- `screenshot` - Capture PNG (returns base64)
- `type` / `press` / `hotkey` - Input simulation
- `wait_for_text` / `wait_for_pattern` / `wait_for_idle` - Wait conditions
- `mouse_click` / `mouse_move` - Mouse events
- `status` - Check if process exited
- `close` - Terminate daemon

For complete protocol reference, see [protocol-reference.md](references/protocol-reference.md).

### 3. Analyzing Failures

When tests fail and artifacts are enabled (`onFailure` or `always`), examine the output directory:

```bash
# Directory structure after run
./termwright-artifacts/20250119-143022/
├── failure-001-screen.txt    # Screen text at failure
├── failure-001-screen.json   # Full screen state with colors
├── vim-content.png           # Screenshot step output (if used)
└── trace.json                # Step execution trace (only with --trace)
```

**Interpreting trace.json:**
```json
[
  {"step": 1, "action": "waitForText", "duration_ms": 245, "before_hash": 123, "after_hash": 456, "error": null},
  {"step": 2, "action": "press", "duration_ms": 15, "before_hash": 456, "after_hash": 789, "error": null},
  {"step": 3, "action": "expectText", "duration_ms": 3001, "before_hash": 789, "after_hash": 789, "error": "timeout waiting for text"}
]
```

- `before_hash` / `after_hash` - Detect if step changed screen
- `duration_ms` - Identify slow operations
- `error` - Null on success, message on failure

### 4. Parallel Testing with Hub

Run multiple TUI sessions simultaneously:

```bash
# Start 5 parallel sessions
termwright hub start --count 5 --output sessions.json -- ./my-app

# sessions.json contains socket paths for each
# Run tests against each socket in parallel

# Clean up all sessions
termwright hub stop --input sessions.json
```

## CLI Reference

| Command | Purpose |
|---------|---------|
| `termwright run` | One-shot capture (text or JSON) |
| `termwright screenshot` | One-shot PNG screenshot |
| `termwright run-steps` | Execute step file |
| `termwright exec` | Single daemon command |
| `termwright daemon` | Start long-lived daemon |
| `termwright hub start/stop` | Manage session pool |
| `termwright fonts` | List available fonts |

For all options, see [cli-reference.md](references/cli-reference.md).

## Shell Scripting Pattern

For custom test scripts without YAML:

```bash
#!/bin/bash
set -euo pipefail

# Start daemon
SOCK=$(termwright daemon --background -- htop)
trap "termwright exec --socket '$SOCK' --method close 2>/dev/null || true" EXIT

# Wait for socket
while ! [ -S "$SOCK" ]; do sleep 0.1; done

# Helper
tw() {
    termwright exec --socket "$SOCK" --method "$1" --params "${2:-null}"
}

# Test sequence
tw handshake
tw wait_for_idle '{"idle_ms":1000,"timeout_ms":10000}'

# Verify content
SCREEN=$(tw screen '{"format":"text"}' | jq -r '.result')
if echo "$SCREEN" | grep -q "CPU"; then
    echo "PASS: htop loaded"
else
    echo "FAIL: CPU info not found"
    exit 1
fi

# Screenshot
tw screenshot | jq -r '.result.png_base64' | base64 -d > htop.png
echo "Screenshot saved to htop.png"
```

## Valid Key Names

For `press` and step files:
- **Navigation:** `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`
- **Special:** `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Insert`
- **Function:** `F1` through `F12`
- **Characters:** Any single character (`a`, `A`, `1`, `@`, etc.)

## Coordinate System

All coordinates are 0-indexed:
- Row 0 = top line
- Column 0 = leftmost character
- Format: `(row, col)` for mouse operations

## Debugging Tips

1. **Start with `--trace`** - Always use trace output when developing tests
2. **Use `wait_for_idle`** - Ensure screen has stabilized before assertions
3. **Check `screen.txt`** - Compare expected vs actual content
4. **Inspect `screen.json`** - Verify cursor position and colors
5. **Lower timeouts during dev** - Faster feedback on failures
6. **Use `artifacts: always`** - Capture state at every step while debugging

## Common Patterns

### Wait for App Startup
```yaml
steps:
  - waitForIdle: {idleMs: 1000, timeoutMs: 10000}
  - waitForText: {text: "Ready", timeoutMs: 5000}
```

### Modal Dialog Handling
```yaml
steps:
  - press: {key: "Escape"}           # Close any open dialog
  - waitForIdle: {idleMs: 200}
  - hotkey: {ctrl: true, ch: "s"}    # Save
  - waitForText: {text: "Saved"}
```

### Navigation and Selection
```yaml
steps:
  - press: {key: "Down"}
  - press: {key: "Down"}
  - press: {key: "Enter"}
  - expectText: {text: "Selected item"}
```

### Form Input
```yaml
steps:
  - waitForText: {text: "Username:"}
  - type: {text: "admin"}
  - press: {key: "Tab"}
  - type: {text: "password123"}
  - press: {key: "Enter"}
```

## Reference Files

- [Protocol Reference](references/protocol-reference.md) - Complete daemon protocol
- [Step Schema](references/step-schema.md) - Full YAML/JSON step file format
- [CLI Reference](references/cli-reference.md) - All command options
- [Examples](references/examples.md) - Real-world test patterns

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Socket not found | Wait for socket file: `while ! [ -S "$SOCK" ]; do sleep 0.1; done` |
| Timeout on wait | Increase `timeoutMs`, check if text actually appears |
| Wrong screen content | Use `wait_for_idle` before assertions |
| Mouse not working | Not all TUIs support mouse; use keyboard navigation |
| Colors not captured | Use `screen` with `format: json` for full cell data |
