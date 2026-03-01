//! Protocol method documentation for info command.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MethodInfo {
    pub name: &'static str,
    pub category: &'static str,
    pub brief: &'static str,
    pub params: &'static str,
    pub response: &'static str,
    pub example_request: &'static str,
    pub example_response: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ProtocolsOverview {
    pub protocol_version: u32,
    pub methods: Vec<MethodInfo>,
}

impl ProtocolsOverview {
    pub fn new() -> Self {
        Self {
            protocol_version: 1,
            methods: all_methods(),
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Daemon Protocol (version {})\n\n",
            self.protocol_version
        ));
        out.push_str("Request format:  {\"id\": N, \"method\": \"...\", \"params\": {...}}\n");
        out.push_str("Response format: {\"id\": N, \"result\": ..., \"error\": null}\n\n");
        out.push_str("Methods:\n\n");

        let mut categories: Vec<&str> = self
            .methods
            .iter()
            .map(|m| m.category)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();

        for category in categories {
            out.push_str(&format!("{}:\n", category));
            for method in self.methods.iter().filter(|m| m.category == category) {
                out.push_str(&format!("  {:20} - {}\n", method.name, method.brief));
            }
            out.push('\n');
        }

        out.push_str("Use `termwright info protocols <method>` for detailed usage.\n");
        out
    }

    pub fn get(&self, name: &str) -> Option<&MethodInfo> {
        self.methods
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }
}

impl Default for ProtocolsOverview {
    fn default() -> Self {
        Self::new()
    }
}

impl MethodInfo {
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\n", self.name));
        out.push_str(&format!("{}\n\n", "-".repeat(self.name.len())));
        out.push_str(&format!("{}\n\n", self.brief));

        out.push_str(&format!("Params:   {}\n", self.params));
        out.push_str(&format!("Response: {}\n\n", self.response));

        out.push_str("Example request:\n");
        out.push_str(&format!("  {}\n\n", self.example_request));

        out.push_str("Example response:\n");
        out.push_str(&format!("  {}\n", self.example_response));

        out
    }
}

