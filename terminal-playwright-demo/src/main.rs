use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{self as ct_terminal, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use rmux_sdk::{
    CleanupPolicy, OwnedSession, Pane, Rmux, SessionName, TerminalLoadState, TerminalSizeSpec,
    TraceSession,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

const SESSION: &str = "terminal-playwright-demo";
const SDK_DAEMON_BINARY_ENV: &str = "RMUX_SDK_DAEMON_BINARY";
const WINDOWS_PIPE: &str = r"\\.\pipe\rmux-demo-terminal-playwright";
const TARGET_TITLE: &str = "playwright:target";
const RUNNER_TITLE: &str = "rmux terminal playwright runner";
const RUNNER_WINDOW_ENV: &str = "RMUX_TERMINAL_PLAYWRIGHT_RUNNER_WINDOW";
const INPUT_VALUE: &str = "rmux";
const EXPECTED_RESULT: &str = "Result: Hello rmux";
const APP_BG: Color = Color::Indexed(234);
const PANEL_BG: Color = Color::Indexed(235);
const PANEL_ALT_BG: Color = Color::Indexed(236);
const FIELD_BG: Color = Color::Indexed(238);
const TEXT: Color = Color::Indexed(254);
const MUTED: Color = Color::Indexed(245);
const DIM: Color = Color::Indexed(240);
const GREEN: Color = Color::Indexed(120);
const BLUE: Color = Color::Indexed(111);
const CYAN: Color = Color::Indexed(80);
const YELLOW: Color = Color::Indexed(222);
const RED: Color = Color::Indexed(203);
const PINK: Color = Color::Indexed(211);
const TAG_BG: Color = Color::Indexed(251);
const STEPS: &[StepSpec] = &[
    StepSpec::new(
        r#"Find "RMUX TEST APP""#,
        r#"pane.get_by_text("RMUX TEST APP").wait_for().await?;"#,
    ),
    StepSpec::new(
        r#"Find "Name" input"#,
        r#"pane.get_by_text("Name").wait_for().await?;"#,
    ),
    StepSpec::new(
        r#"Type "rmux""#,
        r#"pane.keyboard().type_text("rmux").await?;"#,
    ),
    StepSpec::new(
        r#"Click "[ Run ]""#,
        r#"pane.get_by_text("[ Run ]").click().await?;"#,
    ),
    StepSpec::new(
        "Wait for quiet",
        r#"pane.wait_for_load_state(TerminalLoadState::Quiet).await?;"#,
    ),
    StepSpec::new(
        r#"Expect "Hello rmux""#,
        r#"pane.expect_visible_text().to_contain("Result: Hello rmux").await?;"#,
    ),
];

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy)]
struct StepSpec {
    label: &'static str,
    code: &'static str,
}

impl StepSpec {
    const fn new(label: &'static str, code: &'static str) -> Self {
        Self { label, code }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StepStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

struct Step {
    spec: StepSpec,
    status: StepStatus,
    error: Option<String>,
    started_at: Option<Instant>,
    duration: Option<Duration>,
}

enum RunnerEvent {
    Running(usize),
    Passed(usize),
    Failed(usize, String),
    Detail(String),
    Capture(String),
    Done,
}

struct App {
    owned: Option<OwnedSession>,
    socket: PathBuf,
    runner_events: UnboundedReceiver<RunnerEvent>,
    runner_task: JoinHandle<()>,
    steps: Vec<Step>,
    detail: String,
    capture: String,
    done: bool,
    frame: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    force_path_rmux_binary();
    match env::args().nth(1).as_deref() {
        Some("--target-app") => run_target_app(),
        Some("check") => check_commands(),
        Some("smoke") => smoke().await,
        Some("cleanup") => cleanup().await,
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: terminal-playwright-demo [check|smoke|cleanup]");
            Ok(())
        }
        None => run_or_open_app().await,
    }
}

fn force_path_rmux_binary() {
    env::set_var(SDK_DAEMON_BINARY_ENV, "rmux");
}

async fn run_or_open_app() -> Result<()> {
    if should_open_runner_window() {
        println!("{}", open_runner_terminal()?);
        return Ok(());
    }
    run_app().await
}

fn should_open_runner_window() -> bool {
    env::var_os(RUNNER_WINDOW_ENV).is_none()
        && (macos_terminal_available() || windows_terminal_available())
}

async fn run_app() -> Result<()> {
    check_commands()?;

    let mut app = App::new().await?;
    let tui_result = run_tui(&mut app).await;
    let cleanup_result = app.cleanup().await;

    tui_result?;
    cleanup_result?;
    Ok(())
}

impl App {
    async fn new() -> Result<Self> {
        let socket = demo_socket_path()?;
        let (owned, pane, trace) = setup_demo_session(&socket).await?;
        let target_window = open_target_terminal(&socket)?;
        tokio::time::sleep(Duration::from_millis(650)).await;

        let (runner_tx, runner_events) = mpsc::unbounded_channel();
        let runner_task = tokio::spawn(run_flow(pane, trace, runner_tx));

        Ok(Self {
            owned: Some(owned),
            socket,
            runner_events,
            runner_task,
            steps: STEPS
                .iter()
                .map(|spec| Step {
                    spec: *spec,
                    status: StepStatus::Pending,
                    error: None,
                    started_at: None,
                    duration: None,
                })
                .collect(),
            detail: target_window,
            capture: String::new(),
            done: false,
            frame: 0,
        })
    }

