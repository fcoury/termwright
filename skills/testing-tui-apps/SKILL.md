---
name: testing-tui-apps
description: Tests TUI (Terminal User Interface) applications using termwright daemon mode. Use when testing terminal apps, CLI tools with interactive interfaces, ncurses apps, or any text-based UI. Triggers on requests to test, automate, or interact with terminal applications.
allowed-tools:
  - Bash
  - Read
  - Write
  - Grep
  - Glob
---

# Testing TUI Applications with Termwright

Automate and test terminal user interface applications using termwright's daemon mode and Unix socket communication.

## Quick Start

```bash
# 1. Start daemon with your TUI app
SOCK=$(termwright daemon --background -- your-app)

# 2. Send commands via Unix socket
echo '{"id":1,"method":"screen","params":{"format":"text"}}' | nc -U "$SOCK"

# 3. Close when done
echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
```

## Workflow

### Step 1: Start the Daemon

Launch termwright in daemon mode with your target application:

```bash
# Background mode (returns socket path)
SOCK=$(termwright daemon --background -- vim test.txt)

# With custom terminal size
SOCK=$(termwright daemon --background --cols 120 --rows 40 -- htop)

# Foreground mode (prints socket path, stays attached)
termwright daemon -- your-tui-app
```

### Step 2: Connect and Handshake

Always start with a handshake to verify connection:

```bash
echo '{"id":1,"method":"handshake","params":null}' | nc -U "$SOCK"
```

Response:
```json
{"id":1,"result":{"protocol_version":1,"termwright_version":"0.1.0","pid":12345},"error":null}
```

### Step 3: Wait for App Ready

Before interacting, wait for the app to be ready:

```bash
# Wait for specific text
echo '{"id":2,"method":"wait_for_text","params":{"text":"Ready","timeout_ms":5000}}' | nc -U "$SOCK"

# Wait for regex pattern
echo '{"id":2,"method":"wait_for_pattern","params":{"pattern":"\\[.*\\]","timeout_ms":5000}}' | nc -U "$SOCK"

# Wait for screen stability (no changes)
echo '{"id":2,"method":"wait_for_idle","params":{"idle_ms":500,"timeout_ms":5000}}' | nc -U "$SOCK"
```

### Step 4: Interact with the App

**Type text:**
```bash
echo '{"id":3,"method":"type","params":{"text":"hello world"}}' | nc -U "$SOCK"
```

**Press keys:**
```bash
echo '{"id":4,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"
echo '{"id":5,"method":"press","params":{"key":"Tab"}}' | nc -U "$SOCK"
echo '{"id":6,"method":"press","params":{"key":"Escape"}}' | nc -U "$SOCK"
echo '{"id":7,"method":"press","params":{"key":"Up"}}' | nc -U "$SOCK"
```

Available keys: `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`, `F1`-`F12`

**Hotkeys (Ctrl/Alt combinations):**
```bash
echo '{"id":8,"method":"hotkey","params":{"ctrl":true,"ch":"c"}}' | nc -U "$SOCK"
echo '{"id":9,"method":"hotkey","params":{"ctrl":true,"ch":"s"}}' | nc -U "$SOCK"
echo '{"id":10,"method":"hotkey","params":{"alt":true,"ch":"x"}}' | nc -U "$SOCK"
```

**Mouse events:**
```bash
# Move cursor to row 5, column 10
echo '{"id":11,"method":"mouse_move","params":{"row":5,"col":10}}' | nc -U "$SOCK"

# Click at position
echo '{"id":12,"method":"mouse_click","params":{"row":5,"col":10,"button":"left"}}' | nc -U "$SOCK"
```

### Step 5: Capture and Assert

**Get screen content:**
```bash
# Plain text
echo '{"id":20,"method":"screen","params":{"format":"text"}}' | nc -U "$SOCK"

# JSON with full cell data (colors, attributes)
echo '{"id":21,"method":"screen","params":{"format":"json"}}' | nc -U "$SOCK"

# Compact JSON
echo '{"id":22,"method":"screen","params":{"format":"json_compact"}}' | nc -U "$SOCK"
```

**Take screenshot:**
```bash
# Returns base64-encoded PNG
echo '{"id":23,"method":"screenshot","params":{}}' | nc -U "$SOCK"

# With custom font
echo '{"id":24,"method":"screenshot","params":{"font":"Menlo","font_size":14}}' | nc -U "$SOCK"
```

**Check process status:**
```bash
echo '{"id":25,"method":"status","params":null}' | nc -U "$SOCK"
```

### Step 6: Close the Session

```bash
echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
```

## JSON-RPC Protocol

All communication uses line-delimited JSON over Unix socket.

**Request format:**
```json
{"id": <number>, "method": "<string>", "params": <object|null>}
```

**Response format:**
```json
{"id": <number>, "result": <value>, "error": <object|null>}
```

Error object (when present):
```json
{"code": <number>, "message": "<string>"}
```

