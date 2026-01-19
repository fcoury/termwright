# Termwright Daemon Protocol Reference

Complete reference for the JSON-over-Unix-socket protocol.

## Connection

- **Transport:** Unix domain socket
- **Format:** Line-delimited JSON (one object per line, newline terminated)
- **Encoding:** UTF-8
- **Persistence:** Multiple sequential clients supported

## Message Format

### Request

```json
{
  "id": 1,
  "method": "method_name",
  "params": {...} | null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | u64 | Yes | Unique request identifier |
| `method` | string | Yes | Method name |
| `params` | object/null | Yes | Method parameters |

### Response

```json
{
  "id": 1,
  "result": {...} | null,
  "error": null | {"code": "error_code", "message": "description"}
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | u64 | Matches request id |
| `result` | any | Method result (null on error) |
| `error` | object/null | Error details if failed |

## Methods

### Connection Management

#### `handshake`

Initialize connection and get daemon info.

**Params:** `null`

**Response:**
```json
{
  "protocol_version": 1,
  "termwright_version": "0.1.1",
  "pid": 12345
}
```

#### `status`

Check if child process has exited.

**Params:** `null`

**Response:**
```json
{
  "exited": false,
  "exit_code": null
}
```

Or when exited:
```json
{
  "exited": true,
  "exit_code": 0
}
```

#### `close`

Terminate daemon and child process.

**Params:** `null`

**Response:** `null` with error code `"closing"`

---

### Screen Operations

#### `screen`

Get current screen content.

**Params:**
```json
{
  "format": "text" | "json" | "json_compact"
}
```

**Response (text):** Plain text string

**Response (json):** Full screen with cell details:
```json
{
  "size": {"cols": 80, "rows": 24},
  "cursor": {"row": 0, "col": 0},
  "cells": [
    [
      {
        "char": "H",
        "fg": {"type": "Default"},
        "bg": {"type": "Indexed", "value": 0},
        "attrs": {
          "bold": false,
          "italic": false,
          "underline": false,
          "inverse": false
        }
      }
    ]
  ]
}
```

**Response (json_compact):**
```json
{
  "size": {"cols": 80, "rows": 24},
  "cursor": {"row": 0, "col": 0},
  "lines": ["line 1", "line 2", ...]
}
```

**Color Types:**
- `{"type": "Default"}` - Terminal default
- `{"type": "Indexed", "value": 0..255}` - Palette color
- `{"type": "Rgb", "value": [r, g, b]}` - True color

#### `screenshot`

Capture PNG screenshot.

**Params:**
```json
{
  "font": "Menlo",       // optional
  "font_size": 14.0,     // optional, default: 14
  "line_height": 1.2     // optional
}
```

**Response:**
```json
{
  "png_base64": "iVBORw0KGgo..."
}
```

#### `resize`

Resize terminal dimensions.

**Params:**
```json
{
  "cols": 120,
  "rows": 40
}
```

**Response:** `null`

---

### Input Simulation

#### `type`

Type a text string.

**Params:**
```json
{
  "text": "hello world"
}
```

**Response:** `null`

#### `press`

Press a single key.

**Params:**
```json
{
  "key": "Enter"
}
```

**Valid Keys:**
- Navigation: `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`
- Special: `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Insert`
- Function: `F1` through `F12`
- Characters: Any single character

**Response:** `null`

#### `hotkey`

Send modifier key combination.

**Params:**
```json
{
  "ctrl": true,   // optional
  "alt": false,   // optional
  "ch": "c"       // single character
}
```

**Response:** `null`

**Examples:**
- `{"ctrl": true, "ch": "c"}` - Ctrl+C
- `{"alt": true, "ch": "x"}` - Alt+X
- `{"ctrl": true, "alt": true, "ch": "d"}` - Ctrl+Alt+D

#### `raw`

Send raw bytes.

**Params:**
```json
{
  "bytes_base64": "SGVsbG8="
}
```

**Response:** `null`

---

### Mouse Events

#### `mouse_move`

Move mouse cursor.

**Params:**
```json
{
  "row": 5,
  "col": 10,
  "buttons": null | ["left", "middle", "right"]
}
```

**Response:** `null`

#### `mouse_click`

Click at position.

**Params:**
```json
{
  "row": 5,
  "col": 10,
  "button": "left" | "middle" | "right"
}
```

**Response:** `null`

---

### Wait Conditions

#### `wait_for_text`

Wait for text to appear.

**Params:**
```json
{
  "text": "Ready",
  "timeout_ms": 5000   // optional
}
```

**Response:** `null`

#### `wait_for_pattern`

Wait for regex pattern.

**Params:**
```json
{
  "pattern": "error|warning",
  "timeout_ms": 5000   // optional
}
```

**Response:** `null`

#### `wait_for_idle`

Wait for screen stability.

**Params:**
```json
{
  "idle_ms": 500,
  "timeout_ms": 5000   // optional
}
```

**Response:** `null`

Waits for `idle_ms` with no screen changes.

#### `wait_for_exit`

Wait for child process exit.

**Params:**
```json
{
  "timeout_ms": 5000   // optional
}
```

**Response:**
```json
{
  "exit_code": 0
}
```

---

## Error Codes

| Code | Meaning |
|------|---------|
| `closing` | Daemon is shutting down |
| `timeout` | Wait condition timed out |
| `invalid_params` | Malformed parameters |
| `child_exited` | Child process already exited |

## Example Session

```bash
# Connect and send messages
$ echo '{"id":1,"method":"handshake","params":null}' | nc -U /tmp/termwright-123.sock
{"id":1,"result":{"protocol_version":1,"termwright_version":"0.1.1","pid":12345},"error":null}

$ echo '{"id":2,"method":"wait_for_text","params":{"text":"Ready","timeout_ms":5000}}' | nc -U /tmp/termwright-123.sock
{"id":2,"result":null,"error":null}

$ echo '{"id":3,"method":"press","params":{"key":"Enter"}}' | nc -U /tmp/termwright-123.sock
{"id":3,"result":null,"error":null}

$ echo '{"id":4,"method":"screen","params":{"format":"text"}}' | nc -U /tmp/termwright-123.sock
{"id":4,"result":"Welcome to the app\n> ","error":null}
```
