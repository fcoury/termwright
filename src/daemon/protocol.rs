use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    #[serde(default)]
    pub result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Response {
    pub fn ok<T: Serialize>(id: u64, value: T) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id,
            result: serde_json::to_value(value)?,
            error: None,
        })
    }

    pub fn ok_empty(id: u64) -> Self {
        Self {
            id,
            result: serde_json::Value::Null,
            error: None,
        }
    }

    pub fn err(id: u64, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id,
            result: serde_json::Value::Null,
            error: Some(ResponseError {
                code: code.into(),
                message: message.into(),
                data: None,
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeResult {
    pub protocol_version: u32,
    pub termwright_version: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenFormat {
    Text,
    Json,
    JsonCompact,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenParams {
    #[serde(default = "ScreenParams::default_format")]
    pub format: ScreenFormat,
}

impl ScreenParams {
    fn default_format() -> ScreenFormat {
        ScreenFormat::Text
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotParams {
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub png_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeParams {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PressParams {
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HotkeyParams {
    pub ctrl: Option<bool>,
    pub alt: Option<bool>,
    pub ch: char,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawParams {
    pub bytes_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MouseMoveParams {
    pub row: u16,
    pub col: u16,
    pub buttons: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MouseClickParams {
    pub row: u16,
    pub col: u16,
    pub button: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MouseScrollParams {
    pub row: u16,
    pub col: u16,
    pub direction: String,
    pub count: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForTextParams {
    pub text: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForPatternParams {
    pub pattern: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForIdleParams {
    pub idle_ms: u64,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForExitParams {
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForTextGoneParams {
    pub text: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForPatternGoneParams {
    pub pattern: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotExpectTextParams {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotExpectPatternParams {
    pub pattern: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitForExitResult {
    pub exit_code: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeParams {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResult {
    pub exited: bool,
    pub exit_code: Option<i32>,
}
