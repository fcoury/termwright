# Termwright Protocol Reference

Complete specification for the termwright daemon JSON-RPC protocol.

## Connection

**Transport:** Unix domain socket
**Format:** Line-delimited JSON (one JSON object per line)
**Encoding:** UTF-8

## Request Structure

```json
{
  "id": 1,
  "method": "method_name",
  "params": { ... } | null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | u64 | Yes | Unique request identifier |
| `method` | string | Yes | Method name to invoke |
| `params` | object/null | Yes | Method parameters |

## Response Structure

```json
{
  "id": 1,
  "result": { ... } | null,
  "error": null | { "code": -1, "message": "error description" }
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

Verify connection and get version information.

**Params:** `null`

**Result:**
```json
{
  "protocol_version": 1,
  "termwright_version": "0.1.0",
  "pid": 12345
}
```

#### `close`

Close the daemon and terminate the child process.

**Params:** `null`

**Result:** `null`

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

**Result (text format):** String containing screen text

**Result (json format):**
```json
{
  "size": { "cols": 80, "rows": 24 },
  "cursor": { "row": 0, "col": 0 },
  "cells": [
    [
      {
        "char": "H",
        "fg": { "type": "default" } | { "type": "indexed", "value": 255 } | { "type": "rgb", "r": 255, "g": 255, "b": 255 },
        "bg": { ... },
        "bold": false,
        "italic": false,
        "underline": false,
        "inverse": false
      }
    ]
  ]
}
```

#### `screenshot`

Capture PNG screenshot.

**Params:**
```json
{
  "font": "Menlo",       // optional, default: system monospace
  "font_size": 14        // optional, default: 14
}
```

**Result:** Base64-encoded PNG data (string)

#### `resize`

Resize terminal dimensions.

**Params:**
```json
{
  "cols": 120,
  "rows": 40
}
```

**Result:** `null`

#### `status`

Get process status.

**Params:** `null`

**Result:**
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

**Result:** `null`

#### `press`

Press a single key.

**Params:**
```json
{
  "key": "Enter"
}
```

**Valid keys:**
- `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`
- `Up`, `Down`, `Left`, `Right`
- `Home`, `End`, `PageUp`, `PageDown`
- `F1` through `F12`
- Single characters: `a`, `A`, `1`, `@`, etc.

**Result:** `null`

#### `hotkey`

Send modifier key combination.

**Params:**
```json
{
  "modifier": "ctrl" | "alt",
  "key": "c"
}
```

**Result:** `null`

#### `raw`

Send raw bytes.

**Params:**
```json
{
  "bytes": "base64-encoded-data"
}
```

**Result:** `null`

---

### Mouse Events

#### `mouse_move`

Move mouse cursor to position.

**Params:**
```json
{
  "row": 5,
  "col": 10
}
```

**Result:** `null`

#### `mouse_click`

Click mouse button at position.

**Params:**
```json
{
  "row": 5,
  "col": 10,
  "button": "left" | "middle" | "right"
}
```

**Result:** `null`

---

### Wait Conditions

All wait methods accept an optional `timeout_ms` parameter (default: 30000ms).

#### `wait_for_text`

Wait for text to appear on screen.

**Params:**
```json
{
  "text": "Ready",
  "timeout_ms": 5000
}
```

**Result:**
```json
{
  "found": true,
  "position": { "row": 5, "col": 10 }
}
```

#### `wait_for_pattern`

Wait for regex pattern to match.

**Params:**
```json
{
  "pattern": "error|warning",
  "timeout_ms": 5000
}
```

**Result:**
```json
{
  "found": true,
  "matched": "error",
  "position": { "row": 10, "col": 0 }
}
```

#### `wait_for_idle`

Wait for screen to stabilize (no changes).

**Params:**
```json
{
  "duration_ms": 500,
  "timeout_ms": 5000
}
```

**Result:** `null`

#### `wait_for_exit`

Wait for child process to exit.

**Params:**
```json
{
  "timeout_ms": 5000
}
```

**Result:**
```json
{
  "exit_code": 0
}
```

---

## Error Codes

| Code | Description |
|------|-------------|
| -1 | General error |
| -2 | Timeout |
| -3 | Process already exited |
| -4 | Invalid parameters |
| -5 | Method not found |

## Screen Coordinate System

```
     col 0  col 1  col 2  ...  col N
    +------+------+------+----+------+
row 0|  H  |  e  |  l  |  l  |  o  |
    +------+------+------+----+------+
row 1|  W  |  o  |  r  |  l  |  d  |
    +------+------+------+----+------+
...
```

- Coordinates are 0-indexed
- Row 0 is the top line
- Column 0 is the leftmost character

## Color Types

**Default:**
```json
{ "type": "default" }
```

**Indexed (0-255):**
```json
{ "type": "indexed", "value": 196 }
```

**RGB:**
```json
{ "type": "rgb", "r": 255, "g": 128, "b": 0 }
```

## Cell Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `bold` | bool | Bold text |
| `italic` | bool | Italic text |
| `underline` | bool | Underlined text |
| `inverse` | bool | Inverted colors |
