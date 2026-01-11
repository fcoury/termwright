//! Example: Testing vim with termwright
//!
//! This example demonstrates the basic termwright workflow:
//! 1. Spawn vim with a test file
//! 2. Wait for vim to be ready
//! 3. Enter insert mode and type text
//! 4. Verify the text appears on screen
//! 5. Output screen state as JSON
//! 6. Quit vim cleanly

use std::time::Duration;
use termwright::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting vim test...\n");

    // Create a temporary file path
    let test_file = "/tmp/termwright_test.txt";

    // Spawn vim
    let term = Terminal::builder()
        .size(80, 24)
        .spawn("vim", &[test_file])
        .await?;

    println!("Vim spawned, waiting for it to be ready...");

    // Wait for vim to show the filename or be ready
    // Vim shows the filename at the bottom, or shows empty buffer indicators
    // We ignore errors here since vim startup varies
    let _ = term.expect("VIM")
        .timeout(Duration::from_secs(5))
        .await;

    // Give vim a moment to fully initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("Vim ready. Current screen state:");
    let screen = term.screen().await;
    println!("---");
    println!("{}", screen.text());
    println!("---\n");

    // Enter insert mode
    println!("Entering insert mode...");
    term.send_key(Key::Char('i')).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Type some text
    let test_text = "Hello, termwright!";
    println!("Typing: {}", test_text);
    term.type_str(test_text).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Exit insert mode
    println!("Exiting insert mode...");
    term.send_key(Key::Escape).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check screen state
    let screen = term.screen().await;
    println!("\nScreen after typing:");
    println!("---");
    println!("{}", screen.text());
    println!("---\n");

    // Verify the text is on screen
    if screen.contains(test_text) {
        println!("SUCCESS: Text '{}' found on screen!", test_text);
    } else {
        println!("WARNING: Text '{}' not found on screen", test_text);
    }

    // Output JSON for AI agents
    println!("\nCompact JSON output:");
    println!("{}", screen.to_json_compact()?);

    // Show cursor position
    let cursor = screen.cursor();
    println!("\nCursor position: row={}, col={}", cursor.row, cursor.col);

    // Take a screenshot
    println!("\nTaking screenshot...");
    let screenshot_path = "/tmp/termwright_vim_screenshot.png";
    term.screenshot().await.save(screenshot_path)?;
    println!("Screenshot saved to: {}", screenshot_path);

    // Quit vim without saving
    println!("\nQuitting vim...");
    term.type_str(":q!").await?;
    term.enter().await?;

    // Wait for vim to exit
    match term.wait_exit().await {
        Ok(code) => println!("Vim exited with code: {}", code),
        Err(e) => println!("Error waiting for exit: {}", e),
    }

    println!("\nTest complete!");

    // Clean up test file if it exists
    let _ = std::fs::remove_file(test_file);

    Ok(())
}
