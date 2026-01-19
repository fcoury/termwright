# Step File Schema Reference

Complete reference for YAML/JSON step files used with `termwright run-steps`.

## File Format

Step files use YAML (recommended) or JSON format:

```yaml
session:
  # Session configuration

steps:
  # List of test steps

artifacts:
  # Output configuration
```

## Session Configuration

```yaml
session:
  command: vim                    # Required: string or array
  args: [test.txt]               # Optional: arguments
  cols: 120                       # Optional: width (default: 80)
  rows: 40                        # Optional: height (default: 24)
  env:                            # Optional: environment
    TERM: xterm-256color
    EDITOR: vim
  cwd: /home/user                 # Optional: working directory
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `command` | string/array | Yes | - | Command to run |
| `args` | array | No | `[]` | Arguments (ignored if command is array) |
| `cols` | integer | No | 80 | Terminal width |
| `rows` | integer | No | 24 | Terminal height |
| `env` | object | No | `{}` | Environment variables |
| `cwd` | string | No | current | Working directory |

### Command Formats

**String format:**
```yaml
command: vim
args: [test.txt]
# Equivalent to: vim test.txt
```

**Array format:**
```yaml
command: ["sh", "-c", "echo hello && vim"]
# args is ignored when command is array
```

## Steps

Each step is an object with a single key indicating the action.

### Wait Steps

#### `waitForText`

Wait for exact text to appear anywhere on screen.

```yaml
- waitForText:
    text: "Ready"
    timeoutMs: 5000    # Optional, default varies
```

#### `waitForPattern`

Wait for regex pattern to match.

```yaml
- waitForPattern:
    pattern: "error|warning|failed"
    timeoutMs: 5000
```

#### `waitForIdle`

Wait for screen to stop changing.

```yaml
- waitForIdle:
    idleMs: 500        # How long screen must be stable
    timeoutMs: 5000    # Max wait time
```

### Input Steps

#### `press`

Press a single key.

```yaml
- press: {key: Enter}
- press: {key: Escape}
- press: {key: Tab}
- press: {key: F1}
- press: {key: Down}
```

**Valid keys:**
- Navigation: `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`
- Special: `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Insert`
- Function: `F1` - `F12`
- Characters: `a`, `A`, `1`, `@`, etc.

#### `type`

Type a text string.

```yaml
- type: {text: "Hello, world!"}
- type: {text: ":wq"}
```

#### `hotkey`

Send modifier key combination.

```yaml
- hotkey: {ctrl: true, ch: c}        # Ctrl+C
- hotkey: {alt: true, ch: x}         # Alt+X
- hotkey: {ctrl: true, alt: true, ch: d}  # Ctrl+Alt+D
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `ctrl` | boolean | false | Hold Ctrl |
| `alt` | boolean | false | Hold Alt |
| `ch` | string | Required | Single character |

### Assertion Steps

#### `expectText`

Assert text appears (fails if not found within timeout).

```yaml
- expectText:
    text: "Success"
    timeoutMs: 3000
```

Functionally equivalent to `waitForText` but semantically indicates an assertion.

#### `expectPattern`

Assert regex pattern matches.

```yaml
- expectPattern:
    pattern: "completed|done"
    timeoutMs: 3000
```

### Capture Steps

#### `screenshot`

Capture PNG screenshot to artifacts directory.

```yaml
- screenshot: {}                      # Auto-named: step-001-screenshot.png
- screenshot: {name: "final-result"}  # Named: final-result.png
```

## Artifacts Configuration

```yaml
artifacts:
  mode: onFailure     # When to save artifacts
  dir: ./artifacts    # Output directory
```

### Mode Options

| Mode | Behavior |
|------|----------|
| `onFailure` | Save artifacts only when test fails (default) |
| `always` | Save artifacts after every step |
| `off` | Disable artifacts entirely (screenshots require non-off mode) |

### Output Structure

```
{dir}/{YYYYMMDD-HHMMSS}/
├── step-001-screen.txt       # Saved for mode=always
├── step-001-screen.json
├── failure-001-screen.txt    # Saved for mode=onFailure
├── failure-001-screen.json
├── my-screenshot.png         # Named screenshot
├── trace.json                # Only when run-steps --trace is set
```


## Complete Example

```yaml
# test-vim-editing.yaml
session:
  command: vim
  args: [test.txt]
  cols: 120
  rows: 40
  env:
    TERM: xterm-256color

steps:
  # Wait for vim to load
  - waitForText: {text: "VIM", timeoutMs: 5000}

  # Enter insert mode
  - press: {key: i}
  - waitForText: {text: "-- INSERT --"}

  # Type content
  - type: {text: "Hello, Termwright!"}

  # Exit insert mode
  - press: {key: Escape}

  # Verify content
  - expectText: {text: "Hello, Termwright!", timeoutMs: 3000}

  # Take screenshot
  - screenshot: {name: vim-with-content}

  # Save and quit
  - type: {text: ":wq"}
  - press: {key: Enter}

artifacts:
  mode: always
  dir: ./test-output
```

## JSON Format

Equivalent JSON format:

```json
{
  "session": {
    "command": "vim",
    "args": ["test.txt"],
    "cols": 120,
    "rows": 40
  },
  "steps": [
    {"waitForText": {"text": "VIM", "timeoutMs": 5000}},
    {"press": {"key": "i"}},
    {"type": {"text": "Hello"}},
    {"press": {"key": "Escape"}},
    {"expectText": {"text": "Hello"}},
    {"screenshot": {"name": "result"}}
  ],
  "artifacts": {
    "mode": "always",
    "dir": "./output"
  }
}
```

## Tips

1. **Always start with a wait** - Use `waitForText` or `waitForIdle` to ensure app is ready
2. **Use `waitForIdle` between fast actions** - Prevent race conditions
3. **Name important screenshots** - Easier to find in artifacts
4. **Use `artifacts: always` during development** - See state at every step
5. **Keep timeouts reasonable** - Lower during development, higher in CI
