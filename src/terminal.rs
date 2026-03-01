//! Terminal management and interaction.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::error::{Result, TermwrightError};
use crate::input::{Key, MouseButton, ScrollDirection};
use crate::screen::Screen;
use crate::wait::{DEFAULT_TIMEOUT, WaitBuilder, WaitCondition};

mod csi;
mod osc;

use self::{
    csi::CsiEmulator,
    osc::{OscEmulator, initial_color_state},
};

/// Default terminal width.
pub const DEFAULT_COLS: u16 = 80;
/// Default terminal height.
pub const DEFAULT_ROWS: u16 = 24;

fn encode_sgr_mouse(code: u8, row: u16, col: u16, pressed: bool) -> Vec<u8> {
    // SGR (1006) mouse encoding.
    // Coordinates are 1-based.
    let row = row.saturating_add(1);
    let col = col.saturating_add(1);
    let suffix = if pressed { 'M' } else { 'm' };
    format!("\u{1b}[<{};{};{}{}", code, col, row, suffix).into_bytes()
}

/// Configuration for the terminal.
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
    /// Environment variables to set.
    pub env: HashMap<String, String>,
    /// Working directory.
    pub working_dir: Option<PathBuf>,
    /// Default timeout for operations.
    pub timeout: Duration,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            env: HashMap::new(),
            working_dir: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

/// Builder for creating a Terminal instance.
#[derive(Debug)]
pub struct TerminalBuilder {
    config: TerminalConfig,
    inject_default_env: bool,
    osc_emulation: bool,
}

impl TerminalBuilder {
    /// Create a new terminal builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the terminal size.
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.config.cols = cols;
        self.config.rows = rows;
        self
    }

    /// Set an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.env.insert(key.into(), value.into());
        self
    }

    /// Disable default TERM/COLORTERM injection.
    pub fn no_default_env(mut self) -> Self {
        self.inject_default_env = false;
        self
    }

    /// Disable OSC color query emulation.
    pub fn no_osc_emulation(mut self) -> Self {
        self.osc_emulation = false;
        self
    }

    /// Set the working directory.
    pub fn working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.working_dir = Some(path.into());
        self
    }

    /// Set the default timeout for operations.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Spawn a command in the terminal.
    pub async fn spawn(self, cmd: &str, args: &[&str]) -> Result<Terminal> {
        Terminal::spawn_with_config(
            cmd,
            args,
            self.config,
            self.inject_default_env,
            self.osc_emulation,
        )
        .await
    }
}

impl Default for TerminalBuilder {
    fn default() -> Self {
        Self {
            config: TerminalConfig::default(),
            inject_default_env: true,
            osc_emulation: true,
        }
    }
}

/// A terminal instance wrapping a PTY.
pub struct Terminal {
    /// PTY master handle (used for resize).
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    /// Writer to send input to the PTY.
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// The vt100 parser for terminal emulation.
    parser: Arc<Mutex<vt100::Parser>>,
    /// Configuration.
    config: TerminalConfig,
    /// Handle to the reader task.
    _reader_handle: tokio::task::JoinHandle<()>,
    /// Flag indicating if the process has exited.
    exited: Arc<Mutex<Option<i32>>>,
    /// The child process.
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
}

impl Terminal {
    /// Create a new terminal builder.
    pub fn builder() -> TerminalBuilder {
        TerminalBuilder::new()
    }

    /// Spawn a command with the given configuration.
    async fn spawn_with_config(
        cmd: &str,
        args: &[&str],
        config: TerminalConfig,
        inject_default_env: bool,
        osc_emulation: bool,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TermwrightError::SpawnFailed(e.to_string()))?;

        let mut cmd_builder = CommandBuilder::new(cmd);
        cmd_builder.args(args);

        // Set environment variables
        for (key, value) in &config.env {
            cmd_builder.env(key, value);
        }

