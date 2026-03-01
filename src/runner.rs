use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::Local;
use serde::Serialize;
use tokio::time::sleep;

use crate::steps::{
    ArtifactMode, ArtifactsConfig, ExpectPatternStep, ExpectTextStep, NotExpectPatternStep,
    NotExpectTextStep, ScreenshotStep, SessionConfig, Step, StepsFile, WaitForPatternGoneStep,
    WaitForTextGoneStep,
};
use termwright::input::{MouseButton, ScrollDirection};
use termwright::daemon::client::DaemonClient;
use termwright::daemon::server::{DaemonConfig, run_daemon};
use termwright::error::{Result, TermwrightError};
use termwright::terminal::Terminal;

pub struct RunStepsOptions {
    pub connect: Option<PathBuf>,
    pub trace: bool,
    pub no_default_env: bool,
    pub no_osc_emulation: bool,
}

pub async fn run_steps(path: &Path, options: RunStepsOptions) -> Result<()> {
    let steps_file = StepsFile::load(path)?;

    let (client, daemon_handle) = if let Some(socket) = options.connect.clone() {
        let client = connect_daemon(&socket).await?;
        (client, None)
    } else {
        let session = steps_file.session.as_ref().ok_or_else(|| {
            TermwrightError::Protocol(
                "session config is required when not using --connect".to_string(),
            )
        })?;

        let disable_default_env = options.no_default_env || session.no_default_env;
        let disable_osc_emulation = options.no_osc_emulation || session.no_osc_emulation;
        let (socket, handle) =
            spawn_daemon(session, disable_default_env, disable_osc_emulation).await?;
        let client = connect_daemon(&socket).await?;
        (client, Some(handle))
    };

    let artifacts_dir = prepare_artifacts_dir(&steps_file.artifacts, options.trace)?;
    let mut step_index = 0usize;
    let mut trace_entries = Vec::new();

    for step in &steps_file.steps {
        step_index += 1;
        let trace_before = if options.trace {
            Some(capture_trace_snapshot(&client).await?)
        } else {
            None
        };
        let started = Instant::now();

        let result = execute_step(&client, step).await;

        let trace_after = if options.trace {
            Some(capture_trace_snapshot(&client).await?)
        } else {
            None
        };

        if options.trace {
            trace_entries.push(TraceEntry::new(
                step_index,
                step,
                started.elapsed(),
                trace_before.as_ref(),
                trace_after.as_ref(),
                result.as_ref().err(),
            ));
        }

        if let Err(err) = result {
            if steps_file.artifacts.mode != ArtifactMode::Off {
                if let Some(dir) = artifacts_dir.as_ref() {
                    let _ = capture_artifacts(&client, dir, step_index, "failure").await;
                }
            }
            if options.trace {
                if let Some(dir) = artifacts_dir.as_ref() {
                    let _ = write_trace(dir, &trace_entries);
                }
            }
            if let Some(handle) = daemon_handle {
                let _ = client.close().await;
                let _ = handle.await;
            }
            return Err(err);
        }

        if steps_file.artifacts.mode == ArtifactMode::Always {
            if let Some(dir) = artifacts_dir.as_ref() {
                capture_artifacts(&client, dir, step_index, "step").await?;
            }
        }

        if let Step::Screenshot { screenshot } = step {
            if let Some(dir) = artifacts_dir.as_ref() {
                save_screenshot(&client, dir, step_index, screenshot).await?;
            } else {
                return Err(TermwrightError::Protocol(
                    "screenshot step requires artifacts mode".to_string(),
                ));
            }
        }
    }

    if options.trace {
        if let Some(dir) = artifacts_dir.as_ref() {
            write_trace(dir, &trace_entries)?;
        }
    }

    if let Some(handle) = daemon_handle {
        let _ = client.close().await;
        let _ = handle.await;
    }

    Ok(())
}

