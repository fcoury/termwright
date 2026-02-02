//! Info command module for CLI introspection.

pub mod capabilities;
pub mod keys;
pub mod protocols;
pub mod steps;

use serde::Serialize;

/// Overview of available info topics.
#[derive(Debug, Serialize)]
pub struct InfoOverview {
    pub topics: Vec<TopicInfo>,
}

#[derive(Debug, Serialize)]
pub struct TopicInfo {
    pub name: &'static str,
    pub description: &'static str,
}

impl InfoOverview {
    pub fn new() -> Self {
        Self {
            topics: vec![
                TopicInfo {
                    name: "steps",
                    description: "List all step types for YAML/JSON step files",
                },
                TopicInfo {
                    name: "protocols",
                    description: "Daemon protocol methods for JSON-over-socket communication",
                },
                TopicInfo {
                    name: "keys",
                    description: "Valid key names for press/hotkey steps",
                },
                TopicInfo {
                    name: "capabilities",
                    description: "Runtime capabilities and version info",
                },
            ],
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("Available info topics:\n\n");
        for topic in &self.topics {
            out.push_str(&format!("  {:12} - {}\n", topic.name, topic.description));
        }
        out.push_str("\nUsage: termwright info <topic>\n");
        out.push_str("       termwright info <topic> <name>  (for detailed help)\n");
        out.push_str("       termwright info --json          (machine-readable output)\n");
        out
    }
}

impl Default for InfoOverview {
    fn default() -> Self {
        Self::new()
    }
}