    fn drain(&mut self) {
        while let Ok(event) = self.runner_events.try_recv() {
            match event {
                RunnerEvent::Running(index) => self.start_step(index),
                RunnerEvent::Passed(index) => self.pass_step(index),
                RunnerEvent::Failed(index, error) => {
                    self.fail_step(index, error);
                    self.done = true;
                }
                RunnerEvent::Detail(detail) => self.detail = detail,
                RunnerEvent::Capture(capture) => self.capture = capture,
                RunnerEvent::Done => {
                    self.done = true;
                }
            }
        }
        self.frame = self.frame.wrapping_add(1);
    }

    fn start_step(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Running;
            step.started_at = Some(Instant::now());
            step.duration = None;
            step.error = None;
        }
    }

    fn pass_step(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.duration = step.started_at.map(|started| started.elapsed());
            step.status = StepStatus::Passed;
        }
    }

    fn fail_step(&mut self, index: usize, error: String) {
        if let Some(step) = self.steps.get_mut(index) {
            step.duration = step.started_at.map(|started| started.elapsed());
            step.status = StepStatus::Failed;
            step.error = Some(error);
        }
    }

    fn active_step_index(&self) -> usize {
        self.steps
            .iter()
            .position(|step| step.status == StepStatus::Running)
            .or_else(|| {
                self.steps
                    .iter()
                    .rposition(|step| step.status == StepStatus::Passed)
            })
            .unwrap_or(0)
    }

    async fn cleanup(&mut self) -> Result<()> {
        self.runner_task.abort();
        if let Some(mut owned) = self.owned.take() {
            let _ = owned.cleanup().await?;
        }
        shutdown_demo_daemon(&self.socket).await?;
        Ok(())
    }
}

async fn setup_demo_session(socket: &Path) -> Result<(OwnedSession, Pane, TraceSession)> {
    let rmux = demo_rmux_builder(socket)
        .default_timeout(Duration::from_secs(5))
        .connect_or_start()
        .await?;
    let owned = rmux
        .owned_session(SessionName::new(SESSION)?)
        .replace_existing(true)
        .cleanup_policy(CleanupPolicy::KillOnDrop)
        .await?;
    let pane = owned.session().pane(0, 0);
    let (cols, rows) = terminal_size_or_default();
    pane.resize(TerminalSizeSpec::new(cols.max(120), rows.max(36)))
        .await?;

    let exe = env::current_exe()?.to_string_lossy().into_owned();
    pane.spawn([exe, "--target-app".to_owned()])
        .kill_existing(true)
        .title(TARGET_TITLE)
        .keep_alive_on_exit(true)
        .await?;

    tokio::time::sleep(Duration::from_millis(250)).await;
    let trace = rmux.tracing().max_events(256).start().await?;
    Ok((owned, pane, trace))
}

fn open_target_terminal(socket: &Path) -> Result<String> {
    let title = "rmux target under test";
    if macos_terminal_available() {
        let attach = format!(
            "exec rmux -S {} attach-session -t {}",
            shell_quote(&socket.to_string_lossy()),
            shell_quote(SESSION),
        );
        return open_macos_target_terminal(socket, title, &attach);
    }
    if windows_terminal_available() {
        return open_windows_target_terminal(socket, title);
    }

    let attach = format!(
        "exec rmux -S {} attach-session -t {}",
        shell_quote(&socket.to_string_lossy()),
        shell_quote(SESSION),
    );
    let launchers: &[(&str, &[&str])] = &[
        (
            "mate-terminal",
            &[
                "--title",
                title,
                "--geometry",
                "96x32+960+80",
                "--",
                "bash",
                "-lc",
            ],
        ),
        ("gnome-terminal", &["--title", title, "--", "bash", "-lc"]),
        ("xfce4-terminal", &["--title", title, "--command"]),
        (
            "konsole",
            &["--new-tab", "--workdir", ".", "-e", "bash", "-lc"],
        ),
        (
            "xterm",
            &[
                "-T",
                title,
                "-geometry",
                "96x32+960+80",
                "-e",
                "bash",
                "-lc",
            ],
        ),
    ];

    for (program, args) in launchers {
        if !command_exists(program) {
            continue;
        }
        let mut command = Command::new(program);
        command.args(*args);
        command.arg(&attach);
        command.spawn()?;
        return Ok(format!("target terminal opened with {program}"));
    }

    Ok(format!("open target manually: {attach}"))
}