async fn execute_step(client: &DaemonClient, step: &Step) -> Result<()> {
    match step {
        Step::WaitForText { wait_for_text } => {
            client
                .wait_for_text(&wait_for_text.text, timeout(wait_for_text.timeout_ms))
                .await
        }
        Step::WaitForPattern { wait_for_pattern } => {
            client
                .wait_for_pattern(
                    &wait_for_pattern.pattern,
                    timeout(wait_for_pattern.timeout_ms),
                )
                .await
        }
        Step::WaitForIdle { wait_for_idle } => {
            client
                .wait_for_idle(
                    Duration::from_millis(wait_for_idle.idle_ms),
                    timeout(wait_for_idle.timeout_ms),
                )
                .await
        }
        Step::WaitForTextGone { wait_for_text_gone } => {
            wait_for_text_gone_step(client, wait_for_text_gone).await
        }
        Step::WaitForPatternGone {
            wait_for_pattern_gone,
        } => wait_for_pattern_gone_step(client, wait_for_pattern_gone).await,
        Step::Press { press } => client.press(&press.key).await,
        Step::Type { r#type } => client.r#type(&r#type.text).await,
        Step::Hotkey { hotkey } => {
            client
                .hotkey(
                    hotkey.ctrl.unwrap_or(false),
                    hotkey.alt.unwrap_or(false),
                    hotkey.ch,
                )
                .await
        }
        Step::ExpectText { expect_text } => expect_text_step(client, expect_text).await,
        Step::ExpectPattern { expect_pattern } => expect_pattern_step(client, expect_pattern).await,
        Step::NotExpectText { not_expect_text } => {
            not_expect_text_step(client, not_expect_text).await
        }
        Step::NotExpectPattern { not_expect_pattern } => {
            not_expect_pattern_step(client, not_expect_pattern).await
        }
        Step::Screenshot { .. } => Ok(()),
        Step::MouseClick { mouse_click } => {
            let button = mouse_click
                .button
                .as_deref()
                .unwrap_or("left")
                .parse::<MouseButton>()
                .map_err(|e| TermwrightError::Protocol(e.to_string()))?;
            client
                .mouse_click(mouse_click.row, mouse_click.col, button)
                .await
        }
        Step::MouseScroll { mouse_scroll } => {
            let direction = mouse_scroll
                .direction
                .parse::<ScrollDirection>()
                .map_err(|e| TermwrightError::Protocol(e.to_string()))?;
            client
                .mouse_scroll(
                    mouse_scroll.row,
                    mouse_scroll.col,
                    direction,
                    mouse_scroll.count,
                )
                .await
        }
        Step::MouseMove { mouse_move } => {
            client
                .mouse_move(mouse_move.row, mouse_move.col)
                .await
        }
        Step::WaitForExit { wait_for_exit } => {
            client
                .wait_for_exit(timeout(wait_for_exit.timeout_ms))
                .await?;
            Ok(())
        }
        Step::Resize { resize } => {
            client.resize(resize.cols, resize.rows).await
        }
        Step::Sleep { sleep: sleep_step } => {
            tokio::time::sleep(Duration::from_millis(sleep_step.ms)).await;
            Ok(())
        }
        Step::Raw { raw } => {
            client.raw(&raw.bytes_base64).await
        }
    }
}

async fn expect_text_step(client: &DaemonClient, step: &ExpectTextStep) -> Result<()> {
    client
        .wait_for_text(&step.text, timeout(step.timeout_ms))
        .await
}

async fn expect_pattern_step(client: &DaemonClient, step: &ExpectPatternStep) -> Result<()> {
    client
        .wait_for_pattern(&step.pattern, timeout(step.timeout_ms))
        .await
}

async fn wait_for_text_gone_step(client: &DaemonClient, step: &WaitForTextGoneStep) -> Result<()> {
    client
        .wait_for_text_gone(&step.text, timeout(step.timeout_ms))
        .await
}

async fn wait_for_pattern_gone_step(
    client: &DaemonClient,
    step: &WaitForPatternGoneStep,
) -> Result<()> {
    client
        .wait_for_pattern_gone(&step.pattern, timeout(step.timeout_ms))
        .await
}

async fn not_expect_text_step(client: &DaemonClient, step: &NotExpectTextStep) -> Result<()> {
    client.not_expect_text(&step.text).await
}

async fn not_expect_pattern_step(client: &DaemonClient, step: &NotExpectPatternStep) -> Result<()> {
    client.not_expect_pattern(&step.pattern).await
}

fn timeout(timeout_ms: Option<u64>) -> Option<Duration> {
    timeout_ms.map(Duration::from_millis)
}

async fn spawn_daemon(
    session: &SessionConfig,
    no_default_env: bool,
    no_osc_emulation: bool,
) -> Result<(PathBuf, tokio::task::JoinHandle<Result<()>>)> {
    let (command, args) = session.command_and_args()?;
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let mut builder = Terminal::builder().size(session.cols(), session.rows());
    if no_default_env {
        builder = builder.no_default_env();
    }
    if no_osc_emulation {
        builder = builder.no_osc_emulation();
    }
    for (key, value) in &session.env {
        builder = builder.env(key, value);
    }
    if let Some(cwd) = session.cwd.as_ref() {
        builder = builder.working_dir(cwd);
    }

    let terminal = builder.spawn(&command, &args_ref).await?;

    let socket = std::env::temp_dir().join(format!(
        "termwright-steps-{}-{}.sock",
        std::process::id(),
        Local::now().format("%Y%m%d%H%M%S")
    ));

    let handle = tokio::spawn(run_daemon(DaemonConfig::new(socket.clone()), terminal));

    Ok((socket, handle))
}

async fn connect_daemon(socket: &Path) -> Result<DaemonClient> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);

    loop {
        match DaemonClient::connect_unix(socket).await {
            Ok(client) => return Ok(client),
            Err(e) => {
                if tokio::time::Instant::now() > deadline {
                    return Err(e);
                }
                sleep(Duration::from_millis(20)).await;
            }
        }
    }
}

