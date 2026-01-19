# Termwright CLI Reference

Complete reference for all Termwright CLI commands and options.

## Global Usage

```
termwright <COMMAND> [OPTIONS]
```

## Commands

### `termwright run`

Run a command and capture output.

```bash
termwright run [OPTIONS] -- <COMMAND> [ARGS...]
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--cols <N>` | 80 | Terminal width |
| `--rows <N>` | 24 | Terminal height |
| `--wait-for <TEXT>` | - | Wait for text before capture |
| `--delay <MS>` | 500 | Delay before capture |
| `--format <FMT>` | text | Output: `text`, `json`, `json-compact` |
| `--timeout <SECS>` | 30 | Timeout for wait conditions |

**Examples:**

```bash
# Capture htop screen as text
termwright run --delay 1000 -- htop

# Capture vim with JSON format
termwright run --cols 120 --wait-for "VIM" --format json -- vim test.txt

# Quick capture with minimal delay
termwright run --delay 100 --format json-compact -- ./my-app
```

---

### `termwright screenshot`

Take a PNG screenshot.

```bash
termwright screenshot [OPTIONS] -- <COMMAND> [ARGS...]
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--cols <N>` | 80 | Terminal width |
| `--rows <N>` | 24 | Terminal height |
| `--wait-for <TEXT>` | - | Wait for text before capture |
| `--delay <MS>` | 500 | Delay before capture |
| `-o, --output <PATH>` | stdout | Output file path |
| `--font <NAME>` | system | Font name |
| `--font-size <SIZE>` | 14 | Font size in pixels |
| `--timeout <SECS>` | 30 | Timeout for wait conditions |

**Examples:**

```bash
# Screenshot htop
termwright screenshot -o htop.png -- htop

# Wait for app to load, then screenshot
termwright screenshot --wait-for "Ready" -o app.png -- ./my-app

# High-res screenshot with custom font
termwright screenshot --font "JetBrains Mono" --font-size 16 -o result.png -- vim
```

---

### `termwright run-steps`

Execute a YAML/JSON step file.

```bash
termwright run-steps [OPTIONS] <FILE>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<FILE>` | Path to YAML or JSON steps file |

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--connect <PATH>` | - | Connect to existing daemon socket |
| `--trace` | false | Record trace to artifacts |

**Examples:**

```bash
# Run step file
termwright run-steps test.yaml

# Run with trace output
termwright run-steps --trace integration-test.yaml

# Connect to existing daemon
termwright run-steps --connect /tmp/termwright-123.sock steps.yaml
```

---

### `termwright exec`

Execute a single daemon command.

```bash
termwright exec [OPTIONS]
```

**Options:**

| Option | Required | Description |
|--------|----------|-------------|
| `--socket <PATH>` | Yes | Unix socket path |
| `--method <NAME>` | Yes | Method name |
| `--params <JSON>` | No | Parameters (default: null) |

**Examples:**

```bash
# Handshake
termwright exec --socket /tmp/tw.sock --method handshake

# Get screen text
termwright exec --socket /tmp/tw.sock --method screen --params '{"format":"text"}'

# Press key
termwright exec --socket /tmp/tw.sock --method press --params '{"key":"Enter"}'

# Type text
termwright exec --socket /tmp/tw.sock --method type --params '{"text":"hello"}'

# Wait for text
termwright exec --socket /tmp/tw.sock --method wait_for_text --params '{"text":"Ready","timeout_ms":5000}'

# Screenshot (returns base64)
termwright exec --socket /tmp/tw.sock --method screenshot

# Close daemon
termwright exec --socket /tmp/tw.sock --method close
```

---

### `termwright daemon`

Start a long-lived daemon.

```bash
termwright daemon [OPTIONS] -- <COMMAND> [ARGS...]
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--cols <N>` | 80 | Terminal width |
| `--rows <N>` | 24 | Terminal height |
| `--socket <PATH>` | auto | Unix socket path |
| `--background` | false | Run in background |

**Output:** Prints socket path to stdout.

**Examples:**

```bash
# Start daemon in foreground
termwright daemon -- vim test.txt
# Socket path printed, daemon runs until Ctrl+C

# Start in background
SOCK=$(termwright daemon --background -- vim test.txt)
echo "Socket: $SOCK"

# Custom dimensions
termwright daemon --cols 120 --rows 40 --background -- htop

# Specify socket path
termwright daemon --socket /tmp/my-session.sock -- ./my-app
```

---

### `termwright hub start`

Start multiple daemon sessions.

```bash
termwright hub start [OPTIONS] -- <COMMAND> [ARGS...]
```

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--count <N>` | 1 | Number of sessions |
| `--cols <N>` | 80 | Terminal width |
| `--rows <N>` | 24 | Terminal height |
| `--output <FILE>` | - | Write session info to JSON |

**Output:** JSON array of session entries.

**Examples:**

```bash
# Start 5 parallel sessions
termwright hub start --count 5 --output sessions.json -- ./my-app

# Start with custom dimensions
termwright hub start --count 3 --cols 120 --rows 40 -- vim
```

**Session JSON format:**

```json
[
  {"socket": "/tmp/termwright-12345.sock", "pid": 12345},
  {"socket": "/tmp/termwright-12346.sock", "pid": 12346}
]
```

---

### `termwright hub stop`

Stop daemon sessions.

```bash
termwright hub stop [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--socket <PATH>...` | Socket paths to close |
| `--input <FILE>` | Load sockets from hub JSON |

**Examples:**

```bash
# Stop specific sockets
termwright hub stop --socket /tmp/tw-1.sock --socket /tmp/tw-2.sock

# Stop all from session file
termwright hub stop --input sessions.json
```

---

### `termwright fonts`

List available monospace fonts.

```bash
termwright fonts
```

**Output:** List of font names suitable for screenshots.

**Example:**

```bash
$ termwright fonts
Menlo
Monaco
SF Mono
JetBrains Mono
Fira Code
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (parsing, timeout, or execution failure) |

## Tips

1. **Use `--background`** - Returns immediately with socket path
2. **Capture socket path** - `SOCK=$(termwright daemon --background ...)`
3. **Always close daemons** - Use `termwright exec --method close` or `hub stop`
4. **Use `--trace` in CI** - Debug failures with execution trace
5. **Prefer step files** - More maintainable than shell scripts
