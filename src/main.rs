//! Termwright CLI - Terminal automation from the command line.

use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;

use clap::{Parser, Subcommand};
use font_kit::source::SystemSource;
use serde::{Deserialize, Serialize};
use termwright::daemon::protocol::Request;
use termwright::daemon::server::{DaemonConfig, run_daemon};
use termwright::info::{
    InfoOverview, capabilities::CapabilitiesInfo, keys::KeysOverview, protocols::ProtocolsOverview,
    steps::StepsOverview,
};
use termwright::prelude::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

mod runner;
mod steps;

use runner::RunStepsOptions;

#[derive(Parser)]
#[command(name = "termwright")]
#[command(
    author,
    version,
    about = "Playwright-like automation for terminal TUI applications"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available monospace fonts for screenshots
    Fonts,

    /// Run a command and capture its output
    Run {
        /// Terminal width
        #[arg(long, default_value = "80")]
        cols: u16,

        /// Terminal height
        #[arg(long, default_value = "24")]
        rows: u16,

        /// Wait for this text to appear before capturing
        #[arg(long)]
        wait_for: Option<String>,

        /// Delay in milliseconds before capturing (after wait_for or startup)
        #[arg(long, default_value = "500")]
        delay: u64,

        /// Output format
        #[arg(long, default_value = "text")]
        format: OutputFormat,

        /// Timeout in seconds for wait conditions
        #[arg(long, default_value = "30")]
        timeout: u64,

        /// The command to run
        #[arg(required = true)]
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Take a screenshot of a TUI application
    Screenshot {
        /// Terminal width
        #[arg(long, default_value = "80")]
        cols: u16,

        /// Terminal height
        #[arg(long, default_value = "24")]
        rows: u16,

        /// Wait for this text to appear before capturing
        #[arg(long)]
        wait_for: Option<String>,

        /// Delay in milliseconds before capturing
        #[arg(long, default_value = "500")]
        delay: u64,

        /// Output file path (defaults to stdout as PNG)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Font name for rendering
        #[arg(long)]
        font: Option<String>,

        /// Font size in pixels
        #[arg(long, default_value = "14")]
        font_size: f32,

        /// Timeout in seconds for wait conditions
        #[arg(long, default_value = "30")]
        timeout: u64,

        /// The command to run
        #[arg(required = true)]
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Run a steps file for end-to-end testing
    RunSteps {
        /// Path to YAML or JSON steps file
        #[arg(required = true)]
        file: PathBuf,

        /// Connect to an existing daemon socket instead of spawning
        #[arg(long)]
        connect: Option<PathBuf>,

        /// Record trace output to artifacts directory
        #[arg(long)]
        trace: bool,
    },

    /// Execute a single daemon request and print the response
    Exec {
        /// Unix socket path
        #[arg(long)]
        socket: PathBuf,

        /// Method name
        #[arg(long)]
        method: String,

        /// Params JSON (defaults to null)
        #[arg(long)]
        params: Option<String>,
    },

    /// Run a long-lived daemon controlling a single TUI session
    Daemon {
        /// Terminal width
        #[arg(long, default_value = "80")]
        cols: u16,

        /// Terminal height
        #[arg(long, default_value = "24")]
        rows: u16,

        /// Unix socket path (defaults to a temp path)
        #[arg(long)]
        socket: Option<PathBuf>,

        /// Start daemon in background and return immediately
        #[arg(long)]
        background: bool,

        /// Internal flag used for background mode
        #[arg(long, hide = true)]
        background_child: bool,

        /// The command to run
        #[arg(required = true)]
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Manage a pool of daemon sessions
    Hub {
        #[command(subcommand)]
        command: HubCommands,
    },

    /// Show information about steps, protocols, and capabilities
    Info {
        #[command(subcommand)]
        command: Option<InfoCommands>,

        /// Output as JSON
        #[arg(long, global = true)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum HubCommands {
    /// Start multiple daemon sessions
    Start {
        /// Number of sessions to start
        #[arg(long, default_value = "1")]
        count: u16,

        /// Terminal width
        #[arg(long, default_value = "80")]
        cols: u16,

        /// Terminal height
        #[arg(long, default_value = "24")]
        rows: u16,

        /// Write hub session info to this JSON file
        #[arg(long)]
        output: Option<PathBuf>,

        /// The command to run
        #[arg(required = true)]
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Stop daemon sessions by socket list or hub file
    Stop {
        /// Unix socket paths to close
        #[arg(long)]
        socket: Vec<PathBuf>,

        /// Load sockets from a JSON hub file
        #[arg(long)]
        input: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum InfoCommands {
    /// List all step types for YAML/JSON step files
    Steps {
        /// Specific step name to show details
        name: Option<String>,
    },

    /// Daemon protocol methods
    Protocols {
        /// Specific method name to show details
        name: Option<String>,
    },

    /// Valid key names for press/hotkey steps
    Keys,

    /// Runtime capabilities and version info
    Capabilities,
}

#[derive(Clone, Debug, Default)]
enum OutputFormat {
    #[default]
    Text,
    Json,
    JsonCompact,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "json-compact" | "jsoncompact" => Ok(OutputFormat::JsonCompact),
            _ => Err(format!(
                "Unknown format: {}. Use text, json, or json-compact",
                s
            )),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fonts => {
            list_fonts();
        }
        Commands::Run {
            cols,
            rows,
            wait_for,
            delay,
            format,
            timeout,
            command,
            args,
        } => {
            run_command(
                cols, rows, wait_for, delay, format, timeout, &command, &args,
            )
            .await?;
        }
        Commands::Screenshot {
            cols,
            rows,
            wait_for,
            delay,
            output,
            font,
            font_size,
            timeout,
            command,
            args,
        } => {
            take_screenshot(
                cols, rows, wait_for, delay, output, font, font_size, timeout, &command, &args,
            )
            .await?;
        }
        Commands::RunSteps {
            file,
            connect,
            trace,
        } => {
            runner::run_steps(&file, RunStepsOptions { connect, trace }).await?;
        }
        Commands::Exec {
            socket,
            method,
            params,
        } => {
            exec_daemon_request(&socket, &method, params.as_deref()).await?;
        }
        Commands::Daemon {
            cols,
            rows,
            socket,
            background,
            background_child,
            command,
            args,
        } => {
            run_daemon_command(
                cols,
                rows,
                socket,
                background,
                background_child,
                &command,
                &args,
            )
            .await?;
        }
        Commands::Hub { command } => match command {
            HubCommands::Start {
                count,
                cols,
                rows,
                output,
                command,
                args,
            } => {
                hub_start(count, cols, rows, output.as_ref(), &command, &args).await?;
            }
            HubCommands::Stop { socket, input } => {
                hub_stop(&socket, input.as_ref()).await?;
            }
        },
        Commands::Info { command, json } => {
            run_info_command(command, json)?;
        }
    }

    Ok(())
}

async fn run_command(
    cols: u16,
    rows: u16,
    wait_for: Option<String>,
    delay: u64,
    format: OutputFormat,
    timeout: u64,
    command: &str,
    args: &[String],
) -> Result<()> {
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let term = Terminal::builder()
        .size(cols, rows)
        .spawn(command, &args_str)
        .await?;

    // Wait for text if specified
    if let Some(text) = wait_for {
        term.expect(&text)
            .timeout(Duration::from_secs(timeout))
            .await?;
    }

    // Additional delay
    tokio::time::sleep(Duration::from_millis(delay)).await;

    // Capture and output
    let screen = term.screen().await;

    match format {
        OutputFormat::Text => {
            println!("{}", screen.text());
        }
        OutputFormat::Json => {
            println!("{}", screen.to_json()?);
        }
        OutputFormat::JsonCompact => {
            println!("{}", screen.to_json_compact()?);
        }
    }

    // Kill the process
    let _ = term.kill().await;

    Ok(())
}

async fn exec_daemon_request(socket: &PathBuf, method: &str, params: Option<&str>) -> Result<()> {
    let params_value = match params {
        Some(raw) => serde_json::from_str(raw).map_err(TermwrightError::Json)?,
        None => serde_json::Value::Null,
    };

    let request = Request {
        id: 1,
        method: method.to_string(),
        params: params_value,
    };

    let stream = UnixStream::connect(socket)
        .await
        .map_err(|e| TermwrightError::Ipc(format!("connect failed: {e}")))?;

    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let mut bytes = serde_json::to_vec(&request).map_err(TermwrightError::Json)?;
    bytes.push(b'\n');
    write_half
        .write_all(&bytes)
        .await
        .map_err(|e| TermwrightError::Ipc(format!("write failed: {e}")))?;
    write_half
        .flush()
        .await
        .map_err(|e| TermwrightError::Ipc(format!("flush failed: {e}")))?;

    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .await
        .map_err(|e| TermwrightError::Ipc(format!("read failed: {e}")))?;
    if n == 0 {
        return Err(TermwrightError::Ipc("empty response".to_string()));
    }

    println!("{}", line.trim_end());
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct HubEntry {
    socket: PathBuf,
    pid: u32,
}

async fn hub_start(
    count: u16,
    cols: u16,
    rows: u16,
    output: Option<&PathBuf>,
    command: &str,
    args: &[String],
) -> Result<()> {
    let mut entries = Vec::new();

    for index in 0..count {
        let socket = std::env::temp_dir().join(format!(
            "termwright-hub-{}-{}.sock",
            std::process::id(),
            index + 1
        ));
        let pid = spawn_background_daemon(cols, rows, &socket, command, args).await?;
        entries.push(HubEntry { socket, pid });
    }

    let json = serde_json::to_string_pretty(&entries).map_err(TermwrightError::Json)?;
    println!("{json}");

    if let Some(path) = output {
        std::fs::write(path, json).map_err(TermwrightError::Pty)?;
    }

    Ok(())
}

async fn hub_stop(sockets: &[PathBuf], input: Option<&PathBuf>) -> Result<()> {
    let mut entries = Vec::new();

    if let Some(path) = input {
        let contents = std::fs::read_to_string(path).map_err(TermwrightError::Pty)?;
        let parsed: Vec<HubEntry> =
            serde_json::from_str(&contents).map_err(TermwrightError::Json)?;
        entries.extend(parsed.into_iter().map(|entry| entry.socket));
    }

    entries.extend_from_slice(sockets);

    if entries.is_empty() {
        return Err(TermwrightError::Protocol(
            "hub stop requires --socket or --input".to_string(),
        ));
    }

    for socket in entries {
        let _ = send_close_request(&socket).await;
    }

    Ok(())
}

async fn spawn_background_daemon(
    cols: u16,
    rows: u16,
    socket: &PathBuf,
    command: &str,
    args: &[String],
) -> Result<u32> {
    let exe = std::env::current_exe()
        .map_err(|e| TermwrightError::SpawnFailed(format!("current_exe failed: {e}")))?;

    let mut child = ProcessCommand::new(exe);
    child
        .arg("daemon")
        .arg("--cols")
        .arg(cols.to_string())
        .arg("--rows")
        .arg(rows.to_string())
        .arg("--socket")
        .arg(socket)
        .arg("--background-child")
        .arg("--")
        .arg(command);

    for arg in args {
        child.arg(arg);
    }

    child
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = child
        .spawn()
        .map_err(|e| TermwrightError::SpawnFailed(format!("failed to spawn daemon: {e}")))?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if tokio::time::Instant::now() > deadline {
            return Err(TermwrightError::Timeout {
                condition: "daemon socket to become ready".to_string(),
                timeout: Duration::from_secs(2),
            });
        }

        match UnixStream::connect(socket).await {
            Ok(stream) => {
                drop(stream);
                break;
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
        }
    }

    Ok(child.id())
}

async fn send_close_request(socket: &PathBuf) -> Result<()> {
    let request = Request {
        id: 1,
        method: "close".to_string(),
        params: serde_json::Value::Null,
    };

    let stream = UnixStream::connect(socket)
        .await
        .map_err(|e| TermwrightError::Ipc(format!("connect failed: {e}")))?;

    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let mut bytes = serde_json::to_vec(&request).map_err(TermwrightError::Json)?;
    bytes.push(b'\n');
    write_half
        .write_all(&bytes)
        .await
        .map_err(|e| TermwrightError::Ipc(format!("write failed: {e}")))?;
    write_half
        .flush()
        .await
        .map_err(|e| TermwrightError::Ipc(format!("flush failed: {e}")))?;

    let mut line = String::new();
    let _ = reader.read_line(&mut line).await;

    Ok(())
}

async fn take_screenshot(
    cols: u16,
    rows: u16,
    wait_for: Option<String>,
    delay: u64,
    output: Option<PathBuf>,
    font: Option<String>,
    font_size: f32,
    timeout: u64,
    command: &str,
    args: &[String],
) -> Result<()> {
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let term = Terminal::builder()
        .size(cols, rows)
        .spawn(command, &args_str)
        .await?;

    // Wait for text if specified
    if let Some(text) = wait_for {
        term.expect(&text)
            .timeout(Duration::from_secs(timeout))
            .await?;
    }

    // Additional delay
    tokio::time::sleep(Duration::from_millis(delay)).await;

    // Take screenshot
    let mut screenshot = term.screenshot().await;

    if let Some(font_name) = font {
        screenshot = screenshot.font(&font_name, font_size);
    }

    // Output
    match output {
        Some(path) => {
            screenshot.save(&path)?;
            eprintln!("Screenshot saved to: {}", path.display());
        }
        None => {
            // Write PNG to stdout
            let png_bytes = screenshot.to_png()?;
            use std::io::Write;
            std::io::stdout()
                .write_all(&png_bytes)
                .map_err(TermwrightError::Pty)?;
        }
    }

    // Kill the process
    let _ = term.kill().await;

    Ok(())
}

async fn run_daemon_command(
    cols: u16,
    rows: u16,
    socket: Option<PathBuf>,
    background: bool,
    background_child: bool,
    command: &str,
    args: &[String],
) -> Result<()> {
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let socket = socket.unwrap_or_else(|| {
        std::env::temp_dir().join(format!("termwright-{}.sock", std::process::id()))
    });

    if background && !background_child {
        let exe = std::env::current_exe()
            .map_err(|e| TermwrightError::SpawnFailed(format!("current_exe failed: {e}")))?;

        let mut child = ProcessCommand::new(exe);
        child
            .arg("daemon")
            .arg("--cols")
            .arg(cols.to_string())
            .arg("--rows")
            .arg(rows.to_string())
            .arg("--socket")
            .arg(&socket)
            .arg("--background-child")
            .arg("--")
            .arg(command);

        for arg in args {
            child.arg(arg);
        }

        child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        child
            .spawn()
            .map_err(|e| TermwrightError::SpawnFailed(format!("failed to spawn daemon: {e}")))?;

        // Wait until the socket is connectable.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(TermwrightError::Timeout {
                    condition: "daemon socket to become ready".to_string(),
                    timeout: Duration::from_secs(2),
                });
            }

            match tokio::net::UnixStream::connect(&socket).await {
                Ok(stream) => {
                    drop(stream);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
            }
        }

        // Parent prints the socket path then exits.
        println!("{}", socket.display());
        return Ok(());
    }

    let terminal = Terminal::builder()
        .size(cols, rows)
        .spawn(command, &args_str)
        .await?;

    // The socket path is the main handle clients need.
    if !background_child {
        println!("{}", socket.display());
    }

    run_daemon(DaemonConfig::new(socket), terminal).await
}

fn run_info_command(command: Option<InfoCommands>, json: bool) -> Result<()> {
    match command {
        None => {
            let overview = InfoOverview::new();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&overview).map_err(TermwrightError::Json)?
                );
            } else {
                print!("{}", overview.to_text());
            }
        }
        Some(InfoCommands::Steps { name }) => {
            let steps = StepsOverview::new();
            match name {
                None => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&steps).map_err(TermwrightError::Json)?
                        );
                    } else {
                        print!("{}", steps.to_text());
                    }
                }
                Some(step_name) => {
                    if let Some(step) = steps.get(&step_name) {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(step).map_err(TermwrightError::Json)?
                            );
                        } else {
                            print!("{}", step.to_text());
                        }
                    } else {
                        return Err(TermwrightError::Protocol(format!(
                            "unknown step: {}. Use `termwright info steps` to list all steps.",
                            step_name
                        )));
                    }
                }
            }
        }
        Some(InfoCommands::Protocols { name }) => {
            let protocols = ProtocolsOverview::new();
            match name {
                None => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&protocols)
                                .map_err(TermwrightError::Json)?
                        );
                    } else {
                        print!("{}", protocols.to_text());
                    }
                }
                Some(method_name) => {
                    if let Some(method) = protocols.get(&method_name) {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(method)
                                    .map_err(TermwrightError::Json)?
                            );
                        } else {
                            print!("{}", method.to_text());
                        }
                    } else {
                        return Err(TermwrightError::Protocol(format!(
                            "unknown method: {}. Use `termwright info protocols` to list all methods.",
                            method_name
                        )));
                    }
                }
            }
        }
        Some(InfoCommands::Keys) => {
            let keys = KeysOverview::new();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&keys).map_err(TermwrightError::Json)?
                );
            } else {
                print!("{}", keys.to_text());
            }
        }
        Some(InfoCommands::Capabilities) => {
            let caps = CapabilitiesInfo::new();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&caps).map_err(TermwrightError::Json)?
                );
            } else {
                print!("{}", caps.to_text());
            }
        }
    }
    Ok(())
}