fn prepare_artifacts_dir(config: &ArtifactsConfig, trace: bool) -> Result<Option<PathBuf>> {
    if config.mode == ArtifactMode::Off && !trace {
        return Ok(None);
    }

    let base_dir = config.base_dir();
    let run_dir = base_dir.join(Local::now().format("%Y%m%d-%H%M%S").to_string());
    fs::create_dir_all(&run_dir).map_err(TermwrightError::Pty)?;
    Ok(Some(run_dir))
}

async fn capture_artifacts(
    client: &DaemonClient,
    dir: &Path,
    step_index: usize,
    label: &str,
) -> Result<()> {
    let screen_text = client.screen_text().await?;
    let screen_json = client.screen_json().await?;

    let base = format!("{label}-{:03}", step_index);
    let text_path = dir.join(format!("{base}-screen.txt"));
    let json_path = dir.join(format!("{base}-screen.json"));

    fs::write(text_path, screen_text).map_err(TermwrightError::Pty)?;
    fs::write(json_path, screen_json).map_err(TermwrightError::Pty)?;

    Ok(())
}

async fn save_screenshot(
    client: &DaemonClient,
    dir: &Path,
    step_index: usize,
    screenshot: &ScreenshotStep,
) -> Result<()> {
    let png_bytes = client.screenshot_png().await?;
    let name = screenshot
        .name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("step-{:03}-screenshot", step_index));

    let path = dir.join(format!("{name}.png"));
    fs::write(path, png_bytes).map_err(TermwrightError::Pty)?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct TraceEntry {
    step: usize,
    action: String,
    duration_ms: u128,
    before_hash: Option<u64>,
    after_hash: Option<u64>,
    error: Option<String>,
}

impl TraceEntry {
    fn new(
        step: usize,
        action_step: &Step,
        duration: Duration,
        before: Option<&TraceSnapshot>,
        after: Option<&TraceSnapshot>,
        error: Option<&TermwrightError>,
    ) -> Self {
        Self {
            step,
            action: step_label(action_step),
            duration_ms: duration.as_millis(),
            before_hash: before.map(|s| s.hash),
            after_hash: after.map(|s| s.hash),
            error: error.map(|e| e.to_string()),
        }
    }
}

struct TraceSnapshot {
    hash: u64,
}

async fn capture_trace_snapshot(client: &DaemonClient) -> Result<TraceSnapshot> {
    let text = client.screen_text().await?;
    Ok(TraceSnapshot {
        hash: hash_text(&text),
    })
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn step_label(step: &Step) -> String {
    match step {
        Step::WaitForText { .. } => "waitForText".to_string(),
        Step::WaitForPattern { .. } => "waitForPattern".to_string(),
        Step::WaitForIdle { .. } => "waitForIdle".to_string(),
        Step::WaitForTextGone { .. } => "waitForTextGone".to_string(),
        Step::WaitForPatternGone { .. } => "waitForPatternGone".to_string(),
        Step::Press { .. } => "press".to_string(),
        Step::Type { .. } => "type".to_string(),
        Step::Hotkey { .. } => "hotkey".to_string(),
        Step::ExpectText { .. } => "expectText".to_string(),
        Step::ExpectPattern { .. } => "expectPattern".to_string(),
        Step::NotExpectText { .. } => "notExpectText".to_string(),
        Step::NotExpectPattern { .. } => "notExpectPattern".to_string(),
        Step::Screenshot { .. } => "screenshot".to_string(),
        Step::MouseClick { .. } => "mouseClick".to_string(),
        Step::MouseScroll { .. } => "mouseScroll".to_string(),
        Step::MouseMove { .. } => "mouseMove".to_string(),
        Step::WaitForExit { .. } => "waitForExit".to_string(),
        Step::Resize { .. } => "resize".to_string(),
        Step::Sleep { .. } => "sleep".to_string(),
        Step::Raw { .. } => "raw".to_string(),
    }
}

fn write_trace(dir: &Path, trace: &[TraceEntry]) -> Result<()> {
    let path = dir.join("trace.json");
    let json = serde_json::to_string_pretty(trace).map_err(TermwrightError::Json)?;
    fs::write(path, json).map_err(TermwrightError::Pty)?;
    Ok(())
}
