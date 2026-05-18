use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rmux_sdk::{
    EnsureSession, Pane, PaneOutputChunk, PaneOutputStart, Rmux, SessionName, TerminalSizeSpec,
};
use serde::Deserialize;

const INDEX_HTML: &str = include_str!("../static/index.html");
const DEFAULT_SESSION: &str = "web-claude";
const DEFAULT_PORT: u16 = 8080;
const SDK_DAEMON_BINARY_ENV: &str = "RMUX_SDK_DAEMON_BINARY";
const WINDOWS_PIPE: &str = r"\\.\pipe\rmux-demo-web-claude";
const DEFAULT_UNIX_CMD: &str =
    "IS_DEMO=1 claude --dangerously-skip-permissions --permission-mode bypassPermissions || exec bash";
const DEFAULT_WINDOWS_CMD: &str =
    "claude --dangerously-skip-permissions --permission-mode bypassPermissions";

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientEvent {
    Data { data: String },
    Resize { cols: u16, rows: u16 },
}

#[tokio::main]
async fn main() -> Result<()> {
    force_path_rmux_binary();
    match env::args().nth(1).as_deref() {
        Some("check") => check(),
        Some("cleanup") => cleanup().await,
        Some("serve-only") => serve(false).await,
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: web-claude-demo [check|cleanup|serve-only]");
            Ok(())
        }
        None => serve(true).await,
    }
}

fn force_path_rmux_binary() {
    env::set_var(SDK_DAEMON_BINARY_ENV, "rmux");
}

async fn serve(attach: bool) -> Result<()> {
    check()?;

    let pane = ensure_web_pane().await?;
    let app = Router::new()
        .route("/", get(|| async { Html(INDEX_HTML) }))
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws))
        .with_state(pane);

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("web terminal: http://127.0.0.1:{port}");
    println!("iPhone URL:   http://<mac-lan-ip>:{port}");
    println!("native attach:");
    println!("  {}", attach_command()?);

    if !attach {
        axum::serve(listener, app).await?;
        return Ok(());
    }

    println!();
    println!("attaching this terminal to the same rmux session...");
    println!("open the web URL on the side; both clients are live.");

    let server = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("web server stopped: {error}");
        }
    });
    let attach_result = tokio::task::spawn_blocking(run_attach)
        .await
        .context("attach task panicked")?;
    server.abort();
    attach_result
}

async fn ws(ws: WebSocketUpgrade, State(pane): State<Pane>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, pane))
}

