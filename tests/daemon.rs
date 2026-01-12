use std::path::PathBuf;
use std::time::Duration;

use tempfile::tempdir;
use termwright::daemon::client::DaemonClient;
use termwright::daemon::server::{DaemonConfig, run_daemon};
use termwright::prelude::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn daemon_smoke_test_screen_and_wait() -> Result<()> {
    let dir = tempdir().unwrap();
    let socket: PathBuf = dir.path().join("termwright.sock");

    let term = Terminal::builder()
        .size(80, 24)
        .spawn("sh", &["-c", "printf READY; sleep 2"])
        .await?;

    let server_handle = tokio::spawn(run_daemon(DaemonConfig::new(socket.clone()), term));

    // Wait for listener to be ready.
    let client = loop {
        match DaemonClient::connect_unix(&socket).await {
            Ok(c) => break c,
            Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
        }
    };

    client.handshake().await?;
    client
        .wait_for_text("READY", Some(Duration::from_secs(1)))
        .await?;

    let text = client.screen_text().await?;
    assert!(text.contains("READY"));

    client.close().await?;

    let _ = server_handle.await;

    Ok(())
}
