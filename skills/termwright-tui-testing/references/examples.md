# Termwright Examples

Real-world patterns for TUI testing and automation.

## Step File Examples

### Testing a Vim Workflow

```yaml
# test-vim-workflow.yaml
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
  - screenshot: {name: "01-vim-loaded"}

  # Navigate to specific line
  - type: {text: ":10"}
  - press: {key: Enter}
  - waitForIdle: {idleMs: 200}

  # Enter insert mode and add comment
  - press: {key: i}
  - waitForText: {text: "-- INSERT --"}
  - type: {text: "# TODO: Implement this function"}
  - press: {key: Escape}

  # Save file
  - type: {text: ":w"}
  - press: {key: Enter}
  - expectText: {text: "written", timeoutMs: 3000}
  - screenshot: {name: "02-after-save"}

  # Quit
  - type: {text: ":q"}
  - press: {key: Enter}

artifacts:
  mode: always
  dir: ./vim-test-output
```

### Testing an Interactive Menu

```yaml
# test-menu-navigation.yaml
session:
  command: ./my-menu-app
  cols: 80
  rows: 24

steps:
  # Wait for menu to appear
  - waitForText: {text: "Main Menu", timeoutMs: 5000}
  - waitForIdle: {idleMs: 500}

  # Navigate down to "Settings"
  - press: {key: Down}
  - press: {key: Down}
  - press: {key: Down}
  - expectText: {text: "> Settings"}
  - screenshot: {name: "menu-at-settings"}

  # Enter settings
  - press: {key: Enter}
  - waitForText: {text: "Settings", timeoutMs: 3000}

  # Toggle an option
  - press: {key: Down}
  - press: {key: " "}  # Space to toggle
  - expectText: {text: "[x]"}

  # Go back
  - press: {key: Escape}
  - waitForText: {text: "Main Menu"}

artifacts:
  mode: onFailure
  dir: ./menu-test
```

### Testing a Ratatui App

```yaml
# test-ratatui-app.yaml
session:
  command: cargo
  args: [run, --release]
  cols: 100
  rows: 30
  cwd: /path/to/ratatui-project

steps:
  # Wait for app startup
  - waitForIdle: {idleMs: 2000, timeoutMs: 15000}
  - screenshot: {name: "01-startup"}

  # Test tab navigation
  - press: {key: Tab}
  - waitForIdle: {idleMs: 200}
  - screenshot: {name: "02-after-tab"}

  # Test input field
  - type: {text: "search query"}
  - waitForIdle: {idleMs: 200}
  - expectText: {text: "search query"}

  # Test hotkey
  - hotkey: {ctrl: true, ch: "s"}
  - waitForText: {text: "Saved", timeoutMs: 3000}

  # Quit
  - hotkey: {ctrl: true, ch: "q"}

artifacts:
  mode: always
  dir: ./ratatui-test
```

### Testing htop

```yaml
# test-htop.yaml
session:
  command: htop
  cols: 120
  rows: 40

steps:
  # Wait for htop to load
  - waitForIdle: {idleMs: 1000, timeoutMs: 10000}
  - expectText: {text: "CPU", timeoutMs: 5000}
  - screenshot: {name: "htop-loaded"}

  # Open help
  - press: {key: "?"}
  - waitForText: {text: "Help", timeoutMs: 2000}
  - screenshot: {name: "htop-help"}

  # Close help
  - press: {key: Escape}
  - waitForIdle: {idleMs: 500}

  # Open tree view
  - press: {key: "t"}
  - waitForIdle: {idleMs: 500}
  - screenshot: {name: "htop-tree"}

  # Quit
  - press: {key: "q"}

artifacts:
  mode: always
  dir: ./htop-test
```

## Shell Script Examples

### Basic Test Script

```bash
#!/bin/bash
set -euo pipefail

# Start daemon in background
SOCK=$(termwright daemon --background -- ./my-app)
echo "Started daemon with socket: $SOCK"

# Cleanup on exit
trap "termwright exec --socket '$SOCK' --method close 2>/dev/null || true" EXIT

# Wait for socket to be available
while ! [ -S "$SOCK" ]; do sleep 0.1; done

# Helper function
tw() {
    termwright exec --socket "$SOCK" --method "$1" --params "${2:-null}"
}

# Run tests
echo "Testing..."

# 1. Handshake
tw handshake
echo "Handshake OK"

# 2. Wait for app ready
tw wait_for_text '{"text":"Ready","timeout_ms":10000}'
echo "App ready"

# 3. Interact
tw press '{"key":"Enter"}'
tw type '{"text":"hello"}'
tw press '{"key":"Enter"}'

# 4. Verify
SCREEN=$(tw screen '{"format":"text"}' | jq -r '.result')
if echo "$SCREEN" | grep -q "hello"; then
    echo "PASS: Text found"
else
    echo "FAIL: Expected text not found"
    echo "Screen content:"
    echo "$SCREEN"
    exit 1
fi

# 5. Screenshot
tw screenshot | jq -r '.result.png_base64' | base64 -d > result.png
echo "Screenshot saved to result.png"

echo "All tests passed!"
```

### Parallel Testing Script