fn list_fonts() {
    let source = SystemSource::new();

    match source.all_families() {
        Ok(mut families) => {
            families.sort();

            // Filter for likely monospace fonts
            let monospace_keywords = [
                "mono",
                "code",
                "consol",
                "courier",
                "fixed",
                "terminal",
                "menlo",
                "hack",
                "fira",
                "jetbrains",
                "source code",
                "inconsolata",
                "dejavu sans mono",
                "liberation mono",
                "ubuntu mono",
                "cascadia",
                "iosevka",
            ];

            let mut monospace: Vec<_> = families
                .iter()
                .filter(|f| {
                    let lower = f.to_lowercase();
                    monospace_keywords.iter().any(|kw| lower.contains(kw))
                })
                .collect();

            let mut other: Vec<_> = families
                .iter()
                .filter(|f| {
                    let lower = f.to_lowercase();
                    !monospace_keywords.iter().any(|kw| lower.contains(kw))
                })
                .collect();

            monospace.sort();
            other.sort();

            println!("Monospace fonts (recommended for screenshots):");
            println!("----------------------------------------------");
            for font in &monospace {
                println!("  {}", font);
            }

            println!("\nOther fonts:");
            println!("------------");
            for font in &other {
                println!("  {}", font);
            }

            println!(
                "\nTotal: {} fonts ({} monospace)",
                families.len(),
                monospace.len()
            );
        }
        Err(e) => {
            eprintln!("Error listing fonts: {}", e);
        }
    }
}
