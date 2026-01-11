//! Termwright CLI - Terminal automation from the command line.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use termwright::prelude::*;

#[derive(Parser)]
#[command(name = "termwright")]
#[command(author, version, about = "Playwright-like automation for terminal TUI applications")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
            _ => Err(format!("Unknown format: {}. Use text, json, or json-compact", s)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
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
            run_command(cols, rows, wait_for, delay, format, timeout, &command, &args).await?;
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
            std::io::stdout().write_all(&png_bytes).map_err(TermwrightError::Pty)?;
        }
    }

    // Kill the process
    let _ = term.kill().await;

    Ok(())
}