        // Inject default terminal env vars unless disabled.
        if inject_default_env {
            if !config.env.contains_key("TERM") {
                cmd_builder.env("TERM", "xterm-256color");
            }
            if !config.env.contains_key("COLORTERM") {
                cmd_builder.env("COLORTERM", "truecolor");
            }
            // NO_COLOR forces many TUIs to disable color surfaces entirely.
            // Clear it unless caller explicitly set NO_COLOR via session env.
            if !config.env.contains_key("NO_COLOR") {
                cmd_builder.env_remove("NO_COLOR");
            }
        }

        // Set working directory if specified
        if let Some(ref cwd) = config.working_dir {
            cmd_builder.cwd(cwd);
        }

        let child = pair
            .slave
            .spawn_command(cmd_builder)
            .map_err(|e| TermwrightError::SpawnFailed(e.to_string()))?;

        let master = pair.master;

        let reader = master
            .try_clone_reader()
            .map_err(|e| TermwrightError::SpawnFailed(e.to_string()))?;

        let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(
            master
                .take_writer()
                .map_err(|e| TermwrightError::SpawnFailed(e.to_string()))?,
        ));

        let parser = Arc::new(Mutex::new(vt100::Parser::new(
            config.rows,
            config.cols,
            1000, // scrollback lines
        )));
        let parser_clone = parser.clone();
        let writer_clone = writer.clone();
        let mut osc = osc_emulation.then(|| OscEmulator::new(initial_color_state()));
        let mut csi = CsiEmulator::new();

        let exited = Arc::new(Mutex::new(None));
        let exited_clone = exited.clone();

        // Background reader task
        let reader_handle = tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - process exited
                        let rt = tokio::runtime::Handle::current();
                        rt.block_on(async {
                            let mut exited = exited_clone.lock().await;
                            *exited = Some(0);
                        });
                        break;
                    }
                    Ok(n) => {
                        let rt = tokio::runtime::Handle::current();
                        rt.block_on(async {
                            let mut parser = parser_clone.lock().await;
                            parser.process(&buf[..n]);

                            let cursor = {
                                let cursor = parser.screen().cursor_position();
                                crate::screen::Position::new(cursor.0, cursor.1)
                            };

                            let mut responses = Vec::new();
                            if let Some(osc) = osc.as_mut() {
                                responses.extend(osc.process_output(&buf[..n]));
                            }
                            responses.extend(csi.process_output(&buf[..n], cursor));

                            drop(parser);

                            if !responses.is_empty() {
                                let mut writer = writer_clone.lock().await;
                                for response in responses {
                                    let _ = writer.write_all(&response);
                                }
                                let _ = writer.flush();
                            }
                        });
                    }
                    Err(_) => {
                        let rt = tokio::runtime::Handle::current();
                        rt.block_on(async {
                            let mut exited = exited_clone.lock().await;
                            *exited = Some(-1);
                        });
                        break;
                    }
                }
            }
        });

        // Give the process a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(Self {
            master: Arc::new(Mutex::new(master)),
            writer,
            parser,
            config,
            _reader_handle: reader_handle,
            exited,
            child: Arc::new(Mutex::new(child)),
        })
    }

    /// Get a snapshot of the current screen state.
    pub async fn screen(&self) -> Screen {
        let parser = self.parser.lock().await;
        Screen::from_vt100(parser.screen())
    }

    /// Type a string of text into the terminal.
    pub async fn type_str(&self, text: &str) -> Result<&Self> {
        let mut writer = self.writer.lock().await;
        writer
            .write_all(text.as_bytes())
            .map_err(TermwrightError::Pty)?;
        writer.flush().map_err(TermwrightError::Pty)?;
        Ok(self)
    }

    /// Send a key to the terminal.
    pub async fn send_key(&self, key: Key) -> Result<&Self> {
        let bytes = key.to_escape_sequence();
        let mut writer = self.writer.lock().await;
        writer.write_all(&bytes).map_err(TermwrightError::Pty)?;
        writer.flush().map_err(TermwrightError::Pty)?;
        Ok(self)
    }

    /// Press Enter key.
    pub async fn enter(&self) -> Result<&Self> {
        self.send_key(Key::Enter).await
    }

    /// Press Escape key.
    pub async fn escape(&self) -> Result<&Self> {
        self.send_key(Key::Escape).await
    }

    /// Send raw bytes to the terminal.
    pub async fn send_raw(&self, bytes: &[u8]) -> Result<&Self> {
        let mut writer = self.writer.lock().await;
        writer.write_all(bytes).map_err(TermwrightError::Pty)?;
        writer.flush().map_err(TermwrightError::Pty)?;
        Ok(self)
    }

    /// Resize the PTY.
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<&Self> {
        let master = self.master.lock().await;
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| {
                TermwrightError::Pty(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
        Ok(self)
    }

    /// Move the mouse cursor (hover).
    ///
    /// Note: Many TUIs ignore this unless mouse reporting is enabled.
    pub async fn mouse_move(
        &self,
        row: u16,
        col: u16,
        held_buttons: Vec<MouseButton>,
    ) -> Result<&Self> {
        let base_code = held_buttons
            .first()
            .copied()
            .map(|b| b.press_code())
            .unwrap_or(3);

        let code = base_code.saturating_add(32);
        let bytes = encode_sgr_mouse(code, row, col, true);
        self.send_raw(&bytes).await
    }

    /// Click a mouse button at the given cell position.
    pub async fn mouse_click(&self, row: u16, col: u16, button: MouseButton) -> Result<&Self> {
        self.mouse_down(row, col, button).await?;
        self.mouse_up(row, col).await
    }

    /// Press a mouse button at the given cell position.
    pub async fn mouse_down(&self, row: u16, col: u16, button: MouseButton) -> Result<&Self> {
        let code = button.press_code();
        let bytes = encode_sgr_mouse(code, row, col, true);
        self.send_raw(&bytes).await
    }

    /// Release any mouse button at the given cell position.
    pub async fn mouse_up(&self, row: u16, col: u16) -> Result<&Self> {
        let bytes = encode_sgr_mouse(3, row, col, false);
        self.send_raw(&bytes).await
    }

    /// Scroll the mouse wheel at the given cell position.
    ///
    /// Sends `count` scroll events in the given direction.
    pub async fn mouse_scroll(
        &self,
        row: u16,
        col: u16,
        direction: ScrollDirection,
        count: u16,
    ) -> Result<&Self> {
        let bytes = encode_sgr_mouse(direction.sgr_code(), row, col, true);
        for _ in 0..count {
            self.send_raw(&bytes).await?;
        }
        Ok(self)
    }

    /// Wait for specific text to appear on screen.
    pub fn expect(&self, text: &str) -> ExpectBuilder<'_> {
        ExpectBuilder {
            terminal: self,
            wait: WaitBuilder::new(WaitCondition::TextAppears(text.to_string())),
        }
    }

    /// Wait for specific text to disappear from screen.
    pub fn expect_gone(&self, text: &str) -> ExpectBuilder<'_> {
        ExpectBuilder {
            terminal: self,
            wait: WaitBuilder::new(WaitCondition::TextDisappears(text.to_string())),
        }
    }

    /// Wait for a regex pattern to match on screen.
    pub fn expect_pattern(&self, pattern: &str) -> ExpectBuilder<'_> {
        ExpectBuilder {
            terminal: self,
            wait: WaitBuilder::new(WaitCondition::PatternMatches(pattern.to_string())),
        }
    }

    /// Wait for a regex pattern to stop matching on screen.
    pub fn expect_pattern_gone(&self, pattern: &str) -> ExpectBuilder<'_> {
        ExpectBuilder {
            terminal: self,
            wait: WaitBuilder::new(WaitCondition::PatternNotMatches(pattern.to_string())),
        }
    }

    /// Wait for the cursor to reach a specific position.
    pub fn wait_cursor(&self, position: crate::screen::Position) -> ExpectBuilder<'_> {
        ExpectBuilder {
            terminal: self,
            wait: WaitBuilder::new(WaitCondition::CursorAt(position)),
        }
    }

    /// Wait for the screen to stabilize (no changes for the given duration).
    pub fn wait_idle(&self, duration: Duration) -> ExpectBuilder<'_> {
        ExpectBuilder {
            terminal: self,
            wait: WaitBuilder::new(WaitCondition::ScreenStable(duration)),
        }
    }

    /// Wait for the process to exit.
    pub async fn wait_exit(&self) -> Result<i32> {
        let wait = WaitBuilder::new(WaitCondition::ProcessExit).timeout(self.config.timeout);

        let deadline = Instant::now() + wait.get_timeout();

        while Instant::now() < deadline {
            let exited = self.exited.lock().await;
            if let Some(code) = *exited {
                return Ok(code);
            }
            drop(exited);
            tokio::time::sleep(wait.get_poll_interval()).await;
        }

        Err(wait.timeout_error())
    }

    /// Check if the process has exited.
    pub async fn has_exited(&self) -> bool {
        let exited = self.exited.lock().await;
        exited.is_some()
    }

    /// Kill the terminal process.
    pub async fn kill(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        child
            .kill()
            .map_err(|e| TermwrightError::SpawnFailed(format!("Failed to kill process: {}", e)))?;
        Ok(())
    }

    /// Get the terminal configuration.
    pub fn config(&self) -> &TerminalConfig {
        &self.config
    }

    /// Take a screenshot of the current screen.
    ///
    /// Returns a Screenshot that can be saved to a file or converted to PNG bytes.
    pub async fn screenshot(&self) -> crate::output::Screenshot {
        let screen = self.screen().await;
        crate::output::Screenshot::new(screen)
    }

    /// Execute a wait condition.
    async fn execute_wait(&self, wait: &WaitBuilder) -> Result<()> {
        let condition = wait.condition();
        let deadline = Instant::now() + wait.get_timeout();
        let poll_interval = wait.get_poll_interval();

        // For screen stability, we need to track the stable duration
        let stability_duration = if let WaitCondition::ScreenStable(d) = condition {
            Some(*d)
        } else {
            None
        };

        let mut prev_screen: Option<Screen> = None;
        let mut stable_since: Option<Instant> = None;

        while Instant::now() < deadline {
            let screen = self.screen().await;

            // Check for process exit if that's what we're waiting for
            if matches!(condition, WaitCondition::ProcessExit) {
                if self.has_exited().await {
                    return Ok(());
                }
            } else if let Some(stability_dur) = stability_duration {
                // For screen stability, check if screen hasn't changed
                let is_stable = prev_screen
                    .as_ref()
                    .map(|prev| prev.text() == screen.text())
                    .unwrap_or(false);

                if is_stable {
                    if let Some(since) = stable_since {
                        if since.elapsed() >= stability_dur {
                            return Ok(());
                        }
                    } else {
                        stable_since = Some(Instant::now());
                    }
                } else {
                    stable_since = None;
                }

                prev_screen = Some(screen);
            } else if condition.is_satisfied(&screen, prev_screen.as_ref()) {
                return Ok(());
            }

            tokio::time::sleep(poll_interval).await;
        }

        Err(wait.timeout_error())
    }
}

/// Builder for expect operations with fluent API.
pub struct ExpectBuilder<'a> {
    terminal: &'a Terminal,
    wait: WaitBuilder,
}

impl<'a> ExpectBuilder<'a> {
    /// Set the timeout for this wait operation.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.wait = self.wait.timeout(timeout);
        self
    }

    /// Execute the wait and return when the condition is met.
    pub async fn await_condition(self) -> Result<()> {
        self.terminal.execute_wait(&self.wait).await
    }
}

// Allow awaiting ExpectBuilder directly
impl<'a> std::future::IntoFuture for ExpectBuilder<'a> {
    type Output = Result<()>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.await_condition())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_config_default() {
        let config = TerminalConfig::default();
        assert_eq!(config.cols, DEFAULT_COLS);
        assert_eq!(config.rows, DEFAULT_ROWS);
    }

    #[test]
    fn test_builder_size() {
        let builder = TerminalBuilder::new().size(120, 40);
        assert_eq!(builder.config.cols, 120);
        assert_eq!(builder.config.rows, 40);
    }
}
