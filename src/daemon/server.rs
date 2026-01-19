use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::daemon::protocol::*;
use crate::error::{Result, TermwrightError};
use crate::input::{Key, MouseButton};
use crate::terminal::Terminal;

const PROTOCOL_VERSION: u32 = 1;

/// Result from serving a client connection.
enum ClientResult {
    /// Client disconnected normally, ready to accept next client.
    Continue,
    /// Client sent `close` command, daemon should exit.
    Close,
}

pub struct DaemonConfig {
    pub socket_path: PathBuf,
}

impl DaemonConfig {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }
}

pub async fn run_daemon(config: DaemonConfig, terminal: Terminal) -> Result<()> {
    let socket_path = config.socket_path;
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)
            .map_err(|e| TermwrightError::Ipc(format!("failed to remove socket: {e}")))?;
    }

    let listener = UnixListener::bind(&socket_path)
        .map_err(|e| TermwrightError::Ipc(format!("failed to bind socket: {e}")))?;

    let result = accept_clients(listener, &terminal).await;

    // Best-effort cleanup
    let _ = terminal.kill().await;
    let _ = std::fs::remove_file(&socket_path);

    result
}

/// Accept multiple client connections until `close` is called or process exits.
async fn accept_clients(listener: UnixListener, terminal: &Terminal) -> Result<()> {
    loop {
        // Check if the spawned process has exited
        if terminal.has_exited().await {
            return Ok(());
        }

        // Accept with a timeout so we can periodically check process status
        let accept_result =
            tokio::time::timeout(Duration::from_millis(500), listener.accept()).await;

        let stream = match accept_result {
            Ok(Ok((stream, _))) => stream,
            Ok(Err(e)) => {
                return Err(TermwrightError::Ipc(format!("accept failed: {e}")));
            }
            Err(_) => {
                // Timeout - loop back to check process status
                continue;
            }
        };

        // Serve this client; if they send `close`, we exit the loop
        match serve_client(stream, terminal).await {
            Ok(ClientResult::Continue) => {
                // Client disconnected normally, accept next client
                continue;
            }
            Ok(ClientResult::Close) => {
                // Client sent `close` command
                return Ok(());
            }
            Err(e) => {
                // Log error but keep accepting clients
                eprintln!("Client error: {e}");
                continue;
            }
        }
    }
}

async fn serve_client(stream: UnixStream, terminal: &Terminal) -> Result<ClientResult> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| TermwrightError::Ipc(format!("read failed: {e}")))?;
        if n == 0 {
            // Client disconnected, ready for next client
            return Ok(ClientResult::Continue);
        }

        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::err(0, "parse_error", e.to_string());
                write_response(&mut write_half, &resp).await?;
                continue;
            }
        };

        let resp = handle_request(terminal, req).await;
        write_response(&mut write_half, &resp).await?;

        if resp.error.as_ref().is_some_and(|e| e.code == "closing") {
            // Client sent `close` command, daemon should exit
            return Ok(ClientResult::Close);
        }
    }
}

async fn write_response(
    write_half: &mut tokio::net::unix::OwnedWriteHalf,
    resp: &Response,
) -> Result<()> {
    let mut bytes = serde_json::to_vec(resp).map_err(TermwrightError::Json)?;
    bytes.push(b'\n');
    write_half
        .write_all(&bytes)
        .await
        .map_err(|e| TermwrightError::Ipc(format!("write failed: {e}")))?;
    write_half
        .flush()
        .await
        .map_err(|e| TermwrightError::Ipc(format!("flush failed: {e}")))?;
    Ok(())
}