fn all_methods() -> Vec<MethodInfo> {
    vec![
        // Session
        MethodInfo {
            name: "handshake",
            category: "session",
            brief: "Initialize connection, get version info",
            params: "null",
            response: "{protocol_version, termwright_version, pid}",
            example_request: r#"{"id":1,"method":"handshake","params":null}"#,
            example_response: r#"{"id":1,"result":{"protocol_version":1,"termwright_version":"0.1.0","pid":12345}}"#,
        },
        MethodInfo {
            name: "status",
            category: "session",
            brief: "Check if process has exited",
            params: "null",
            response: "{exited: bool, exit_code: number|null}",
            example_request: r#"{"id":1,"method":"status","params":null}"#,
            example_response: r#"{"id":1,"result":{"exited":false,"exit_code":null}}"#,
        },
        MethodInfo {
            name: "close",
            category: "session",
            brief: "Terminate daemon and process",
            params: "null",
            response: "error with code 'closing'",
            example_request: r#"{"id":1,"method":"close","params":null}"#,
            example_response: r#"{"id":1,"result":null,"error":{"code":"closing","message":"closing"}}"#,
        },
        // Screen
        MethodInfo {
            name: "screen",
            category: "screen",
            brief: "Get screen content",
            params: r#"{format: "text"|"json"|"json_compact"}"#,
            response: "string (text) or object (json)",
            example_request: r#"{"id":1,"method":"screen","params":{"format":"text"}}"#,
            example_response: r#"{"id":1,"result":"Screen content here..."}"#,
        },
        MethodInfo {
            name: "screenshot",
            category: "screen",
            brief: "Capture PNG screenshot",
            params: r#"{font?: string, font_size?: number, line_height?: number}"#,
            response: "{png_base64: string}",
            example_request: r#"{"id":1,"method":"screenshot","params":{}}"#,
            example_response: r#"{"id":1,"result":{"png_base64":"iVBORw0KGgo..."}}"#,
        },
        // Input
        MethodInfo {
            name: "type",
            category: "input",
            brief: "Type a string of text",
            params: r#"{text: string}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"type","params":{"text":"Hello"}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "press",
            category: "input",
            brief: "Press a single key",
            params: r#"{key: string}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"press","params":{"key":"Enter"}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "hotkey",
            category: "input",
            brief: "Press key with modifiers",
            params: r#"{ch: char, ctrl?: bool, alt?: bool}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"hotkey","params":{"ctrl":true,"ch":"c"}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "raw",
            category: "input",
            brief: "Send raw bytes to terminal",
            params: r#"{bytes_base64: string}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"raw","params":{"bytes_base64":"G1s="}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "mouse_click",
            category: "input",
            brief: "Click mouse at position",
            params: r#"{row: number, col: number, button?: "left"|"right"|"middle"}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"mouse_click","params":{"row":5,"col":10}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "mouse_move",
            category: "input",
            brief: "Move mouse cursor",
            params: r#"{row: number, col: number, buttons?: string[]}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"mouse_move","params":{"row":5,"col":10}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "mouse_scroll",
            category: "input",
            brief: "Scroll mouse wheel at position",
            params: r#"{row: number, col: number, direction: "up"|"down", count?: number (default: 3)}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"mouse_scroll","params":{"row":5,"col":10,"direction":"down","count":3}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        // Wait
        MethodInfo {
            name: "wait_for_text",
            category: "wait",
            brief: "Wait for text to appear",
            params: r#"{text: string, timeout_ms?: number}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"wait_for_text","params":{"text":"Ready","timeout_ms":5000}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "wait_for_pattern",
            category: "wait",
            brief: "Wait for regex pattern to match",
            params: r#"{pattern: string, timeout_ms?: number}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"wait_for_pattern","params":{"pattern":"v\\d+","timeout_ms":5000}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "wait_for_idle",
            category: "wait",
            brief: "Wait for screen to stabilize",
            params: r#"{idle_ms: number, timeout_ms?: number}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"wait_for_idle","params":{"idle_ms":500,"timeout_ms":10000}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "wait_for_exit",
            category: "wait",
            brief: "Wait for process to exit",
            params: r#"{timeout_ms?: number}"#,
            response: "{exit_code: number}",
            example_request: r#"{"id":1,"method":"wait_for_exit","params":{"timeout_ms":5000}}"#,
            example_response: r#"{"id":1,"result":{"exit_code":0}}"#,
        },
        MethodInfo {
            name: "wait_for_text_gone",
            category: "wait",
            brief: "Wait for text to disappear",
            params: r#"{text: string, timeout_ms?: number}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"wait_for_text_gone","params":{"text":"Loading...","timeout_ms":5000}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "wait_for_pattern_gone",
            category: "wait",
            brief: "Wait for pattern to stop matching",
            params: r#"{pattern: string, timeout_ms?: number}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"wait_for_pattern_gone","params":{"pattern":"\\d+%","timeout_ms":10000}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        // Assert
        MethodInfo {
            name: "not_expect_text",
            category: "assert",
            brief: "Assert text is NOT present (immediate)",
            params: r#"{text: string}"#,
            response: "null (or error if text found)",
            example_request: r#"{"id":1,"method":"not_expect_text","params":{"text":"ERROR"}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        MethodInfo {
            name: "not_expect_pattern",
            category: "assert",
            brief: "Assert pattern does NOT match (immediate)",
            params: r#"{pattern: string}"#,
            response: "null (or error if pattern matches)",
            example_request: r#"{"id":1,"method":"not_expect_pattern","params":{"pattern":"error|fail"}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
        // Control
        MethodInfo {
            name: "resize",
            category: "control",
            brief: "Resize terminal",
            params: r#"{cols: number, rows: number}"#,
            response: "null",
            example_request: r#"{"id":1,"method":"resize","params":{"cols":120,"rows":40}}"#,
            example_response: r#"{"id":1,"result":null}"#,
        },
    ]
}
