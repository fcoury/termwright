use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::Engine;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use crate::daemon::protocol::*;
use crate::error::{Result, TermwrightError};
use crate::input::{MouseButton, ScrollDirection};
use crate::screen::{Screen, TextMatch};

pub struct DaemonClient {
    next_id: AtomicU64,
    state: Mutex<ClientState>,
}

struct ClientState {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl DaemonClient {
    pub async fn connect_unix(path: impl AsRef<Path>) -> Result<Self> {
        let stream = UnixStream::connect(path)
            .await
            .map_err(|e| TermwrightError::Ipc(format!("connect failed: {e}")))?;

        let (read_half, write_half) = stream.into_split();

        Ok(Self {
            next_id: AtomicU64::new(1),
            state: Mutex::new(ClientState {
                reader: BufReader::new(read_half),
                writer: write_half,
            }),
        })
    }

    pub async fn handshake(&self) -> Result<HandshakeResult> {
        self.call("handshake", serde_json::Value::Null).await
    }

    pub async fn screen_text(&self) -> Result<String> {
        self.call(
            "screen",
            ScreenParams {
                format: ScreenFormat::Text,
            },
        )
        .await
    }

    pub async fn screen_json(&self) -> Result<String> {
        let screen: Screen = self
            .call(
                "screen",
                ScreenParams {
                    format: ScreenFormat::Json,
                },
            )
            .await?;

        screen.to_json().map_err(TermwrightError::Json)
    }

    pub async fn screenshot_png(&self) -> Result<Vec<u8>> {
        let res: ScreenshotResult = self
            .call(
                "screenshot",
                ScreenshotParams {
                    font: None,
                    font_size: None,
                    line_height: None,
                },
            )
            .await?;

        base64::engine::general_purpose::STANDARD
            .decode(res.png_base64)
            .map_err(|e| TermwrightError::Protocol(e.to_string()))
    }

    pub async fn find_text(&self, text: impl Into<String>) -> Result<Vec<TextMatch>> {
        self.call("find_text", FindTextParams { text: text.into() })
            .await
    }

    pub async fn find_pattern(&self, pattern: impl Into<String>) -> Result<Vec<TextMatch>> {
        self.call("find_pattern", FindPatternParams { pattern: pattern.into() })
            .await
    }

    pub async fn r#type(&self, text: impl Into<String>) -> Result<()> {
        self.call::<_, serde_json::Value>("type", TypeParams { text: text.into() })
            .await?;
        Ok(())
    }

    pub async fn press(&self, key: impl Into<String>) -> Result<()> {
        self.call::<_, serde_json::Value>("press", PressParams { key: key.into() })
            .await?;
        Ok(())
    }

