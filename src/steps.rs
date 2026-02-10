use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use termwright::error::{Result, TermwrightError};
use termwright::terminal::{DEFAULT_COLS, DEFAULT_ROWS};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepsFile {
    #[serde(default)]
    pub session: Option<SessionConfig>,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub artifacts: ArtifactsConfig,
}

impl StepsFile {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|e| TermwrightError::Protocol(format!("failed to read steps file: {e}")))?;

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "json" {
            serde_json::from_str(&contents).map_err(TermwrightError::Json)
        } else if ext == "yaml" || ext == "yml" {
            serde_yaml::from_str(&contents)
                .map_err(|e| TermwrightError::Protocol(format!("yaml error: {e}")))
        } else {
            serde_json::from_str(&contents)
                .or_else(|_| serde_yaml::from_str(&contents))
                .map_err(|e| TermwrightError::Protocol(format!("steps parse error: {e}")))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub command: CommandSpec,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cols: Option<u16>,
    #[serde(default)]
    pub rows: Option<u16>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub no_default_env: bool,
    #[serde(default)]
    pub no_osc_emulation: bool,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

impl SessionConfig {
    pub fn cols(&self) -> u16 {
        self.cols.unwrap_or(DEFAULT_COLS)
    }

    pub fn rows(&self) -> u16 {
        self.rows.unwrap_or(DEFAULT_ROWS)
    }

    pub fn command_and_args(&self) -> Result<(String, Vec<String>)> {
        match &self.command {
            CommandSpec::Array(values) => {
                let Some((command, rest)) = values.split_first() else {
                    return Err(TermwrightError::Protocol(
                        "command array must include at least one entry".to_string(),
                    ));
                };
                Ok((command.clone(), rest.to_vec()))
            }
            CommandSpec::Single(command) => Ok((command.clone(), self.args.clone())),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CommandSpec {
    Array(Vec<String>),
    Single(String),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactsConfig {
    #[serde(default)]
    pub mode: ArtifactMode,
    #[serde(default)]
    pub dir: Option<PathBuf>,
}

impl Default for ArtifactsConfig {
    fn default() -> Self {
        Self {
            mode: ArtifactMode::OnFailure,
            dir: None,
        }
    }
}

impl ArtifactsConfig {
    pub fn base_dir(&self) -> PathBuf {
        self.dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("termwright-artifacts"))
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactMode {
    OnFailure,
    Always,
    Off,
}

impl Default for ArtifactMode {
    fn default() -> Self {
        ArtifactMode::OnFailure
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Step {
    WaitForText {
        #[serde(rename = "waitForText")]
        wait_for_text: WaitForTextStep,
    },
    WaitForPattern {
        #[serde(rename = "waitForPattern")]
        wait_for_pattern: WaitForPatternStep,
    },
    WaitForIdle {
        #[serde(rename = "waitForIdle")]
        wait_for_idle: WaitForIdleStep,
    },
    WaitForTextGone {
        #[serde(rename = "waitForTextGone")]
        wait_for_text_gone: WaitForTextGoneStep,
    },
    WaitForPatternGone {
        #[serde(rename = "waitForPatternGone")]
        wait_for_pattern_gone: WaitForPatternGoneStep,
    },
    Press {
        press: PressStep,
    },
    Type {
        #[serde(rename = "type")]
        r#type: TypeStep,
    },
    Hotkey {
        hotkey: HotkeyStep,
    },
    ExpectText {
        #[serde(rename = "expectText")]
        expect_text: ExpectTextStep,
    },
    ExpectPattern {
        #[serde(rename = "expectPattern")]
        expect_pattern: ExpectPatternStep,
    },
    NotExpectText {
        #[serde(rename = "notExpectText")]
        not_expect_text: NotExpectTextStep,
    },
    NotExpectPattern {
        #[serde(rename = "notExpectPattern")]
        not_expect_pattern: NotExpectPatternStep,
    },
    Screenshot {
        screenshot: ScreenshotStep,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForTextStep {
    pub text: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForPatternStep {
    pub pattern: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForIdleStep {
    pub idle_ms: u64,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PressStep {
    pub key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeStep {
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStep {
    #[serde(default)]
    pub ctrl: Option<bool>,
    #[serde(default)]
    pub alt: Option<bool>,
    pub ch: char,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectTextStep {
    pub text: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectPatternStep {
    pub pattern: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForTextGoneStep {
    pub text: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForPatternGoneStep {
    pub pattern: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotExpectTextStep {
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotExpectPatternStep {
    pub pattern: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotStep {
    #[serde(default)]
    pub name: Option<String>,
}