async fn handle_socket(socket: WebSocket, pane: Pane) {
    let (mut tx, mut rx) = socket.split();

    let output_pane = pane.clone();
    let output = tokio::spawn(async move {
        let Ok(mut stream) = output_pane
            .output_stream_starting_at(PaneOutputStart::Oldest)
            .await
        else {
            let _ = tx
                .send(Message::Text(
                    "\r\nrmux output stream failed; restart the demo.\r\n".to_owned(),
                ))
                .await;
            return;
        };

        loop {
            match stream.next().await {
                Ok(Some(PaneOutputChunk::Bytes { bytes, .. })) => {
                    if tx.send(Message::Binary(bytes)).await.is_err() {
                        break;
                    }
                }
                Ok(Some(PaneOutputChunk::Lag(notice))) => {
                    if tx.send(Message::Binary(notice.recent.bytes)).await.is_err() {
                        break;
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => break,
            }
        }
    });

    while let Some(Ok(message)) = rx.next().await {
        match message {
            Message::Text(text) => {
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    let _ = match event {
                        ClientEvent::Data { data } => pane.keyboard().type_text(data).await,
                        ClientEvent::Resize { cols, rows } => {
                            pane.resize(TerminalSizeSpec::new(cols.max(20), rows.max(6)))
                                .await
                        }
                    };
                }
            }
            Message::Binary(bytes) => {
                if let Ok(text) = String::from_utf8(bytes) {
                    let _ = pane.keyboard().type_text(text).await;
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }

    output.abort();
}

async fn ensure_web_pane() -> Result<Pane> {
    let rmux = demo_rmux_builder()?
        .default_timeout(Duration::from_secs(8))
        .connect_or_start()
        .await?;

    let workdir = demo_workdir()?;
    let command = env::var("RMUX_WEB_CMD").unwrap_or_else(|_| default_command().to_owned());
    let session_name = SessionName::new(session_name())?;
    let mut ensure = EnsureSession::named(session_name.clone())
        .create_or_reuse()
        .detached(true)
        .working_directory(workdir.to_string_lossy())
        .size(TerminalSizeSpec::new(120, 34));
    if env::consts::OS == "windows" {
        ensure = ensure.argv(windows_web_launcher_argv(&workdir, &command)?);
    } else {
        ensure = ensure.shell(format!("cd {} && {command}", sh_quote(&workdir)));
    }
    let session = rmux.ensure_session(ensure).await?;

    let pane = session.pane(0, 0);
    pane.set_title("Claude Web").await?;
    if is_claude_command(&command) {
        accept_claude_startup_prompt(&pane).await?;
    }
    Ok(pane)
}

fn default_command() -> &'static str {
    if env::consts::OS == "windows" {
        DEFAULT_WINDOWS_CMD
    } else {
        DEFAULT_UNIX_CMD
    }
}

fn is_claude_command(command: &str) -> bool {
    command
        .split_whitespace()
        .find(|token| !token.contains('='))
        == Some("claude")
}

fn windows_web_launcher_argv(workdir: &Path, command: &str) -> Result<Vec<String>> {
    let launcher = write_windows_web_launcher(workdir, command)?;
    Ok(vec![
        "powershell.exe".to_owned(),
        "-NoExit".to_owned(),
        "-NoProfile".to_owned(),
        "-ExecutionPolicy".to_owned(),
        "Bypass".to_owned(),
        "-File".to_owned(),
        launcher.to_string_lossy().into_owned(),
    ])
}

fn write_windows_web_launcher(workdir: &Path, command: &str) -> Result<PathBuf> {
    let launcher_dir = env::temp_dir().join("rmux-web-claude-demo");
    fs::create_dir_all(&launcher_dir)?;
    let launcher = launcher_dir.join("rmux-web-claude.ps1");
    let script = format!(
        "$ErrorActionPreference = 'Continue'\r\n\
         $Host.UI.RawUI.WindowTitle = 'Claude Web'\r\n\
         Set-Location -LiteralPath {}\r\n\
         $env:IS_DEMO = '1'\r\n\
         {}\r\n\
         $exitCode = if ($global:LASTEXITCODE -is [int]) {{ $global:LASTEXITCODE }} else {{ 0 }}\r\n\
         if ($exitCode -ne 0) {{\r\n\
           Write-Host \"\"\r\n\
           Write-Host ('[Claude exited with code ' + $exitCode + '; the rmux pane stays open.]')\r\n\
         }}\r\n",
        powershell_quote(&workdir.to_string_lossy()),
        command,
    );
    fs::write(&launcher, script)?;
    Ok(launcher)
}

async fn accept_claude_startup_prompt(pane: &Pane) -> Result<()> {
    let should_continue = pane
        .expect_visible_text()
        .to_match_any([
            "Do you trust",
            "Quick safety check",
            "Press Enter",
            "1. Yes",
        ])
        .timeout(Duration::from_secs(2))
        .await
        .is_ok();
    if should_continue {
        pane.keyboard().press("Enter").await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Ok(())
}

fn check() -> Result<()> {
    ensure_rmux_binary()?;

    if env::consts::OS == "windows" {
        if !command_exists("powershell.exe") {
            bail!("missing command in PATH: powershell.exe");
        }
    } else if !command_exists("bash") {
        bail!("missing command in PATH: bash");
    }

    println!("rmux binary: rmux");
    println!("rmux endpoint: {}", demo_endpoint_arg()?);
    println!(
        "{}",
        if command_exists("claude") {
            "claude is available"
        } else {
            "claude not found; set RMUX_WEB_CMD to choose another command"
        }
    );
    Ok(())
}

async fn cleanup() -> Result<()> {
    ensure_rmux_binary()?;
    let rmux = demo_rmux_builder()?
        .default_timeout(Duration::from_secs(2))
        .build();

    if let Ok(session) = rmux.session(SessionName::new(session_name())?).await {
        let _ = session.kill().await;
    }
    println!("cleaned session {}", session_name());
    Ok(())
}

fn ensure_rmux_binary() -> Result<()> {
    if command_exists("rmux") {
        Ok(())
    } else {
        bail!("missing rmux binary in PATH");
    }
}

fn attach_command() -> Result<String> {
    Ok(format!(
        "rmux -S {} attach-session -t {}",
        sh_quote_str(&demo_endpoint_arg()?),
        session_name()
    ))
}

fn run_attach() -> Result<()> {
    let status = Command::new("rmux")
        .arg("-S")
        .arg(demo_endpoint_arg()?)
        .arg("attach-session")
        .arg("-t")
        .arg(session_name())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to launch rmux attach-session")?;
    if status.success() {
        Ok(())
    } else {
        bail!("rmux attach-session exited with {status}");
    }
}

fn demo_socket_path() -> Result<PathBuf> {
    let owner = env::var("UID")
        .or_else(|_| env::var("USER"))
        .unwrap_or_else(|_| "user".to_owned());
    let dir = env::temp_dir().join(format!("rmux-web-claude-{owner}"));
    fs::create_dir_all(&dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    }

    Ok(dir.join("socket"))
}

fn demo_workdir() -> Result<PathBuf> {
    let path = env::temp_dir().join("rmux-web-claude-work");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn session_name() -> String {
    env::var("RMUX_WEB_SESSION").unwrap_or_else(|_| DEFAULT_SESSION.to_owned())
}

fn command_exists(command: &str) -> bool {
    if env::consts::OS == "windows" {
        return Command::new("where.exe")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {}", sh_quote(Path::new(command))))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn sh_quote(value: &Path) -> String {
    let value = value.to_string_lossy();
    sh_quote_str(&value)
}

fn sh_quote_str(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn demo_rmux_builder() -> Result<rmux_sdk::RmuxBuilder> {
    let builder = Rmux::builder();
    if env::consts::OS == "windows" {
        Ok(builder.windows_pipe(WINDOWS_PIPE))
    } else {
        Ok(builder.unix_socket(demo_socket_path()?))
    }
}

fn demo_endpoint_arg() -> Result<String> {
    if env::consts::OS == "windows" {
        Ok(WINDOWS_PIPE.to_owned())
    } else {
        Ok(demo_socket_path()?.to_string_lossy().into_owned())
    }
}