fn macos_terminal_available() -> bool {
    env::consts::OS == "macos" && command_exists("open") && command_exists("osascript")
}

fn windows_terminal_available() -> bool {
    env::consts::OS == "windows" && command_exists("powershell.exe")
}

fn open_runner_terminal() -> Result<String> {
    if macos_terminal_available() {
        return open_macos_runner_terminal();
    }
    if windows_terminal_available() {
        return open_windows_runner_terminal();
    }
    Err("no supported terminal opener found".into())
}

fn open_macos_target_terminal(socket: &Path, title: &str, attach: &str) -> Result<String> {
    open_macos_terminal_launcher(
        socket,
        "target",
        title,
        attach,
        "96x34+960+80",
        "target terminal opened with Terminal.app",
    )
}

fn open_macos_runner_terminal() -> Result<String> {
    let socket = demo_socket_path()?;
    let exe = env::current_exe()?;
    let cwd = env::current_dir()?;
    let command = format!(
        "cd {} && exec env {}=1 {}",
        shell_quote(&cwd.to_string_lossy()),
        RUNNER_WINDOW_ENV,
        shell_quote(&exe.to_string_lossy()),
    );
    open_macos_terminal_launcher(
        &socket,
        "runner",
        RUNNER_TITLE,
        &command,
        "96x34+40+80",
        "runner terminal opened with Terminal.app",
    )
}

fn open_windows_target_terminal(socket: &Path, title: &str) -> Result<String> {
    let command = format!(
        "& rmux -S {} attach-session -t {}",
        powershell_quote(&socket.to_string_lossy()),
        powershell_quote(SESSION),
    );
    open_windows_terminal_launcher(
        title,
        &command,
        "96x34+960+80",
        "target terminal opened with Windows Terminal",
    )
}

fn open_windows_runner_terminal() -> Result<String> {
    let exe = env::current_exe()?;
    let cwd = env::current_dir()?;
    let command = format!(
        "Set-Location -LiteralPath {}; $env:{} = '1'; & {}",
        powershell_quote(&cwd.to_string_lossy()),
        RUNNER_WINDOW_ENV,
        powershell_quote(&exe.to_string_lossy()),
    );
    open_windows_terminal_launcher(
        RUNNER_TITLE,
        &command,
        "96x34+40+80",
        "runner terminal opened with Windows Terminal",
    )
}

fn open_windows_terminal_launcher(
    title: &str,
    command: &str,
    geometry: &str,
    message: &str,
) -> Result<String> {
    let (cols, rows, left, top) = parse_terminal_geometry(geometry).unwrap_or((96, 34, 80, 80));
    let launcher = write_windows_terminal_launcher(title, command)?;

    if let Some(program) = windows_terminal_program() {
        Command::new(program)
            .args([
                "-w",
                "new",
                "--pos",
                &format!("{left},{top}"),
                "--size",
                &format!("{cols},{rows}"),
                "--title",
                title,
                "powershell.exe",
                "-NoExit",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &launcher.to_string_lossy(),
            ])
            .spawn()?;
        return Ok(message.to_owned());
    }

    open_windows_powershell_window(title, &launcher, geometry)?;
    Ok(format!("{message} via PowerShell"))
}

fn write_windows_terminal_launcher(title: &str, command: &str) -> Result<PathBuf> {
    let launcher_dir = env::temp_dir().join("rmux-terminal-playwright-demo");
    fs::create_dir_all(&launcher_dir)?;
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    let slug = if slug.is_empty() {
        "terminal".to_owned()
    } else {
        slug
    };
    let launcher = launcher_dir.join(format!("rmux-terminal-playwright-{slug}.ps1"));
    let script = format!(
        "$ErrorActionPreference = 'Stop'\r\n$Host.UI.RawUI.WindowTitle = {}\r\n{}\r\n",
        powershell_quote(title),
        command,
    );
    fs::write(&launcher, script)?;
    Ok(launcher)
}

