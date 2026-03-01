//! Step type documentation for info command.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StepInfo {
    pub name: &'static str,
    pub category: &'static str,
    pub brief: &'static str,
    pub params: Vec<ParamInfo>,
    pub example: &'static str,
    pub tips: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ParamInfo {
    pub name: &'static str,
    pub required: bool,
    pub r#type: &'static str,
    pub default: Option<&'static str>,
    pub description: &'static str,
}

#[derive(Debug, Serialize)]
pub struct StepsOverview {
    pub steps: Vec<StepInfo>,
}

impl StepsOverview {
    pub fn new() -> Self {
        Self { steps: all_steps() }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("Available step types:\n\n");

        let mut categories: Vec<&str> = self
            .steps
            .iter()
            .map(|s| s.category)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();

        for category in categories {
            out.push_str(&format!("{}:\n", category));
            for step in self.steps.iter().filter(|s| s.category == category) {
                out.push_str(&format!("  {:20} - {}\n", step.name, step.brief));
            }
            out.push('\n');
        }

        out.push_str("Use `termwright info steps <step-name>` for detailed usage.\n");
        out
    }

    pub fn get(&self, name: &str) -> Option<&StepInfo> {
        self.steps
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }
}

impl Default for StepsOverview {
    fn default() -> Self {
        Self::new()
    }
}

impl StepInfo {
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\n", self.name));
        out.push_str(&format!("{}\n\n", "-".repeat(self.name.len())));
        out.push_str(&format!("{}\n\n", self.brief));