    pub async fn hotkey_ctrl(&self, ch: char) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "hotkey",
            HotkeyParams {
                ctrl: Some(true),
                alt: Some(false),
                ch,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn hotkey(&self, ctrl: bool, alt: bool, ch: char) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "hotkey",
            HotkeyParams {
                ctrl: Some(ctrl),
                alt: Some(alt),
                ch,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn mouse_click(&self, row: u16, col: u16, button: MouseButton) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "mouse_click",
            MouseClickParams {
                row,
                col,
                button: Some(button.to_string()),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn mouse_scroll(
        &self,
        row: u16,
        col: u16,
        direction: ScrollDirection,
        count: Option<u16>,
    ) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "mouse_scroll",
            MouseScrollParams {
                row,
                col,
                direction: direction.to_string(),
                count,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn mouse_move(&self, row: u16, col: u16) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "mouse_move",
            MouseMoveParams {
                row,
                col,
                buttons: None,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn wait_for_text(
        &self,
        text: impl Into<String>,
        timeout: Option<Duration>,
    ) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "wait_for_text",
            WaitForTextParams {
                text: text.into(),
                timeout_ms: timeout.map(|d| d.as_millis() as u64),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn wait_for_pattern(
        &self,
        pattern: impl Into<String>,
        timeout: Option<Duration>,
    ) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "wait_for_pattern",
            WaitForPatternParams {
                pattern: pattern.into(),
                timeout_ms: timeout.map(|d| d.as_millis() as u64),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn wait_for_idle(&self, idle: Duration, timeout: Option<Duration>) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "wait_for_idle",
            WaitForIdleParams {
                idle_ms: idle.as_millis() as u64,
                timeout_ms: timeout.map(|d| d.as_millis() as u64),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn wait_for_text_gone(
        &self,
        text: impl Into<String>,
        timeout: Option<Duration>,
    ) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "wait_for_text_gone",
            WaitForTextGoneParams {
                text: text.into(),
                timeout_ms: timeout.map(|d| d.as_millis() as u64),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn wait_for_pattern_gone(
        &self,
        pattern: impl Into<String>,
        timeout: Option<Duration>,
    ) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "wait_for_pattern_gone",
            WaitForPatternGoneParams {
                pattern: pattern.into(),
                timeout_ms: timeout.map(|d| d.as_millis() as u64),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn not_expect_text(&self, text: impl Into<String>) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "not_expect_text",
            NotExpectTextParams { text: text.into() },
        )
        .await?;
        Ok(())
    }

    pub async fn not_expect_pattern(&self, pattern: impl Into<String>) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "not_expect_pattern",
            NotExpectPatternParams {
                pattern: pattern.into(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn detect_boxes(&self) -> Result<Vec<crate::screen::DetectedBox>> {
        self.call("detect_boxes", serde_json::Value::Null).await
    }

    pub async fn wait_for_cursor_at(
        &self,
        row: u16,
        col: u16,
        timeout: Option<Duration>,
    ) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "wait_for_cursor_at",
            WaitForCursorAtParams {
                row,
                col,
                timeout_ms: timeout.map(|d| d.as_millis() as u64),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn wait_for_exit(&self, timeout: Option<Duration>) -> Result<i32> {
        let res: WaitForExitResult = self
            .call(
                "wait_for_exit",
                WaitForExitParams {
                    timeout_ms: timeout.map(|d| d.as_millis() as u64),
                },
            )
            .await?;
        Ok(res.exit_code)
    }

    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.call::<_, serde_json::Value>("resize", ResizeParams { cols, rows })
            .await?;
        Ok(())
    }

    pub async fn raw(&self, bytes_base64: impl Into<String>) -> Result<()> {
        self.call::<_, serde_json::Value>(
            "raw",
            RawParams {
                bytes_base64: bytes_base64.into(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        let _ = self
            .call::<_, serde_json::Value>("close", serde_json::Value::Null)
            .await;
        Ok(())
    }

    pub async fn call_raw(
        &self,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.call(&method.into(), params).await
    }

    async fn call<P: Serialize, R: DeserializeOwned>(&self, method: &str, params: P) -> Result<R> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = Request {
            id,
            method: method.to_string(),
            params: serde_json::to_value(params).map_err(TermwrightError::Json)?,
        };

        let mut state = self.state.lock().await;

        let mut bytes = serde_json::to_vec(&request).map_err(TermwrightError::Json)?;
        bytes.push(b'\n');
        state
            .writer
            .write_all(&bytes)
            .await
            .map_err(|e| TermwrightError::Ipc(format!("write failed: {e}")))?;
        state
            .writer
            .flush()
            .await
            .map_err(|e| TermwrightError::Ipc(format!("flush failed: {e}")))?;

        let mut line = String::new();
        state
            .reader
            .read_line(&mut line)
            .await
            .map_err(|e| TermwrightError::Ipc(format!("read failed: {e}")))?;

        let response: Response = serde_json::from_str(&line).map_err(TermwrightError::Json)?;
        if response.id != id {
            return Err(TermwrightError::Protocol(format!(
                "mismatched response id: expected {id} got {}",
                response.id
            )));
        }

        if let Some(err) = response.error {
            return Err(TermwrightError::Protocol(format!(
                "{}: {}",
                err.code, err.message
            )));
        }

        serde_json::from_value(response.result)
            .map_err(|e| TermwrightError::Protocol(e.to_string()))
    }
}
