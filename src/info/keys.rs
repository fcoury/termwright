//! Key names documentation for info command.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct KeyInfo {
    pub name: &'static str,
    pub aliases: Vec<&'static str>,
    pub description: &'static str,
}

#[derive(Debug, Serialize)]
pub struct KeyCategory {
    pub name: &'static str,
    pub keys: Vec<KeyInfo>,
}

#[derive(Debug, Serialize)]
pub struct KeysOverview {
    pub categories: Vec<KeyCategory>,
}

impl KeysOverview {
    pub fn new() -> Self {
        Self {
            categories: all_keys(),
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("Valid key names for press/hotkey steps:\n\n");

        for category in &self.categories {
            out.push_str(&format!("{}:\n", category.name));
            for key in &category.keys {
                if key.aliases.is_empty() {
                    out.push_str(&format!("  {:12} - {}\n", key.name, key.description));
                } else {
                    out.push_str(&format!(
                        "  {:12} - {} (aliases: {})\n",
                        key.name,
                        key.description,
                        key.aliases.join(", ")
                    ));
                }
            }
            out.push('\n');
        }

        out.push_str("Examples:\n");
        out.push_str("  press: {key: \"Enter\"}\n");
        out.push_str("  press: {key: \"F1\"}\n");
        out.push_str("  press: {key: \"a\"}\n");
        out.push_str("  hotkey: {ctrl: true, ch: \"c\"}\n");
        out
    }
}

impl Default for KeysOverview {
    fn default() -> Self {
        Self::new()
    }
}

fn all_keys() -> Vec<KeyCategory> {
    vec![
        KeyCategory {
            name: "Navigation",
            keys: vec![
                KeyInfo {
                    name: "Up",
                    aliases: vec![],
                    description: "Arrow up",
                },
                KeyInfo {
                    name: "Down",
                    aliases: vec![],
                    description: "Arrow down",
                },
                KeyInfo {
                    name: "Left",
                    aliases: vec![],
                    description: "Arrow left",
                },
                KeyInfo {
                    name: "Right",
                    aliases: vec![],
                    description: "Arrow right",
                },
                KeyInfo {
                    name: "Home",
                    aliases: vec![],
                    description: "Home key",
                },
                KeyInfo {
                    name: "End",
                    aliases: vec![],
                    description: "End key",
                },
                KeyInfo {
                    name: "PageUp",
                    aliases: vec!["page_up"],
                    description: "Page up",
                },
                KeyInfo {
                    name: "PageDown",
                    aliases: vec!["page_down"],
                    description: "Page down",
                },
            ],
        },
        KeyCategory {
            name: "Special",
            keys: vec![
                KeyInfo {
                    name: "Enter",
                    aliases: vec![],
                    description: "Enter/Return key",
                },
                KeyInfo {
                    name: "Tab",
                    aliases: vec![],
                    description: "Tab key",
                },
                KeyInfo {
                    name: "Escape",
                    aliases: vec!["esc"],
                    description: "Escape key",
                },
                KeyInfo {
                    name: "Backspace",
                    aliases: vec![],
                    description: "Backspace key",
                },
                KeyInfo {
                    name: "Delete",
                    aliases: vec!["del"],
                    description: "Delete key",
                },
            ],
        },
        KeyCategory {
            name: "Function",
            keys: vec![
                KeyInfo {
                    name: "F1-F12",
                    aliases: vec![],
                    description: "Function keys (e.g., F1, F2, ... F12)",
                },
            ],
        },
        KeyCategory {
            name: "Characters",
            keys: vec![
                KeyInfo {
                    name: "<char>",
                    aliases: vec![],
                    description: "Any single character (a, A, 1, @, etc.)",
                },
            ],
        },
    ]
}
