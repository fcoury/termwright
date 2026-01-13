# TUI Testing Examples

Real-world examples for testing common terminal applications.

## Bash Script Helper

Create a helper script for cleaner test writing:

```bash
#!/bin/bash
# termwright-test.sh - Helper for TUI testing

SOCK=""

tw_start() {
    local cmd="$1"
    shift
    SOCK=$(termwright daemon --background -- "$cmd" "$@")
    echo "Started: $SOCK"
}

tw_send() {
    local method="$1"
    local params="$2"
    local id="${3:-1}"
    echo "{\"id\":$id,\"method\":\"$method\",\"params\":$params}" | nc -U "$SOCK"
}

tw_handshake() {
    tw_send "handshake" "null"
}

tw_type() {
    tw_send "type" "{\"text\":\"$1\"}"
}

tw_press() {
    tw_send "press" "{\"key\":\"$1\"}"
}

tw_ctrl() {
    tw_send "hotkey" "{\"modifier\":\"ctrl\",\"key\":\"$1\"}"
}

tw_wait_text() {
    tw_send "wait_for_text" "{\"text\":\"$1\",\"timeout_ms\":${2:-5000}}"
}

tw_wait_idle() {
    tw_send "wait_for_idle" "{\"duration_ms\":${1:-500},\"timeout_ms\":${2:-5000}}"
}

tw_screen() {
    tw_send "screen" "{\"format\":\"text\"}" | jq -r '.result'
}

tw_close() {
    tw_send "close" "null"
}

tw_assert_contains() {
    local expected="$1"
    local screen=$(tw_screen)
    if echo "$screen" | grep -q "$expected"; then
        echo "PASS: Found '$expected'"
        return 0
    else
        echo "FAIL: Expected '$expected' not found"
        echo "Screen content:"
        echo "$screen"
        return 1
    fi
}
```

Usage:
```bash
source termwright-test.sh

tw_start vim test.txt
tw_handshake
tw_wait_text "VIM" 5000
tw_press "i"
tw_type "Hello, World!"
tw_press "Escape"
tw_assert_contains "Hello, World!"
tw_close
```

---

## Testing a Custom CLI App

### Scenario: Test a Todo App

```bash
#!/bin/bash
set -e

SOCK=$(termwright daemon --background -- ./todo-app)

# Helper
send() {
    echo "$1" | nc -U "$SOCK"
}

# Wait for app to start
send '{"id":1,"method":"wait_for_text","params":{"text":"Todo List","timeout_ms":5000}}'

# Add a new todo
send '{"id":2,"method":"press","params":{"key":"a"}}'  # 'a' to add
send '{"id":3,"method":"wait_for_text","params":{"text":"New todo:","timeout_ms":2000}}'
send '{"id":4,"method":"type","params":{"text":"Write tests"}}'
send '{"id":5,"method":"press","params":{"key":"Enter"}}'

# Verify todo was added
send '{"id":6,"method":"wait_for_idle","params":{"duration_ms":200,"timeout_ms":2000}}'
SCREEN=$(send '{"id":7,"method":"screen","params":{"format":"text"}}' | jq -r '.result')

if echo "$SCREEN" | grep -q "Write tests"; then
    echo "PASS: Todo was added"
else
    echo "FAIL: Todo not found in list"
    exit 1
fi

# Mark as complete
send '{"id":8,"method":"press","params":{"key":"Enter"}}'  # Select
send '{"id":9,"method":"press","params":{"key":"c"}}'      # Complete

# Verify completion indicator
send '{"id":10,"method":"wait_for_idle","params":{"duration_ms":200,"timeout_ms":2000}}'
SCREEN=$(send '{"id":11,"method":"screen","params":{"format":"text"}}' | jq -r '.result')

if echo "$SCREEN" | grep -q "\[x\].*Write tests"; then
    echo "PASS: Todo marked complete"
else
    echo "FAIL: Completion indicator not found"
    exit 1
fi

# Clean up
send '{"id":99,"method":"close","params":null}'
echo "All tests passed!"
```