fn windows_terminal_program() -> Option<&'static str> {
    if command_exists("wt.exe") {
        Some("wt.exe")
    } else if command_exists("wt") {
        Some("wt")
    } else {
        None
    }
}

fn open_windows_powershell_window(_title: &str, launcher: &Path, geometry: &str) -> Result<()> {
    let (cols, rows, left, top) = parse_terminal_geometry(geometry).unwrap_or((96, 34, 80, 80));
    let width = i32::from(cols) * 8 + 36;
    let height = i32::from(rows) * 17 + 70;
    let script = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class RmuxDemoWindow {
  [DllImport("user32.dll")]
  public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int nWidth, int nHeight, bool bRepaint);
}
"@
$arguments = @("-NoExit", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $env:RMUX_DEMO_WIN_LAUNCHER)
$process = Start-Process -FilePath "powershell.exe" -ArgumentList $arguments -PassThru
for ($i = 0; $i -lt 40 -and $process.MainWindowHandle -eq 0; $i++) {
  Start-Sleep -Milliseconds 100
  $process.Refresh()
}
if ($process.MainWindowHandle -ne 0) {
  [RmuxDemoWindow]::MoveWindow($process.MainWindowHandle, [int]$env:RMUX_DEMO_WIN_LEFT, [int]$env:RMUX_DEMO_WIN_TOP, [int]$env:RMUX_DEMO_WIN_WIDTH, [int]$env:RMUX_DEMO_WIN_HEIGHT, $true) | Out-Null
}
"#;
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .env("RMUX_DEMO_WIN_LAUNCHER", launcher)
        .env("RMUX_DEMO_WIN_LEFT", left.to_string())
        .env("RMUX_DEMO_WIN_TOP", top.to_string())
        .env("RMUX_DEMO_WIN_WIDTH", width.to_string())
        .env("RMUX_DEMO_WIN_HEIGHT", height.to_string())
        .spawn()?;
    Ok(())
}

fn open_macos_terminal_launcher(
    socket: &Path,
    slug: &str,
    title: &str,
    command: &str,
    geometry: &str,
    message: &str,
) -> Result<String> {
    let launcher_dir = socket
        .parent()
        .ok_or("demo socket path has no parent directory")?;
    fs::create_dir_all(launcher_dir)?;
    let launcher = launcher_dir.join(format!("rmux-terminal-playwright-{slug}.command"));
    let script = format!(
        "#!/usr/bin/env bash\nprintf '\\033]0;%s\\007' {}\n{}\n",
        shell_quote(title),
        command,
    );
    fs::write(&launcher, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o700))?;
    }

    Command::new("open")
        .args(["-n", "-a", "Terminal"])
        .arg(&launcher)
        .status()?;
    thread::sleep(Duration::from_millis(800));
    set_macos_front_window_bounds(geometry)?;
    Ok(message.to_owned())
}

fn set_macos_front_window_bounds(geometry: &str) -> Result<()> {
    let (left, top, right, bottom) = macos_terminal_bounds(geometry);
    let script = format!(
        "tell application \"Terminal\" to set bounds of front window to {{{left}, {top}, {right}, {bottom}}}"
    );
    let status = Command::new("osascript").arg("-e").arg(script).status()?;
    if status.success() {
        Ok(())
    } else {
        Err("failed to position Terminal.app window".into())
    }
}

fn macos_terminal_bounds(geometry: &str) -> (i32, i32, i32, i32) {
    let (cols, rows, left, top) = parse_terminal_geometry(geometry).unwrap_or((96, 34, 80, 80));
    let width = i32::from(cols) * 8 + 36;
    let height = i32::from(rows) * 17 + 70;
    (left, top, left + width, top + height)
}

