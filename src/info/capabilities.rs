//! Runtime capabilities documentation for info command.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CapabilitiesInfo {
    pub termwright_version: &'static str,
    pub protocol_version: u32,
    pub features: Vec<FeatureInfo>,
}

#[derive(Debug, Serialize)]
pub struct FeatureInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub available: bool,
}

impl CapabilitiesInfo {
    pub fn new() -> Self {
        Self {
            termwright_version: env!("CARGO_PKG_VERSION"),
            protocol_version: 1,
            features: vec![
                FeatureInfo {
                    name: "screenshots",
                    description: "PNG screenshot capture",
                    available: true,
                },
                FeatureInfo {
                    name: "mouse",
                    description: "Mouse input (SGR mode)",
                    available: true,
                },
                FeatureInfo {
                    name: "resize",
                    description: "Terminal resize",
                    available: true,
                },
                FeatureInfo {
                    name: "colors",
                    description: "256-color and true color support",
                    available: true,
                },
                FeatureInfo {
                    name: "negative_assertions",
                    description: "notExpectText/notExpectPattern steps",
                    available: true,
                },
                FeatureInfo {
                    name: "pattern_gone",
                    description: "waitForTextGone/waitForPatternGone steps",
                    available: true,
                },
            ],
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Termwright v{}\n", self.termwright_version));
        out.push_str(&format!("Protocol version: {}\n\n", self.protocol_version));

        out.push_str("Features:\n");
        for feature in &self.features {
            let status = if feature.available { "+" } else { "-" };
            out.push_str(&format!(
                "  [{}] {:20} - {}\n",
                status, feature.name, feature.description
            ));
        }

        out
    }
}

impl Default for CapabilitiesInfo {
    fn default() -> Self {
        Self::new()
    }
}