---

## Testing ncurses Applications

### Scenario: Test Midnight Commander (mc)

```bash
SOCK=$(termwright daemon --background --cols 120 --rows 40 -- mc)

# Wait for MC to render
echo '{"id":1,"method":"wait_for_idle","params":{"duration_ms":1000,"timeout_ms":10000}}' | nc -U "$SOCK"

# Navigate to a directory - press F7 for mkdir
echo '{"id":2,"method":"press","params":{"key":"F7"}}' | nc -U "$SOCK"
echo '{"id":3,"method":"wait_for_text","params":{"text":"Create a new Directory","timeout_ms":2000}}' | nc -U "$SOCK"

# Type directory name
echo '{"id":4,"method":"type","params":{"text":"test-dir"}}' | nc -U "$SOCK"
echo '{"id":5,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

# Verify directory appears in listing
echo '{"id":6,"method":"wait_for_idle","params":{"duration_ms":500,"timeout_ms":5000}}' | nc -U "$SOCK"
SCREEN=$(echo '{"id":7,"method":"screen","params":{"format":"text"}}' | nc -U "$SOCK" | jq -r '.result')

if echo "$SCREEN" | grep -q "test-dir"; then
    echo "PASS: Directory created and visible"
fi

# Exit MC (F10)
echo '{"id":8,"method":"press","params":{"key":"F10"}}' | nc -U "$SOCK"
echo '{"id":9,"method":"wait_for_text","params":{"text":"exit","timeout_ms":2000}}' | nc -U "$SOCK"
echo '{"id":10,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
```

---

## Testing with Screenshots

### Scenario: Visual Regression Test

```bash
#!/bin/bash

SOCK=$(termwright daemon --background -- htop)

# Wait for render
echo '{"id":1,"method":"wait_for_idle","params":{"duration_ms":1000,"timeout_ms":10000}}' | nc -U "$SOCK"

# Capture baseline
SCREENSHOT=$(echo '{"id":2,"method":"screenshot","params":{"font":"Menlo","font_size":12}}' | nc -U "$SOCK" | jq -r '.result')

echo "$SCREENSHOT" | base64 -d > screenshots/htop-current.png

# Compare with baseline (using ImageMagick)
if [ -f screenshots/htop-baseline.png ]; then
    DIFF=$(compare -metric AE screenshots/htop-baseline.png screenshots/htop-current.png null: 2>&1)
    if [ "$DIFF" -lt 1000 ]; then
        echo "PASS: Screenshot matches baseline (diff: $DIFF pixels)"
    else
        echo "WARN: Screenshot differs from baseline (diff: $DIFF pixels)"
        compare screenshots/htop-baseline.png screenshots/htop-current.png screenshots/htop-diff.png
    fi
else
    echo "INFO: No baseline found, saving current as baseline"
    cp screenshots/htop-current.png screenshots/htop-baseline.png
fi

echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
```

---

## Testing Interactive Prompts

### Scenario: Test a CLI Wizard

```bash
SOCK=$(termwright daemon --background -- ./setup-wizard)

# Page 1: Welcome
echo '{"id":1,"method":"wait_for_text","params":{"text":"Welcome","timeout_ms":5000}}' | nc -U "$SOCK"
echo '{"id":2,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

# Page 2: Enter name
echo '{"id":3,"method":"wait_for_text","params":{"text":"Enter your name","timeout_ms":2000}}' | nc -U "$SOCK"
echo '{"id":4,"method":"type","params":{"text":"Test User"}}' | nc -U "$SOCK"
echo '{"id":5,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

# Page 3: Select option (use arrows)
echo '{"id":6,"method":"wait_for_text","params":{"text":"Select an option","timeout_ms":2000}}' | nc -U "$SOCK"
echo '{"id":7,"method":"press","params":{"key":"Down"}}' | nc -U "$SOCK"
echo '{"id":8,"method":"press","params":{"key":"Down"}}' | nc -U "$SOCK"
echo '{"id":9,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

# Page 4: Confirmation
echo '{"id":10,"method":"wait_for_text","params":{"text":"Confirm","timeout_ms":2000}}' | nc -U "$SOCK"
echo '{"id":11,"method":"type","params":{"text":"y"}}' | nc -U "$SOCK"
echo '{"id":12,"method":"press","params":{"key":"Enter"}}' | nc -U "$SOCK"

# Verify success
echo '{"id":13,"method":"wait_for_text","params":{"text":"Success","timeout_ms":5000}}' | nc -U "$SOCK"

echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
echo "Wizard test passed!"
```