fn parse_terminal_geometry(geometry: &str) -> Option<(u16, u16, i32, i32)> {
    let mut parts = geometry.split('+');
    let size = parts.next()?;
    let left = parts.next()?.parse().ok()?;
    let top = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let (cols, rows) = size.split_once('x')?;
    Some((cols.parse().ok()?, rows.parse().ok()?, left, top))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

async fn run_flow(pane: Pane, trace: TraceSession, tx: UnboundedSender<RunnerEvent>) {
    if run_flow_inner(&pane, &trace, &tx).await.is_ok() {
        let _ = tx.send(RunnerEvent::Done);
    }
}

async fn run_flow_inner(
    pane: &Pane,
    trace: &TraceSession,
    tx: &UnboundedSender<RunnerEvent>,
) -> Result<()> {
    run_step(tx, trace, 0, || {
        pane.get_by_text("RMUX TEST APP")
            .wait_for()
            .timeout(Duration::from_secs(5))
    })
    .await?;
    run_step(tx, trace, 1, || {
        pane.get_by_text("Name")
            .wait_for()
            .timeout(Duration::from_secs(5))
    })
    .await?;

    mark_running(tx, trace, 2)?;
    demo_delay(180).await;
    pane.keyboard().type_text(INPUT_VALUE).await?;
    trace.record_input(pane, INPUT_VALUE)?;
    pane.get_by_text(INPUT_VALUE)
        .wait_for()
        .timeout(Duration::from_secs(5))
        .await?;
    mark_passed(tx, 2).await;

    mark_running(tx, trace, 3)?;
    demo_delay(180).await;
    pane.get_by_text("[ Run ]").click().await?;
    if cfg!(windows) {
        // ConPTY can drop terminal mouse reports in this demo path; Enter keeps the flow deterministic.
        pane.keyboard().press("Enter").await?;
    }
    trace.record_action("locator.click([ Run ])")?;
    mark_passed(tx, 3).await;

    run_step(tx, trace, 4, || {
        pane.wait_for_load_state(TerminalLoadState::Quiet)
            .stable_for(Duration::from_millis(500))
            .timeout(Duration::from_secs(8))
    })
    .await?;

    mark_running(tx, trace, 5)?;
    demo_delay(180).await;
    pane.expect_visible_text()
        .to_contain(EXPECTED_RESULT)
        .timeout(Duration::from_secs(5))
        .await?;
    mark_passed(tx, 5).await;

    let capture = pane
        .get_by_text(EXPECTED_RESULT)
        .capture()
        .preserve_style(true)
        .await?;
    let _ = tx.send(RunnerEvent::Capture(capture.text));

    trace.record_snapshot(pane).await?;
    let trace_dir = env::temp_dir().join("rmux-terminal-playwright-demo-trace");
    let trace_path = trace.clone().stop(&trace_dir).await?;
    let _ = tx.send(RunnerEvent::Detail(format!(
        "trace written to {}",
        trace_path.display()
    )));
    Ok(())
}

async fn run_step<F, W>(
    tx: &UnboundedSender<RunnerEvent>,
    trace: &TraceSession,
    index: usize,
    wait: F,
) -> Result<()>
where
    F: FnOnce() -> W,
    W: std::future::IntoFuture<Output = rmux_sdk::Result<rmux_sdk::PaneSnapshot>>,
{
    mark_running(tx, trace, index)?;
    demo_delay(180).await;
    match wait().into_future().await {
        Ok(_) => {
            mark_passed(tx, index).await;
            Ok(())
        }
        Err(error) => {
            let message = error.to_string();
            let _ = tx.send(RunnerEvent::Failed(index, message.clone()));
            Err(message.into())
        }
    }
}

fn mark_running(
    tx: &UnboundedSender<RunnerEvent>,
    trace: &TraceSession,
    index: usize,
) -> Result<()> {
    trace.record_action(STEPS[index].code)?;
    let _ = tx.send(RunnerEvent::Detail(STEPS[index].code.to_owned()));
    let _ = tx.send(RunnerEvent::Running(index));
    Ok(())
}

async fn mark_passed(tx: &UnboundedSender<RunnerEvent>, index: usize) {
    let _ = tx.send(RunnerEvent::Passed(index));
    demo_delay(340).await;
}

async fn demo_delay(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

async fn run_tui(app: &mut App) -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        app.drain();
        terminal.draw(|frame| draw(frame, app))?;

        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    fill(frame, area, APP_BG);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(14),
            Constraint::Length(6),
            Constraint::Length(2),
        ])
        .split(area);
    draw_header(frame, vertical[0], app);
    draw_steps(frame, vertical[1], app);
    draw_detail(frame, vertical[2], app);
    draw_footer(frame, vertical[3], app);
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let passed = app
        .steps
        .iter()
        .filter(|step| step.status == StepStatus::Passed)
        .count();
    let status = if app.done {
        "🚀 PASS"
    } else {
        "⏳ LIVE TEST"
    };
    let bar = progress_bar(passed, app.steps.len(), 24);
    let lines = vec![
        Line::from(vec![
            badge(" Playwright for Terminals ", GREEN),
            Span::raw("  "),
            Span::styled(
                status,
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {passed}/{}", app.steps.len()),
                Style::default().fg(TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled(bar, Style::default().fg(GREEN)),
            Span::raw("  "),
            Span::styled(
                "testing the app in the second terminal",
                Style::default().fg(MUTED),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(APP_BG))
            .alignment(Alignment::Left),
        area,
    );
}

fn draw_steps(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let inner = card(
        frame,
        area.inner(Margin {
            horizontal: 0,
            vertical: 1,
        }),
        " test runner ",
        BLUE,
        PANEL_ALT_BG,
    );
    let lines = app
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let is_active = step.status == StepStatus::Running;
            let (marker, status, color) = match step.status {
                StepStatus::Pending => ("○", "queued".to_owned(), DIM),
                StepStatus::Running => (
                    spinner(app.frame),
                    format_duration(step.started_at.map(|t| t.elapsed())),
                    YELLOW,
                ),
                StepStatus::Passed => ("✅", format_duration(step.duration), GREEN),
                StepStatus::Failed => (
                    "❌",
                    format!("failed {}", format_duration(step.duration)),
                    PINK,
                ),
            };
            let label_style = if is_active {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            Line::from(vec![
                Span::styled(format!(" {:02} ", index + 1), Style::default().fg(DIM)),
                Span::styled(format!("{marker}  "), Style::default().fg(color)),
                Span::styled(step.spec.label, label_style),
                Span::raw("  "),
                Span::styled(status, Style::default().fg(color)),
            ])
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(PANEL_ALT_BG))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_detail(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let inner = card(frame, area, " assertion ", YELLOW, PANEL_BG);
    let step = &app.steps[app.active_step_index()];
    let lines = if app.done {
        vec![
            Line::from(vec![
                Span::styled(
                    "🚀  ",
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    EXPECTED_RESULT,
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("capture  ", Style::default().fg(BLUE)),
                Span::styled(
                    if app.capture.is_empty() {
                        app.detail.as_str()
                    } else {
                        app.capture.as_str()
                    },
                    Style::default().fg(MUTED),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("sdk  ", Style::default().fg(YELLOW)),
                Span::styled(step.spec.code, Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("expect  ", Style::default().fg(GREEN)),
                Span::styled(
                    EXPECTED_RESULT,
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
            ]),
        ]
    };
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(PANEL_BG))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let state = if app.done {
        "🚀 all green - press q to quit"
    } else {
        "⏳ tests running"
    };
    let line = Line::from(vec![
        Span::styled(" q ", Style::default().fg(Color::Black).bg(TAG_BG)),
        Span::raw(" quit   "),
        Span::styled(state, Style::default().fg(GREEN)),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(APP_BG)),
        area,
    );
}

fn badge(text: &'static str, color: Color) -> Span<'static> {
    Span::styled(
        text,
        Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

fn fill(frame: &mut Frame<'_>, area: Rect, color: Color) {
    frame.render_widget(Paragraph::new("").style(Style::default().bg(color)), area);
}

fn card(frame: &mut Frame<'_>, area: Rect, title: &'static str, border: Color, bg: Color) -> Rect {
    fill(frame, area, bg);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border).bg(bg))
        .title(Span::styled(
            title,
            Style::default()
                .fg(border)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(bg));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    frame.render_widget(block, area);
    fill(frame, inner, bg);
    inner
}

fn progress_bar(done: usize, total: usize, width: usize) -> String {
    let filled = width.saturating_mul(done) / total.max(1);
    format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}

fn spinner(frame: u64) -> &'static str {
    ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"][(frame as usize / 2) % 10]
}

fn format_duration(duration: Option<Duration>) -> String {
    duration
        .map(|duration| format!("{:.2}s", duration.as_secs_f32()))
        .unwrap_or_else(|| "0.00s".to_owned())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        ct_terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
        let _ = ct_terminal::disable_raw_mode();
    }
}

#[derive(Default)]
struct TargetApp {
    name: String,
    running: bool,
    started: Option<Instant>,
    phase: usize,
    frame: u64,
    result: String,
    dirty: bool,
}

fn run_target_app() -> Result<()> {
    let mut stdout = io::stdout();
    ct_terminal::enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_target_loop(&mut terminal);
    let _ = execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        cursor::Show
    );
    let _ = ct_terminal::disable_raw_mode();
    result
}

fn run_target_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let mut app = TargetApp {
        dirty: true,
        ..TargetApp::default()
    };
    loop {
        app.tick();
        if app.dirty {
            terminal.draw(|frame| draw_target_app(frame, &app))?;
            app.dirty = false;
        }
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('q') if !app.running && !app.name.is_empty() => break,
                    KeyCode::Char(ch) if !app.running => {
                        app.name.push(ch);
                        app.dirty = true;
                    }
                    KeyCode::Backspace if !app.running => {
                        app.name.pop();
                        app.dirty = true;
                    }
                    KeyCode::Enter if !app.running => app.start(),
                    _ => {}
                },
                Event::Mouse(mouse) if !app.running => {
                    let (cols, rows) = ct_terminal::size().unwrap_or((120, 36));
                    let layout = target_layout(Rect::new(0, 0, cols, rows));
                    if matches!(mouse.kind, MouseEventKind::Down(_))
                        && contains(layout.button, mouse.column, mouse.row)
                    {
                        app.start();
                    }
                }
                Event::Resize(_, _) => app.dirty = true,
                _ => {}
            }
        }
    }
    Ok(())
}

impl TargetApp {
    fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        self.started = Some(Instant::now());
        self.phase = 0;
        self.result.clear();
        self.dirty = true;
    }

    fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.running {
            self.dirty = true;
        }

        let Some(started) = self.started else {
            return;
        };
        let elapsed = started.elapsed();
        if self.phase == 0 && elapsed >= Duration::from_millis(350) {
            self.phase = 1;
            self.dirty = true;
        }
        if self.phase == 1 && elapsed >= Duration::from_millis(750) {
            self.phase = 2;
            self.dirty = true;
        }
        if self.phase == 2 && elapsed >= Duration::from_millis(1150) {
            let name = if self.name.trim().is_empty() {
                "terminal"
            } else {
                self.name.trim()
            };
            self.result = format!("Result: Hello {name}");
            self.phase = 3;
            self.running = false;
            self.dirty = true;
        }
    }
}

