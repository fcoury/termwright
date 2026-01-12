//! Termwright CLI - Terminal automation from the command line.

use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;

use clap::{Parser, Subcommand};
use font_kit::source::SystemSource;
use termwright::daemon::server::{DaemonConfig, run_daemon};
use termwright::prelude::*;

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