---

## Testing with JSON Screen Data

### Scenario: Verify Specific Cell Colors

```bash
SOCK=$(termwright daemon --background -- ./colored-output-app)

echo '{"id":1,"method":"wait_for_idle","params":{"duration_ms":500,"timeout_ms":5000}}' | nc -U "$SOCK"

# Get full JSON screen data
SCREEN_JSON=$(echo '{"id":2,"method":"screen","params":{"format":"json"}}' | nc -U "$SOCK" | jq -r '.result')

# Check if error text is red (indexed color 1 or RGB red)
ERROR_CELL=$(echo "$SCREEN_JSON" | jq '.cells[5][0]')  # Row 5, Col 0

FG_TYPE=$(echo "$ERROR_CELL" | jq -r '.fg.type')
if [ "$FG_TYPE" = "indexed" ]; then
    FG_VALUE=$(echo "$ERROR_CELL" | jq '.fg.value')
    if [ "$FG_VALUE" = "1" ] || [ "$FG_VALUE" = "9" ]; then
        echo "PASS: Error text is red"
    fi
elif [ "$FG_TYPE" = "rgb" ]; then
    R=$(echo "$ERROR_CELL" | jq '.fg.r')
    if [ "$R" -gt 200 ]; then
        echo "PASS: Error text is red (RGB)"
    fi
fi

echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"
```

---

## Parallel Testing

### Scenario: Run Multiple Tests Concurrently

```bash
#!/bin/bash

run_test() {
    local name="$1"
    local cmd="$2"

    SOCK=$(termwright daemon --background -- $cmd)

    echo '{"id":1,"method":"wait_for_idle","params":{"duration_ms":1000,"timeout_ms":10000}}' | nc -U "$SOCK"

    SCREEN=$(echo '{"id":2,"method":"screen","params":{"format":"text"}}' | nc -U "$SOCK" | jq -r '.result')

    echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK"

    if [ -n "$SCREEN" ]; then
        echo "PASS: $name"
    else
        echo "FAIL: $name"
    fi
}

# Run tests in parallel
run_test "htop" "htop" &
run_test "vim" "vim --clean" &
run_test "less" "less /etc/hosts" &

wait
echo "All parallel tests complete"
```

---

## Error Handling Pattern

```bash
#!/bin/bash
set -e

cleanup() {
    if [ -n "$SOCK" ] && [ -S "$SOCK" ]; then
        echo '{"id":99,"method":"close","params":null}' | nc -U "$SOCK" 2>/dev/null || true
    fi
}
trap cleanup EXIT

SOCK=$(termwright daemon --background -- ./my-app)

# All commands with error checking
send_and_check() {
    local response=$(echo "$1" | nc -U "$SOCK")
    local error=$(echo "$response" | jq -r '.error // empty')
    if [ -n "$error" ]; then
        echo "Error: $error"
        exit 1
    fi
    echo "$response"
}

send_and_check '{"id":1,"method":"handshake","params":null}'
send_and_check '{"id":2,"method":"wait_for_text","params":{"text":"Ready","timeout_ms":5000}}'
# ... more commands

echo "Test passed!"
```