struct TargetLayout {
    browser: Rect,
    hero: Rect,
    form: Rect,
    input: Rect,
    button: Rect,
    status: Rect,
    result: Rect,
}

fn target_layout(area: Rect) -> TargetLayout {
    let content = area.inner(Margin {
        horizontal: 4,
        vertical: 1,
    });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(content);
    let form_inner = rows[2].inner(Margin {
        horizontal: 3,
        vertical: 1,
    });
    let form_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(form_inner);
    let button_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(18),
            Constraint::Min(0),
        ])
        .split(form_rows[2]);
    TargetLayout {
        browser: rows[0],
        hero: rows[1],
        form: rows[2],
        input: form_rows[1],
        button: button_cols[1],
        status: form_rows[0],
        result: rows[3],
    }
}

fn draw_target_app(frame: &mut Frame<'_>, app: &TargetApp) {
    let area = frame.area();
    fill(frame, area, APP_BG);

    let layout = target_layout(area);
    draw_target_browser(frame, layout.browser);
    draw_target_hero(frame, layout.hero, app);
    draw_target_form(
        frame,
        layout.form,
        layout.input,
        layout.button,
        layout.status,
        app,
    );
    draw_target_result(frame, layout.result, app);
}

fn draw_target_browser(frame: &mut Frame<'_>, area: Rect) {
    let inner = card(frame, area, " browser ", CYAN, PANEL_BG);
    let lines = vec![Line::from(vec![
        Span::styled("● ● ●", Style::default().fg(RED)),
        Span::raw("  "),
        Span::styled("https://demo.rmux.io/signup", Style::default().fg(MUTED)),
    ])];
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(PANEL_BG)),
        inner,
    );
}