        out.push_str("Parameters:\n");
        for param in &self.params {
            let req = if param.required {
                "(required)"
            } else {
                "(optional)"
            };
            out.push_str(&format!("  {} {} [{}]\n", param.name, req, param.r#type));
            out.push_str(&format!("      {}\n", param.description));
            if let Some(default) = param.default {
                out.push_str(&format!("      Default: {}\n", default));
            }
        }

        out.push_str("\nExample:\n");
        out.push_str(&format!("  {}\n", self.example));

        if !self.tips.is_empty() {
            out.push_str("\nTips:\n");
            for tip in &self.tips {
                out.push_str(&format!("  - {}\n", tip));
            }
        }

        out
    }
}

fn all_steps() -> Vec<StepInfo> {
    vec![
        // Wait steps
        StepInfo {
            name: "waitForText",
            category: "wait",
            brief: "Wait for text to appear on screen",
            params: vec![
                ParamInfo {
                    name: "text",
                    required: true,
                    r#type: "string",
                    default: None,
                    description: "Text to wait for",
                },
                ParamInfo {
                    name: "timeoutMs",
                    required: false,
                    r#type: "number",
                    default: Some("30000"),
                    description: "Timeout in milliseconds",
                },
            ],
            example: r#"waitForText: {text: "Ready", timeoutMs: 5000}"#,
            tips: vec!["Use exact text that appears on screen"],
        },
        StepInfo {
            name: "waitForPattern",
            category: "wait",
            brief: "Wait for regex pattern to match on screen",
            params: vec![
                ParamInfo {
                    name: "pattern",
                    required: true,
                    r#type: "string",
                    default: None,
                    description: "Regex pattern to match",
                },
                ParamInfo {
                    name: "timeoutMs",
                    required: false,
                    r#type: "number",
                    default: Some("30000"),
                    description: "Timeout in milliseconds",
                },
            ],
            example: r#"waitForPattern: {pattern: "v\\d+\\.\\d+", timeoutMs: 5000}"#,
            tips: vec!["Escape backslashes in YAML strings"],
        },
        StepInfo {
            name: "waitForIdle",
            category: "wait",
            brief: "Wait for screen to stabilize (no changes)",
            params: vec![
                ParamInfo {
                    name: "idleMs",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "Duration of stability required (ms)",
                },
                ParamInfo {
                    name: "timeoutMs",
                    required: false,
                    r#type: "number",
                    default: Some("30000"),
                    description: "Overall timeout in milliseconds",
                },
            ],
            example: r#"waitForIdle: {idleMs: 500, timeoutMs: 10000}"#,
            tips: vec![
                "Useful before assertions to reduce flakiness",
                "Use after app startup to ensure it's fully loaded",
            ],
        },
        StepInfo {
            name: "waitForTextGone",
            category: "wait",
            brief: "Wait for text to disappear from screen",
            params: vec![
                ParamInfo {
                    name: "text",
                    required: true,
                    r#type: "string",
                    default: None,
                    description: "Text to wait to disappear",
                },
                ParamInfo {
                    name: "timeoutMs",
                    required: false,
                    r#type: "number",
                    default: Some("30000"),
                    description: "Timeout in milliseconds",
                },
            ],
            example: r#"waitForTextGone: {text: "Loading...", timeoutMs: 5000}"#,
            tips: vec!["Useful for waiting for loading indicators to finish"],
        },
        StepInfo {
            name: "waitForPatternGone",
            category: "wait",
            brief: "Wait for regex pattern to stop matching",
            params: vec![
                ParamInfo {
                    name: "pattern",
                    required: true,
                    r#type: "string",
                    default: None,
                    description: "Regex pattern to stop matching",
                },
                ParamInfo {
                    name: "timeoutMs",
                    required: false,
                    r#type: "number",
                    default: Some("30000"),
                    description: "Timeout in milliseconds",
                },
            ],
            example: r#"waitForPatternGone: {pattern: "progress:\\s*\\d+%", timeoutMs: 10000}"#,
            tips: vec!["Waits until the pattern no longer matches anywhere on screen"],
        },
        // Input steps
        StepInfo {
            name: "press",
            category: "input",
            brief: "Press a single key",
            params: vec![ParamInfo {
                name: "key",
                required: true,
                r#type: "string",
                default: None,
                description: "Key name (Enter, Tab, Up, Down, F1, etc.)",
            }],
            example: r#"press: {key: "Enter"}"#,
            tips: vec![
                "Use `termwright info keys` for valid key names",
                "Single characters work: {key: \"a\"}",
            ],
        },
        StepInfo {
            name: "type",
            category: "input",
            brief: "Type a string of text",
            params: vec![ParamInfo {
                name: "text",
                required: true,
                r#type: "string",
                default: None,
                description: "Text to type",
            }],
            example: r#"type: {text: "Hello, World!"}"#,
            tips: vec!["Does not press Enter at the end; use a separate press step"],
        },
        StepInfo {
            name: "hotkey",
            category: "input",
            brief: "Press a key with modifiers (Ctrl, Alt)",
            params: vec![
                ParamInfo {
                    name: "ch",
                    required: true,
                    r#type: "char",
                    default: None,
                    description: "Character to press",
                },
                ParamInfo {
                    name: "ctrl",
                    required: false,
                    r#type: "bool",
                    default: Some("false"),
                    description: "Hold Ctrl key",
                },
                ParamInfo {
                    name: "alt",
                    required: false,
                    r#type: "bool",
                    default: Some("false"),
                    description: "Hold Alt key",
                },
            ],
            example: r#"hotkey: {ctrl: true, ch: "c"}"#,
            tips: vec!["Use for Ctrl+C, Ctrl+S, Alt+F, etc."],
        },
        // Assert steps
        StepInfo {
            name: "expectText",
            category: "assert",
            brief: "Assert text is present (with optional wait)",
            params: vec![
                ParamInfo {
                    name: "text",
                    required: true,
                    r#type: "string",
                    default: None,
                    description: "Text that must be present",
                },
                ParamInfo {
                    name: "timeoutMs",
                    required: false,
                    r#type: "number",
                    default: Some("30000"),
                    description: "Timeout to wait for text",
                },
            ],
            example: r#"expectText: {text: "Success"}"#,
            tips: vec!["Use waitForIdle before for more reliable assertions"],
        },
        StepInfo {
            name: "expectPattern",
            category: "assert",
            brief: "Assert regex pattern matches (with optional wait)",
            params: vec![
                ParamInfo {
                    name: "pattern",
                    required: true,
                    r#type: "string",
                    default: None,
                    description: "Regex pattern that must match",
                },
                ParamInfo {
                    name: "timeoutMs",
                    required: false,
                    r#type: "number",
                    default: Some("30000"),
                    description: "Timeout to wait for pattern",
                },
            ],
            example: r#"expectPattern: {pattern: "Items:\\s*\\d+"}"#,
            tips: vec!["Escape backslashes in YAML"],
        },
        StepInfo {
            name: "notExpectText",
            category: "assert",
            brief: "Assert text is NOT present (immediate check)",
            params: vec![ParamInfo {
                name: "text",
                required: true,
                r#type: "string",
                default: None,
                description: "Text that must NOT be present",
            }],
            example: r#"notExpectText: {text: "ERROR"}"#,
            tips: vec![
                "Immediate check, no waiting",
                "Fails instantly if text is found",
            ],
        },
        StepInfo {
            name: "notExpectPattern",
            category: "assert",
            brief: "Assert regex pattern does NOT match (immediate check)",
            params: vec![ParamInfo {
                name: "pattern",
                required: true,
                r#type: "string",
                default: None,
                description: "Regex pattern that must NOT match",
            }],
            example: r#"notExpectPattern: {pattern: "error|fail|crash"}"#,
            tips: vec![
                "Immediate check, no waiting",
                "Useful for checking no errors after an action",
            ],
        },
        // Capture steps
        StepInfo {
            name: "screenshot",
            category: "capture",
            brief: "Capture a PNG screenshot",
            params: vec![ParamInfo {
                name: "name",
                required: false,
                r#type: "string",
                default: Some("step-NNN-screenshot"),
                description: "Output filename (without .png extension)",
            }],
            example: r#"screenshot: {name: "final-result"}"#,
            tips: vec![
                "Requires artifacts mode 'always' or 'onFailure'",
                "Saved to the artifacts directory",
            ],
        },
        // Mouse steps
        StepInfo {
            name: "mouseClick",
            category: "input",
            brief: "Click mouse at a screen position",
            params: vec![
                ParamInfo {
                    name: "row",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "Row position (0-indexed)",
                },
                ParamInfo {
                    name: "col",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "Column position (0-indexed)",
                },
                ParamInfo {
                    name: "button",
                    required: false,
                    r#type: "string",
                    default: Some("left"),
                    description: "Mouse button: left, right, or middle",
                },
            ],
            example: r#"mouseClick: {row: 5, col: 10}"#,
            tips: vec!["Uses SGR 1006 mouse encoding"],
        },
        StepInfo {
            name: "mouseScroll",
            category: "input",
            brief: "Scroll mouse wheel at a position",
            params: vec![
                ParamInfo {
                    name: "row",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "Row position (0-indexed)",
                },
                ParamInfo {
                    name: "col",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "Column position (0-indexed)",
                },
                ParamInfo {
                    name: "direction",
                    required: false,
                    r#type: "string",
                    default: Some("down"),
                    description: "Scroll direction: up or down",
                },
                ParamInfo {
                    name: "count",
                    required: false,
                    r#type: "number",
                    default: Some("3"),
                    description: "Number of scroll events",
                },
            ],
            example: r#"mouseScroll: {row: 5, col: 10, direction: "down", count: 5}"#,
            tips: vec!["Default count is 3 scroll events"],
        },
        StepInfo {
            name: "mouseMove",
            category: "input",
            brief: "Move mouse cursor to a position",
            params: vec![
                ParamInfo {
                    name: "row",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "Row position (0-indexed)",
                },
                ParamInfo {
                    name: "col",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "Column position (0-indexed)",
                },
            ],
            example: r#"mouseMove: {row: 5, col: 10}"#,
            tips: vec!["Useful for hover effects in TUI apps"],
        },
        // Wait steps (additional)
        StepInfo {
            name: "waitForExit",
            category: "wait",
            brief: "Wait for the process to exit",
            params: vec![ParamInfo {
                name: "timeoutMs",
                required: false,
                r#type: "number",
                default: Some("30000"),
                description: "Timeout in milliseconds",
            }],
            example: r#"waitForExit: {timeoutMs: 10000}"#,
            tips: vec![
                "Returns the exit code of the process",
                "Useful as the final step in a test",
            ],
        },
        // Control steps
        StepInfo {
            name: "resize",
            category: "control",
            brief: "Resize the terminal",
            params: vec![
                ParamInfo {
                    name: "cols",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "New column count",
                },
                ParamInfo {
                    name: "rows",
                    required: true,
                    r#type: "number",
                    default: None,
                    description: "New row count",
                },
            ],
            example: r#"resize: {cols: 120, rows: 40}"#,
            tips: vec!["Useful for testing responsive TUI layouts"],
        },
        StepInfo {
            name: "sleep",
            category: "control",
            brief: "Pause execution for a duration",
            params: vec![ParamInfo {
                name: "ms",
                required: true,
                r#type: "number",
                default: None,
                description: "Duration in milliseconds",
            }],
            example: r#"sleep: {ms: 1000}"#,
            tips: vec![
                "Prefer waitForText or waitForIdle over sleep",
                "Use as a last resort when no wait condition applies",
            ],
        },
        StepInfo {
            name: "raw",
            category: "input",
            brief: "Send raw bytes to the terminal",
            params: vec![ParamInfo {
                name: "bytesBase64",
                required: true,
                r#type: "string",
                default: None,
                description: "Base64-encoded bytes to send",
            }],
            example: r#"raw: {bytesBase64: "G1s="}"#,
            tips: vec![
                "Use for escape sequences not covered by other steps",
                "G1s= is base64 for ESC[",
            ],
        },
    ]
}