```bash
#!/bin/bash
set -euo pipefail

# Start hub with multiple sessions
echo "Starting 3 parallel sessions..."
termwright hub start --count 3 --output sessions.json -- ./my-app

# Cleanup on exit
trap "termwright hub stop --input sessions.json" EXIT

# Extract socket paths
SOCKETS=($(jq -r '.[].socket' sessions.json))

# Run tests in parallel
run_test() {
    local sock=$1
    local test_num=$2

    echo "[$test_num] Running test on $sock"

    termwright exec --socket "$sock" --method handshake > /dev/null
    termwright exec --socket "$sock" --method wait_for_text \
        --params '{"text":"Ready","timeout_ms":10000}' > /dev/null

    termwright exec --socket "$sock" --method type \
        --params "{\"text\":\"test-$test_num\"}" > /dev/null

    termwright exec --socket "$sock" --method press \
        --params '{"key":"Enter"}' > /dev/null

    local screen=$(termwright exec --socket "$sock" --method screen \
        --params '{"format":"text"}' | jq -r '.result')

    if echo "$screen" | grep -q "test-$test_num"; then
        echo "[$test_num] PASS"
        return 0
    else
        echo "[$test_num] FAIL"
        return 1
    fi
}

# Run all tests in parallel
pids=()
for i in "${!SOCKETS[@]}"; do
    run_test "${SOCKETS[$i]}" "$i" &
    pids+=($!)
done

# Wait for all tests
failed=0
for pid in "${pids[@]}"; do
    if ! wait $pid; then
        failed=1
    fi
done

if [ $failed -eq 0 ]; then
    echo "All parallel tests passed!"
else
    echo "Some tests failed"
    exit 1
fi
```

### Screenshot Comparison Script

```bash
#!/bin/bash
set -euo pipefail

BASELINE_DIR="./baselines"
OUTPUT_DIR="./current"
mkdir -p "$OUTPUT_DIR"

# Take current screenshots
SOCK=$(termwright daemon --background -- ./my-app)
trap "termwright exec --socket '$SOCK' --method close 2>/dev/null || true" EXIT

while ! [ -S "$SOCK" ]; do sleep 0.1; done

tw() {
    termwright exec --socket "$SOCK" --method "$1" --params "${2:-null}"
}

# Capture screens
tw handshake > /dev/null
tw wait_for_idle '{"idle_ms":1000,"timeout_ms":10000}' > /dev/null

# Screenshot 1: Main view
tw screenshot | jq -r '.result.png_base64' | base64 -d > "$OUTPUT_DIR/main.png"

# Screenshot 2: After pressing tab
tw press '{"key":"Tab"}' > /dev/null
tw wait_for_idle '{"idle_ms":500}' > /dev/null
tw screenshot | jq -r '.result.png_base64' | base64 -d > "$OUTPUT_DIR/tab.png"

# Compare with baselines (requires ImageMagick)
compare_images() {
    local name=$1
    local baseline="$BASELINE_DIR/$name.png"
    local current="$OUTPUT_DIR/$name.png"

    if [ ! -f "$baseline" ]; then
        echo "No baseline for $name, creating..."
        cp "$current" "$baseline"
        return 0
    fi

    # Compare with ImageMagick
    diff=$(compare -metric AE "$baseline" "$current" /dev/null 2>&1 || true)

    if [ "$diff" -eq 0 ]; then
        echo "PASS: $name matches baseline"
        return 0
    else
        echo "FAIL: $name differs by $diff pixels"
        # Generate diff image
        compare "$baseline" "$current" "$OUTPUT_DIR/${name}-diff.png" || true
        return 1
    fi
}

failed=0
for img in main tab; do
    if ! compare_images "$img"; then
        failed=1
    fi
done

exit $failed
```

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/tui-tests.yml
name: TUI Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Termwright
        run: cargo install termwright

      - name: Build app
        run: cargo build --release

      - name: Run TUI tests
        run: termwright run-steps --trace tests/e2e.yaml

      - name: Upload artifacts on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: tui-test-artifacts
          path: termwright-artifacts/
```

### Test Matrix

```yaml
# Run same test with different terminal sizes
jobs:
  test:
    strategy:
      matrix:
        size:
          - { cols: 80, rows: 24 }
          - { cols: 120, rows: 40 }
          - { cols: 160, rows: 50 }

    steps:
      - name: Run tests at ${{ matrix.size.cols }}x${{ matrix.size.rows }}
        run: |
          cat > test.yaml << EOF
          session:
            command: ./my-app
            cols: ${{ matrix.size.cols }}
            rows: ${{ matrix.size.rows }}
          steps:
            - waitForText: {text: "Ready", timeoutMs: 10000}
            - screenshot: {name: "size-${{ matrix.size.cols }}x${{ matrix.size.rows }}"}
          artifacts:
            mode: always
          EOF
          termwright run-steps --trace test.yaml
```

## Debugging Patterns

### Verbose Test with Artifacts

```yaml
session:
  command: ./problematic-app
  cols: 120
  rows: 40

steps:
  - waitForIdle: {idleMs: 2000, timeoutMs: 30000}
  - screenshot: {name: "debug-01-after-idle"}

  - press: {key: Enter}
  - waitForIdle: {idleMs: 500}
  - screenshot: {name: "debug-02-after-enter"}

  # Add more screenshots at each step to trace issue

artifacts:
  mode: always  # Capture everything during debugging
  dir: ./debug-output
```

### Inspect Screen State

```bash
# Get detailed screen info for debugging
SOCK=$(termwright daemon --background -- ./my-app)
sleep 2

# Full JSON with colors and attributes
termwright exec --socket "$SOCK" --method screen \
    --params '{"format":"json"}' | jq '.result' > screen.json

# Just text
termwright exec --socket "$SOCK" --method screen \
    --params '{"format":"text"}' | jq -r '.result'

# Compact JSON with cursor position
termwright exec --socket "$SOCK" --method screen \
    --params '{"format":"json_compact"}' | jq '.result'

termwright exec --socket "$SOCK" --method close
```