async fn handle_request(terminal: &Terminal, req: Request) -> Response {
    let id = req.id;

    let result: Result<Response> = (|| async {
        match req.method.as_str() {
            "handshake" => {
                let value = HandshakeResult {
                    protocol_version: PROTOCOL_VERSION,
                    termwright_version: env!("CARGO_PKG_VERSION").to_string(),
                    pid: std::process::id(),
                };
                Ok(Response::ok(id, value)?)
            }
            "status" => {
                let exited = terminal.has_exited().await;
                let exit_code = if exited {
                    Some(terminal.wait_exit().await.unwrap_or(-1))
                } else {
                    None
                };
                Ok(Response::ok(id, StatusResult { exited, exit_code })?)
            }
            "screen" => {
                let params: ScreenParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let screen = terminal.screen().await;

                match params.format {
                    ScreenFormat::Text => Ok(Response::ok(id, screen.text())?),
                    ScreenFormat::Json => Ok(Response::ok(id, screen)?),
                    ScreenFormat::JsonCompact => Ok(Response::ok(id, screen.to_json_compact()?)?),
                }
            }
            "screenshot" => {
                let params: ScreenshotParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let mut screenshot = terminal.screenshot().await;
                if let Some(font) = params.font {
                    screenshot = screenshot.font(&font, params.font_size.unwrap_or(14.0));
                }
                if let Some(line_height) = params.line_height {
                    screenshot = screenshot.line_height(line_height);
                }

                let png = screenshot.to_png()?;
                let png_base64 = base64::engine::general_purpose::STANDARD.encode(png);
                Ok(Response::ok(id, ScreenshotResult { png_base64 })?)
            }
            "type" => {
                let params: TypeParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;
                terminal.type_str(&params.text).await?;
                Ok(Response::ok_empty(id))
            }
            "press" => {
                let params: PressParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;
                let key = parse_key(&params.key)?;
                terminal.send_key(key).await?;
                Ok(Response::ok_empty(id))
            }
            "hotkey" => {
                let params: HotkeyParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                if params.ctrl.unwrap_or(false) {
                    terminal.send_key(Key::Ctrl(params.ch)).await?;
                } else if params.alt.unwrap_or(false) {
                    terminal.send_key(Key::Alt(params.ch)).await?;
                } else {
                    terminal.send_key(Key::Char(params.ch)).await?;
                }

                Ok(Response::ok_empty(id))
            }
            "raw" => {
                let params: RawParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(params.bytes_base64)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;
                terminal.send_raw(&bytes).await?;
                Ok(Response::ok_empty(id))
            }
            "mouse_move" => {
                let params: MouseMoveParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let held = parse_mouse_buttons(params.buttons.as_deref())?;
                terminal.mouse_move(params.row, params.col, held).await?;
                Ok(Response::ok_empty(id))
            }
            "mouse_click" => {
                let params: MouseClickParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let button = params
                    .button
                    .as_deref()
                    .unwrap_or("left")
                    .parse::<MouseButton>()
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                terminal.mouse_click(params.row, params.col, button).await?;
                Ok(Response::ok_empty(id))
            }
            "wait_for_text" => {
                let params: WaitForTextParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let mut waiter = terminal.expect(&params.text);
                if let Some(timeout_ms) = params.timeout_ms {
                    waiter = waiter.timeout(Duration::from_millis(timeout_ms));
                }
                waiter.await?;
                Ok(Response::ok_empty(id))
            }
            "wait_for_pattern" => {
                let params: WaitForPatternParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let mut waiter = terminal.expect_pattern(&params.pattern);
                if let Some(timeout_ms) = params.timeout_ms {
                    waiter = waiter.timeout(Duration::from_millis(timeout_ms));
                }
                waiter.await?;
                Ok(Response::ok_empty(id))
            }
            "wait_for_idle" => {
                let params: WaitForIdleParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let mut waiter = terminal.wait_idle(Duration::from_millis(params.idle_ms));
                if let Some(timeout_ms) = params.timeout_ms {
                    waiter = waiter.timeout(Duration::from_millis(timeout_ms));
                }
                waiter.await?;
                Ok(Response::ok_empty(id))
            }
            "wait_for_exit" => {
                let params: WaitForExitParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;

                let exit_code = if let Some(timeout_ms) = params.timeout_ms {
                    tokio::time::timeout(Duration::from_millis(timeout_ms), terminal.wait_exit())
                        .await
                        .map_err(|_| TermwrightError::Timeout {
                            condition: "process to exit".to_string(),
                            timeout: Duration::from_millis(timeout_ms),
                        })??
                } else {
                    terminal.wait_exit().await?
                };

                Ok(Response::ok(id, WaitForExitResult { exit_code })?)
            }
            "resize" => {
                let params: ResizeParams = serde_json::from_value(req.params)
                    .map_err(|e| TermwrightError::Protocol(e.to_string()))?;
                terminal.resize(params.cols, params.rows).await?;
                Ok(Response::ok_empty(id))
            }
            "close" => {
                let _ = terminal.kill().await;
                Ok(Response::err(id, "closing", "closing"))
            }
            other => Ok(Response::err(
                id,
                "unknown_method",
                format!("unknown method: {other}"),
            )),
        }
    })()
    .await;

    match result {
        Ok(r) => r,
        Err(e) => Response::err(id, "error", e.to_string()),
    }
}

fn parse_key(input: &str) -> Result<Key> {
    let normalized = input.trim().to_lowercase();

    let key = match normalized.as_str() {
        "enter" => Key::Enter,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "page_up" => Key::PageUp,
        "pagedown" | "page_down" => Key::PageDown,
        _ if normalized.starts_with('f') => {
            let n: u8 = normalized[1..]
                .parse()
                .map_err(|_| TermwrightError::Protocol(format!("invalid key: {input}")))?;
            Key::F(n)
        }
        _ => {
            let mut chars = input.chars();
            let ch = chars
                .next()
                .ok_or_else(|| TermwrightError::Protocol("empty key".to_string()))?;
            if chars.next().is_some() {
                return Err(TermwrightError::Protocol(format!("invalid key: {input}")));
            }
            Key::Char(ch)
        }
    };

    Ok(key)
}

fn parse_mouse_buttons(buttons: Option<&[String]>) -> Result<Vec<MouseButton>> {
    let Some(buttons) = buttons else {
        return Ok(Vec::new());
    };

    buttons
        .iter()
        .map(|b| {
            b.parse::<MouseButton>()
                .map_err(|e| TermwrightError::Protocol(e.to_string()))
        })
        .collect()
}
