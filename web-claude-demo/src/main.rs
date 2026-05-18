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
const DEFAULT_CMD: &str = "claude || exec bash";

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientEvent {
    Data { data: String },
    Resize { cols: u16, rows: u16 },
}

#[tokio::main]
async fn main() -> Result<()> {
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
    let rmux = Rmux::builder()
        .unix_socket(demo_socket_path()?)
        .default_timeout(Duration::from_secs(8))
        .connect_or_start()
        .await?;

    let workdir = demo_workdir()?;
    let command = env::var("RMUX_WEB_CMD").unwrap_or_else(|_| DEFAULT_CMD.to_owned());
    let shell = format!("cd {} && {command}", sh_quote(&workdir));
    let session_name = SessionName::new(session_name())?;
    let session = rmux
        .ensure_session(
            EnsureSession::named(session_name.clone())
                .create_or_reuse()
                .detached(true)
                .working_directory(workdir.to_string_lossy())
                .size(TerminalSizeSpec::new(120, 34))
                .shell(shell),
        )
        .await?;

    let pane = session.pane(0, 0);
    pane.set_title("Claude Web").await?;
    Ok(rmux.get_pane_by_title("Claude Web").await?)
}

fn check() -> Result<()> {
    ensure_rmux_binary()?;

    if !command_exists("bash") {
        bail!("missing command in PATH: bash");
    }

    println!("rmux binary: rmux");
    println!("rmux socket: {}", demo_socket_path()?.display());
    println!(
        "{}",
        if command_exists("claude") {
            "claude is available"
        } else {
            "claude not found; default command falls back to bash"
        }
    );
    Ok(())
}

async fn cleanup() -> Result<()> {
    ensure_rmux_binary()?;
    let rmux = Rmux::builder()
        .unix_socket(demo_socket_path()?)
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
        sh_quote(&demo_socket_path()?),
        session_name()
    ))
}

fn run_attach() -> Result<()> {
    let status = Command::new("rmux")
        .arg("-S")
        .arg(demo_socket_path()?)
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
    format!("'{}'", value.replace('\'', "'\\''"))
}