## Available Methods

| Method | Params | Description |
|--------|--------|-------------|
| `handshake` | none | Verify connection, get version info |
| `screen` | `format`: text/json/json_compact | Get screen content |
| `screenshot` | `font`, `font_size` (optional) | Get PNG screenshot (base64) |
| `status` | none | Get process exit status |
| `resize` | `cols`, `rows` | Resize terminal |
| `type` | `text` | Type text string |
| `press` | `key` | Press a key |
| `hotkey` | `ctrl`/`alt` (bool), `ch` (char) | Send Ctrl/Alt combo |
| `raw` | `bytes` (base64) | Send raw bytes |
| `mouse_move` | `row`, `col` | Move mouse cursor |
| `mouse_click` | `row`, `col`, `button` | Click mouse button |
| `wait_for_text` | `text`, `timeout_ms` | Wait for text |
| `wait_for_pattern` | `pattern`, `timeout_ms` | Wait for regex |
| `wait_for_idle` | `idle_ms`, `timeout_ms` | Wait for stability |
| `wait_for_exit` | `timeout_ms` | Wait for process exit |
| `close` | none | Close daemon and app |

## Examples

### Testing a vim Session

```bash
# Start vim
SOCK=$(termwright daemon --background -- vim test.txt)

# Wait for vim to load
echo '{"id":1,"method":"wait_for_text","params":{"text":"VIM","timeout_ms":5000}}' | nc -U "$SOCK"

# Enter insert mode
echo '{"id":2,"method":"press","params":{"key":"i"}}' | nc -U "$SOCK"

# Type some text
echo '{"id":3,"method":"type","params":{"text":"Hello, World!"}}' | nc -U "$SOCK"

# Exit insert mode
echo '{"id":4,"method":"press","params":{"key":"Escape"}}' | nc -U "$SOCK"

# Save and quit
echo '{"id":5,"method":"type","params":{"text":":wq"}}' | nc -U "$SOCK"
echo '{"id":6,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

# Wait for exit
echo '{"id":7,"method":"wait_for_exit","params":{"timeout_ms":2000}}' | nc -U "$SOCK"

# Clean up
echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
```

### Testing htop Navigation

```bash
SOCK=$(termwright daemon --background --cols 120 --rows 40 -- htop)

# Wait for htop to render
echo '{"id":1,"method":"wait_for_idle","params":{"idle_ms":500,"timeout_ms":5000}}' | nc -U "$SOCK"

# Press F6 for sort menu
echo '{"id":2,"method":"press","params":{"key":"F6"}}' | nc -U "$SOCK"

# Navigate and select
echo '{"id":3,"method":"press","params":{"key":"Down"}}' | nc -U "$SOCK"
echo '{"id":4,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

# Take screenshot for verification
RESPONSE=$(echo '{"id":5,"method":"screenshot","params":{}}' | nc -U "$SOCK")
echo "$RESPONSE" | jq -r '.result' | base64 -d > htop-sorted.png

# Quit
echo '{"id":6,"method":"press","params":{"key":"q"}}' | nc -U "$SOCK"
echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
```

### Asserting Screen Content

```bash
# Get screen text and check for expected content
SCREEN=$(echo '{"id":1,"method":"screen","params":{"format":"text"}}' | nc -U "$SOCK" | jq -r '.result')

if echo "$SCREEN" | grep -q "Expected Text"; then
    echo "PASS: Found expected text"
else
    echo "FAIL: Expected text not found"
    exit 1
fi
```

## Platform Notes

**macOS:** Use `socat` instead of `nc -U` for more reliable Unix socket communication:
```bash
# Install socat if needed
brew install socat

# Use socat instead of nc
echo '{"id":1,"method":"screen","params":{"format":"text"}}' | socat - UNIX-CONNECT:"$SOCK"
```

**Multi-client support:** The daemon accepts multiple sequential client connections. Each command can be a separate process - the daemon only exits when:
- A `close` command is received
- The spawned TUI process exits

## Guidelines

- Always start with `handshake` to verify connection
- Use `wait_for_text` or `wait_for_idle` before asserting screen content
- Increment request IDs for easier debugging
- Always call `close` to clean up the daemon process
- Use `--background` flag to get the socket path directly
- Screen coordinates are 0-indexed (row 0 is the top)

## Troubleshooting

**Connection refused:**
- Verify the socket path exists: `ls -la /tmp/termwright-*.sock`
- Check if daemon is running: `ps aux | grep termwright`

**App not responding:**
- Increase timeout values
- Check if app requires specific terminal size
- Verify app supports the input method (some apps ignore mouse events)

**Screen content empty:**
- Wait for app to render with `wait_for_idle`
- Some apps need time after launch before accepting input

## References

For detailed documentation, see:

- [Protocol Reference](references/protocol-reference.md) - Complete JSON-RPC protocol specification
- [Testing Examples](references/testing-examples.md) - Real-world testing scenarios and patterns
