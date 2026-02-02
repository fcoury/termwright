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

## First Step: Discover Features

```bash
termwright info steps              # List step types
termwright info steps waitForText  # Specific step syntax
termwright info keys               # Valid key names
termwright info protocols          # Daemon methods
```

## Discovery Quick Reference

| Need | Command |
|------|---------|
| Step syntax | `termwright info steps <step>` |
| Key names | `termwright info keys` |
| Protocol method | `termwright info protocols <method>` |
| Capabilities | `termwright info capabilities` |
| JSON output | Add `--json` to any info command |

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
  - notExpectText: {text: "ERROR"}
  - screenshot: {name: "vim-content"}
artifacts:
  mode: onFailure
  dir: ./termwright-artifacts
```

### Run the Test

```bash
termwright run-steps --trace test.yaml
```

## Negative Assertions

Check that content is NOT present:

```yaml
steps:
  - notExpectText: {text: "ERROR"}           # Immediate check
  - notExpectPattern: {pattern: "fail|crash"} # Regex check
  - waitForTextGone: {text: "Loading...", timeoutMs: 5000}  # Wait to disappear
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

## Debugging Tips

- Use `waitForIdle` before assertions to reduce flakiness
- Check `screen.json` for color/cursor mismatches
- Use `--trace` when diagnosing timing issues
- Run `termwright info steps` to discover available step types