fn draw_target_hero(frame: &mut Frame<'_>, area: Rect, app: &TargetApp) {
    let inner = card(frame, area, " landing page ", GREEN, PANEL_ALT_BG);
    let headline = if app.result.is_empty() {
        "RMUX TEST APP"
    } else {
        "RMUX TEST APP  ✅"
    };
    let lines = vec![
        Line::from(Span::styled(
            headline,
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "A simulated web page rendered inside a real terminal pane.",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::styled("tested by ", Style::default().fg(MUTED)),
            Span::styled(
                "rmux locators + keyboard + click",
                Style::default().fg(GREEN),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().bg(PANEL_ALT_BG)),
        inner,
    );
}

fn draw_target_form(
    frame: &mut Frame<'_>,
    area: Rect,
    input: Rect,
    button: Rect,
    status: Rect,
    app: &TargetApp,
) {
    let inner = card(frame, area, " signup form ", BLUE, PANEL_BG);
    fill(frame, inner, PANEL_BG);

    let status_text = if app.running {
        format!("{} running browser-like flow", spinner(app.frame))
    } else if app.result.is_empty() {
        "waiting for test runner".to_owned()
    } else {
        "✅ interaction complete".to_owned()
    };
    frame.render_widget(
        Paragraph::new(status_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL_BG)),
        status,
    );

    let cursor = if app.running { "" } else { "█" };
    let value = if app.name.is_empty() {
        cursor.to_owned()
    } else {
        format!("{}{}", app.name, cursor)
    };
    frame.render_widget(
        Paragraph::new(format!("Name   {value}"))
            .style(Style::default().fg(TEXT).bg(FIELD_BG))
            .alignment(Alignment::Center),
        input,
    );

    let button_style = if app.running {
        Style::default().fg(MUTED).bg(FIELD_BG)
    } else {
        Style::default()
            .fg(Color::Black)
            .bg(GREEN)
            .add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new("[ Run ]")
            .alignment(Alignment::Center)
            .style(button_style),
        button,
    );
}

fn draw_target_result(frame: &mut Frame<'_>, area: Rect, app: &TargetApp) {
    let inner = card(frame, area, " response ", GREEN, PANEL_ALT_BG);
    let text = if app.result.is_empty() {
        "Result appears here"
    } else {
        app.result.as_str()
    };
    let style = if app.result.is_empty() {
        Style::default().fg(MUTED).bg(PANEL_ALT_BG)
    } else {
        Style::default()
            .fg(GREEN)
            .bg(PANEL_ALT_BG)
            .add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new(if app.result.is_empty() {
            text.to_owned()
        } else {
            format!("🚀  {text}")
        })
        .alignment(Alignment::Center)
        .style(style),
        inner,
    );
}

fn contains(area: Rect, col: u16, row: u16) -> bool {
    col >= area.x
        && col < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

async fn cleanup() -> Result<()> {
    let socket = demo_socket_path()?;
    remove_macos_target_launcher(&socket);
    let rmux = match demo_rmux_builder(&socket)
        .default_timeout(Duration::from_secs(2))
        .connect()
        .await
    {
        Ok(rmux) => rmux,
        Err(_) => {
            println!("cleaned {SESSION}");
            return Ok(());
        }
    };
    if let Ok(session) = rmux.session(SessionName::new(SESSION)?).await {
        let _ = session.kill().await?;
    }
    let _ = rmux.shutdown().await;
    println!("cleaned {SESSION}");
    Ok(())
}

fn remove_macos_target_launcher(socket: &Path) {
    remove_macos_launchers(socket);
}

fn remove_macos_launchers(socket: &Path) {
    if let Some(parent) = socket.parent() {
        let _ = fs::remove_file(parent.join("rmux-terminal-playwright-target.command"));
        let _ = fs::remove_file(parent.join("rmux-terminal-playwright-runner.command"));
    }
}

async fn smoke() -> Result<()> {
    check_commands()?;
    let socket = demo_socket_path()?;
    let (mut owned, pane, trace) = setup_demo_session(&socket).await?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    run_flow_inner(&pane, &trace, &tx).await?;
    while rx.try_recv().is_ok() {}
    let _ = owned.cleanup().await?;
    shutdown_demo_daemon(&socket).await?;
    println!("terminal playwright smoke passed");
    Ok(())
}

async fn shutdown_demo_daemon(socket: &Path) -> Result<()> {
    remove_macos_target_launcher(socket);
    let rmux = match demo_rmux_builder(socket)
        .default_timeout(Duration::from_secs(2))
        .connect()
        .await
    {
        Ok(rmux) => rmux,
        Err(_) => return Ok(()),
    };
    let _ = rmux.shutdown().await;
    Ok(())
}

fn check_commands() -> Result<()> {
    let mut missing = Vec::new();
    if !command_exists("rmux") {
        missing.push("rmux");
    }
    if missing.is_empty() {
        println!("rmux is available");
        Ok(())
    } else {
        Err(format!("missing commands: {}", missing.join(", ")).into())
    }
}

fn command_exists(command: &str) -> bool {
    if env::consts::OS == "windows" {
        return Command::new("where.exe")
            .arg(command)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn demo_socket_path() -> Result<PathBuf> {
    if env::consts::OS == "windows" {
        return Ok(PathBuf::from(WINDOWS_PIPE));
    }

    let owner = env::var("UID")
        .or_else(|_| env::var("USER"))
        .unwrap_or_else(|_| "user".to_owned());
    let socket_dir = env::temp_dir().join(format!("rmux-{owner}"));
    fs::create_dir_all(&socket_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&socket_dir, fs::Permissions::from_mode(0o700))?;
    }

    Ok(socket_dir.join(SESSION))
}

fn demo_rmux_builder(endpoint: &Path) -> rmux_sdk::RmuxBuilder {
    let builder = Rmux::builder();
    if env::consts::OS == "windows" {
        builder.windows_pipe(endpoint.to_string_lossy().into_owned())
    } else {
        builder.unix_socket(endpoint)
    }
}

fn terminal_size_or_default() -> (u16, u16) {
    ct_terminal::size().unwrap_or((160, 48))
}
