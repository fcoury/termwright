---
name: termwright-tui-testing-focused
description: Focused workflow for TUI E2E testing with Termwright. Use for fast agent guidance when running step files, capturing artifacts, and debugging with trace output.
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
  - Glob
  - Grep
---

# Termwright TUI Testing (Focused)

Quick, agent-friendly workflow for testing terminal UIs with minimal ceremony.

## When to Use

- You need to test or debug terminal UIs (ratatui, crossterm, ncurses, etc.)
- You want repeatable E2E flows with artifacts for failures
- You need parallel sessions across multiple daemons

## Decision Guide

- **Use `run-steps`** for most E2E tests (preferred).
- **Use `exec`** for one-off commands or manual inspection.
- **Use `hub`** when multiple daemons are needed in parallel.

## Core Workflow (Preferred)

1) Write a step file (YAML or JSON)
2) Run with `run-steps`
3) Inspect artifacts and trace on failure

### Example Step File

```yaml
session:
  command: ["vim", "test.txt"]
  cols: 120
  rows: 40
steps:
  - waitForText: {text: "VIM", timeoutMs: 5000}
  - press: {key: i}
  - type: {text: "Hello"}
  - press: {key: Escape}
  - expectText: {text: "Hello"}
  - screenshot: {name: "vim-content"}
artifacts:
  mode: onFailure
  dir: ./termwright-artifacts
```

### Run the Test

```bash
termwright run-steps --trace test.yaml
```

## Artifact/Trace Behavior

- `onFailure`: save `failure-###-screen.txt/json` only when a step fails
- `always`: save `step-###-screen.txt/json` after every step
- `off`: no artifacts (screenshots require non-off mode)
- `--trace`: adds `trace.json` with step timings and hashes

## One-off Commands (Exec)

```bash
SOCK=$(termwright daemon --background -- vim test.txt)
termwright exec --socket "$SOCK" --method screen --params '{"format":"text"}'
termwright exec --socket "$SOCK" --method close
```

## Parallel Sessions (Hub)

```bash
termwright hub start --count 3 --output sessions.json -- ./my-app
termwright hub stop --input sessions.json
```

## Step Types (Quick Reference)

- `waitForText`, `waitForPattern`, `waitForIdle`
- `press`, `type`, `hotkey`
- `expectText`, `expectPattern`
- `screenshot`

## Debugging Tips

- Use `waitForIdle` before assertions to reduce flakiness
- Check `screen.json` for color/cursor mismatches
- Use `--trace` when diagnosing timing issues

## Protocol Note

Daemon methods use JSON-over-unix-socket. Prefer `termwright exec` unless you need a custom client.
