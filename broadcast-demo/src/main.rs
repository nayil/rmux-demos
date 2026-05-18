use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs};

use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use ratatui_rmux::{cell_style, glyph_symbol, PaneDriver};
use rmux_sdk::{
    EnsureSession, Input, Pane, PaneCell, PaneColor, PaneOutputChunk, PaneSet, PaneSnapshot,
    RenderUpdate, Rmux, Session, SessionName, SplitDirection, TerminalSizeSpec,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::task::JoinHandle;

const SESSION: &str = "broadcast-demo";
const SDK_DAEMON_BINARY_ENV: &str = "RMUX_SDK_DAEMON_BINARY";
const WINDOWS_PIPE: &str = r"\\.\pipe\rmux-demo-broadcast";
const SPINNER: &[&str] = &["|", "/", "-", "\\"];
const AGENT_REPLY_DIRECTIVE: &str = "Reply directly in one short sentence. Do not use tools.";
const LEADING_NOISE_CHARS: &str = "●•✓◆✦■✻✽✶✢⠋⠙⠹⠸⠼⠴⠦⠧⠇⠃┃│>❯›-\\/|";
const UI_FRAME_CHARS: &str = "│─╭╮╰╯┌┐└┘╎╏┃";
const STATUS_PREFIXES: &str = "[⇣◉⠋⠙⠹⠸⠼⠴⠦⠧⠇⠃";
const VIBE_EXCLUDED_CHARS: &str = "✓◆▶⎣│─╭╮╰╯┌┐└┘┃";
const BOX_EDGE_CHARS: &str = "│┃║|╎╏";

const AGENTS: &[Agent] = &[
    agent(
        "Claude",
        &[
            "claude",
            "--dangerously-skip-permissions",
            "--permission-mode",
            "bypassPermissions",
        ],
        &["Enter"],
        Color::Indexed(209),
    ),
    agent(
        "Codex",
        &["codex", "--dangerously-bypass-approvals-and-sandbox"],
        &["Enter"],
        Color::Indexed(75),
    ),
    agent(
        "Gemini",
        &["gemini", "--skip-trust", "--approval-mode", "yolo"],
        &[],
        Color::Indexed(111),
    ),
    agent("Vibe", &["vibe", "--trust"], &[], Color::Indexed(120)),
    agent(
        "Grok",
        &["grok", "--always-approve"],
        &[],
        Color::Indexed(222),
    ),
];

#[derive(Clone, Copy)]
struct Agent {
    title: &'static str,
    command: &'static [&'static str],
    startup_keys: &'static [&'static str],
    accent: Color,
}

const fn agent(
    title: &'static str,
    command: &'static [&'static str],
    startup_keys: &'static [&'static str],
    accent: Color,
) -> Agent {
    Agent {
        title,
        command,
        startup_keys,
        accent,
    }
}

struct AgentPane {
    agent: Agent,
    driver: PaneDriver,
    logo: Option<CapturedLogo>,
    output_baseline: Option<Vec<String>>,
    output_lines: Vec<String>,
}

struct CapturedLogo {
    rows: Vec<Vec<LogoCell>>,
    score: i32,
}

struct LogoCell {
    symbol: String,
    style: Style,
    blank: bool,
}

enum AgentUpdate {
    Snapshot(RenderUpdate),
    OutputLine(String),
}

type RenderMessage = (usize, AgentUpdate);

struct App {
    rmux: Option<Rmux>,
    session: Option<Session>,
    panes: Vec<AgentPane>,
    render_updates: UnboundedReceiver<RenderMessage>,
    render_tasks: Vec<JoinHandle<()>>,
    prompt: String,
    last_prompt: Option<String>,
    active_targets: Vec<usize>,
    sent_frame: Option<u64>,
    focused_agent: Option<usize>,
    status: String,
    frame: u64,
    last_logo_refresh: Instant,
    last_output_refresh: Instant,
    last_prompt_dismissal: Instant,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    force_path_rmux_binary();
    match env::args().nth(1).as_deref() {
        Some("check") => check_commands(),
        Some("cleanup") => cleanup().await,
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: broadcast-demo [check|cleanup]");
            Ok(())
        }
        None => run_app().await,
    }
}

fn force_path_rmux_binary() {
    env::set_var(SDK_DAEMON_BINARY_ENV, "rmux");
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
        let rmux = demo_rmux_builder()?
            .default_timeout(Duration::from_secs(3))
            .connect_or_start()
            .await?;
        let session_name = SessionName::new(SESSION)?;
        if let Ok(existing) = rmux.session(session_name.clone()).await {
            let _ = existing.kill().await?;
        }

        let (cols, rows) = terminal_size_or_default();
        let work_dir = demo_work_dir()?;
        let session = rmux
            .ensure_session(
                EnsureSession::named(session_name.clone())
                    .create_only()
                    .detached(true)
                    .working_directory(work_dir.to_string_lossy().into_owned())
                    .size(TerminalSizeSpec::new(cols.max(240), rows.max(120))),
            )
            .await?;
        let root = session.pane(0, 0);
        root.resize(TerminalSizeSpec::new(cols.max(240), rows.max(120)))
            .await?;
        let agents = selected_agents()?;
        let root = stable_pane(&session, &root, agents[0]).await?;

        let second = split_agent(&root, SplitDirection::Right, agents[1], &work_dir).await?;
        let second = stable_pane(&session, &second, agents[1]).await?;
        let third = split_agent(&second, SplitDirection::Right, agents[2], &work_dir).await?;
        let third = stable_pane(&session, &third, agents[2]).await?;
        let fourth = split_agent(&root, SplitDirection::Down, agents[3], &work_dir).await?;
        let fourth = stable_pane(&session, &fourth, agents[3]).await?;
        let fifth = split_agent(&second, SplitDirection::Down, agents[4], &work_dir).await?;
        let fifth = stable_pane(&session, &fifth, agents[4]).await?;
        launch_agent(&root, agents[0], &work_dir).await?;

        let panes = vec![
            (agents[0], root.clone()),
            (agents[1], second),
            (agents[2], third),
            (agents[3], fourth),
            (agents[4], fifth),
        ];

        for (agent, pane) in &panes {
            send_startup_keys(pane, *agent).await?;
        }

        tokio::time::sleep(Duration::from_millis(900)).await;

        let (render_tx, render_updates) = mpsc::unbounded_channel();
        let mut render_tasks = Vec::with_capacity(panes.len());
        let mut agent_panes = Vec::with_capacity(panes.len());
        for (index, (agent, pane)) in panes.into_iter().enumerate() {
            let mut driver = PaneDriver::new(pane.clone());
            let _ = driver.refresh().await;
            let logo = extract_logo(agent, &driver.state().snapshot);
            render_tasks.push(spawn_render_task(index, pane.clone(), render_tx.clone()));
            render_tasks.push(spawn_line_task(index, pane, render_tx.clone()));
            agent_panes.push(AgentPane {
                agent,
                driver,
                logo,
                output_baseline: None,
                output_lines: Vec::new(),
            });
        }
        drop(render_tx);

        Ok(Self {
            rmux: Some(rmux),
            session: Some(session),
            panes: agent_panes,
            render_updates,
            render_tasks,
            prompt: String::new(),
            last_prompt: None,
            active_targets: Vec::new(),
            sent_frame: None,
            focused_agent: None,
            status: "ready: type locally, Enter broadcasts once, Esc/Ctrl-C quits".to_owned(),
            frame: 0,
            last_logo_refresh: Instant::now(),
            last_output_refresh: Instant::now(),
            last_prompt_dismissal: Instant::now(),
        })
    }

    async fn cleanup(&mut self) -> Result<()> {
        for task in &self.render_tasks {
            task.abort();
        }
        if let Some(session) = self.session.take() {
            let _ = session.kill().await;
        }
        if let Some(rmux) = self.rmux.take() {
            let _ = rmux.shutdown().await;
        }
        Ok(())
    }

    fn drain_render_updates(&mut self) {
        while let Ok((index, update)) = self.render_updates.try_recv() {
            let Some(pane) = self.panes.get_mut(index) else {
                continue;
            };
            match update {
                AgentUpdate::Snapshot(update) => {
                    pane.driver.apply_snapshot(update.into_snapshot());
                    if let Some(prompt) = self.last_prompt.as_deref() {
                        if self.active_targets.contains(&index) {
                            remember_visible_output(pane, prompt);
                        }
                    } else {
                        update_best_logo(pane);
                    }
                }
                AgentUpdate::OutputLine(line) => {
                    if let Some(prompt) = self.last_prompt.as_deref() {
                        if self.active_targets.contains(&index) {
                            remember_output_text(pane, prompt, &line);
                        }
                    }
                }
            }
        }
    }

    async fn refresh_startup_logos(&mut self) {
        if self.last_prompt.is_some()
            || self.panes.iter().all(|pane| pane.logo.is_some())
            || self.last_logo_refresh.elapsed() < Duration::from_millis(250)
        {
            return;
        }

        self.last_logo_refresh = Instant::now();
        for pane in &mut self.panes {
            let Ok(snapshot) = pane.driver.pane().snapshot().await else {
                continue;
            };
            pane.driver.apply_snapshot(snapshot);
            update_best_logo(pane);
        }
    }

    async fn refresh_active_outputs(&mut self) {
        let Some(prompt) = self.last_prompt.clone() else {
            return;
        };
        if self.last_output_refresh.elapsed() < Duration::from_millis(250) {
            return;
        }

        self.last_output_refresh = Instant::now();
        for index in self.active_targets.clone() {
            let Some(pane) = self.panes.get_mut(index) else {
                continue;
            };
            let Ok(snapshot) = pane.driver.pane().snapshot().await else {
                continue;
            };
            pane.driver.apply_snapshot(snapshot);
            remember_visible_output(pane, &prompt);
        }
    }

    async fn dismiss_blocking_agent_prompts(&mut self) {
        if self.last_prompt.is_none()
            || self.last_prompt_dismissal.elapsed() < Duration::from_millis(250)
        {
            return;
        }
        self.last_prompt_dismissal = Instant::now();

        for pane in &self.panes {
            if !has_blocking_permission_prompt(&pane.driver.state().snapshot) {
                continue;
            }
            let _ = pane.driver.pane().keyboard().press("Enter").await;
        }
    }

    async fn send_prompt_to_targets(&mut self, targets: &[usize], prompt: &str) -> bool {
        self.capture_output_baselines().await;
        let agent_panes = self.target_agent_panes(targets);
        if agent_panes.is_empty() {
            self.status = "broadcast error: no target pane selected".to_owned();
            return false;
        }

        self.last_prompt = Some(prompt.to_owned());
        self.active_targets = targets.to_vec();
        self.sent_frame = Some(self.frame);
        self.prompt.clear();
        self.status = match self.focused_agent_title() {
            Some(title) => format!("sent to {title}"),
            None => "sent to all agents".to_owned(),
        };

        let prompt = prompt.to_owned();
        self.render_tasks.push(tokio::spawn(async move {
            for (agent, pane) in &agent_panes {
                let _ = send_prompt_to_agent(*agent, pane, &prompt).await;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }));
        true
    }

    async fn capture_output_baselines(&mut self) {
        for pane in &mut self.panes {
            if let Ok(snapshot) = pane.driver.pane().snapshot().await {
                pane.driver.apply_snapshot(snapshot);
            }
            pane.output_baseline = Some(snapshot_rows(&pane.driver.state().snapshot));
            pane.output_lines.clear();
        }
    }

    fn target_agent_panes(&self, targets: &[usize]) -> Vec<(Agent, Pane)> {
        targets
            .iter()
            .filter_map(|index| self.panes.get(*index))
            .map(|pane| (pane.agent, pane.driver.pane().clone()))
            .collect()
    }

    fn prompt_targets(&self) -> Vec<usize> {
        self.focused_agent
            .map(|index| vec![index])
            .unwrap_or_else(|| (0..self.panes.len()).collect())
    }

    fn focused_agent_title(&self) -> Option<&'static str> {
        self.focused_agent
            .and_then(|index| self.panes.get(index))
            .map(|pane| pane.agent.title)
    }

    fn target_label(&self, targets: &[usize]) -> String {
        match targets {
            [index] => self
                .panes
                .get(*index)
                .map(|pane| pane.agent.title.to_owned())
                .unwrap_or_else(|| "selected pane".to_owned()),
            _ => "all agents".to_owned(),
        }
    }

    fn staged_status(&self) -> String {
        match self.focused_agent_title() {
            Some(title) => format!("staged for {title}; Enter sends only there"),
            None => "staged locally; Enter broadcasts to all agents".to_owned(),
        }
    }

    fn set_focus(&mut self, target: Option<usize>) {
        self.focused_agent = target;
        self.status = match self.focused_agent_title() {
            Some(title) => format!("target: {title}; Enter sends only there"),
            None => "broadcast mode: Enter sends to all agents".to_owned(),
        };
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        let root = app_layout(area);
        if let Some(target) = hit_test_agent(mouse.column, mouse.row, root[1]) {
            self.set_focus(Some(target));
        } else if rect_contains(root[2], mouse.column, mouse.row) {
            self.set_focus(None);
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }

        match key.code {
            KeyCode::Esc => true,
            KeyCode::Char(char) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    self.prompt.push(char);
                    self.status = self.staged_status();
                }
                false
            }
            KeyCode::Backspace => {
                if self.prompt.pop().is_some() {
                    self.status = self.staged_status();
                }
                false
            }
            KeyCode::Enter => {
                let prompt = self.prompt.trim().to_owned();
                if !prompt.is_empty() {
                    let targets = self.prompt_targets();
                    self.send_prompt_to_targets(&targets, &prompt).await;
                } else {
                    self.prompt.clear();
                }
                false
            }
            _ => false,
        }
    }
}

async fn run_tui(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let _guard = TerminalRestore;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide
    )?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        app.drain_render_updates();
        app.refresh_startup_logos().await;
        app.refresh_active_outputs().await;
        app.dismiss_blocking_agent_prompts().await;
        app.frame = app.frame.wrapping_add(1);
        terminal.draw(|frame| draw(frame, app))?;

        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if app.handle_key(key).await {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    let size = terminal.size()?;
                    app.handle_mouse(mouse, Rect::new(0, 0, size.width, size.height));
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let root = app_layout(frame.area());

    draw_header(frame, root[0], app);
    draw_agents(frame, root[1], app);
    draw_prompt(frame, root[2], app);
}

fn app_layout(area: Rect) -> [Rect; 3] {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);
    [root[0], root[1], root[2]]
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let title = Line::from(vec![
        Span::styled("RMUX BROADCAST ARENA", bold_fg(Color::Green)),
        Span::raw("  "),
        Span::styled("5 agents", fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(&app.status, fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(title), area);
}

fn draw_agents(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let areas = agent_areas(area);

    for (index, (pane, area)) in app.panes.iter().zip(areas).enumerate() {
        draw_agent(frame, area, index, pane, app);
    }
}

fn agent_areas(area: Rect) -> [Rect; 5] {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(rows[0]);
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    [top[0], top[1], top[2], bottom[0], bottom[1]]
}

fn draw_agent(frame: &mut Frame<'_>, area: Rect, index: usize, pane: &AgentPane, app: &App) {
    let focused = app.focused_agent == Some(index);
    let selected_bg = selected_background(pane.agent.accent);
    let title = if focused {
        format!(" {} target ", pane.agent.title)
    } else {
        format!(" {} ", pane.agent.title)
    };
    let border_style = if focused {
        bold_fg(pane.agent.accent)
    } else {
        fg(pane.agent.accent)
    };
    let panel_style = if focused {
        Style::default().bg(selected_bg)
    } else {
        Style::default()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .style(panel_style)
        .border_style(border_style)
        .title(Span::styled(title, bold_fg(pane.agent.accent)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let inner = inner.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let mut lines = Vec::new();

    let receives_staged_prompt = app
        .focused_agent
        .map(|focused| focused == index)
        .unwrap_or(true);
    let received_last_prompt = app.active_targets.contains(&index);
    let after_send = app.last_prompt.is_some() && app.prompt.is_empty() && received_last_prompt;

    let mut logo_lines = if let Some(logo) = &pane.logo {
        render_logo_lines(logo)
    } else if pane.agent.title == "Claude" {
        claude_fallback_logo_lines(pane.agent.accent)
    } else {
        vec![Line::from(Span::styled(
            "capturing native startup mark",
            fg(Color::DarkGray),
        ))]
    };
    if after_send {
        logo_lines.truncate(usize::from(inner.height / 3).max(1));
    }
    lines.extend(logo_lines);

    lines.push(Line::raw(""));
    if let Some(last_prompt) = app.last_prompt.as_deref() {
        if !app.prompt.is_empty() && receives_staged_prompt {
            lines.push(prompt_echo_line(&app.prompt, pane.agent.accent));
        } else if app.prompt.is_empty() && received_last_prompt {
            let keep_spinner_visible = app
                .sent_frame
                .map(|sent_frame| app.frame.saturating_sub(sent_frame) < 15)
                .unwrap_or(false);
            let output_lines = render_output_lines(
                pane,
                last_prompt,
                inner.height.saturating_sub(lines.len() as u16) as usize,
            );
            if keep_spinner_visible || output_lines.is_empty() {
                lines.push(status_line(pane, app.frame));
            } else {
                lines.extend(output_lines);
            }
        }
    } else if receives_staged_prompt {
        lines.push(prompt_echo_line(&app.prompt, pane.agent.accent));
    }

    let sent_frame = if app.prompt.is_empty() && received_last_prompt {
        app.sent_frame
    } else {
        None
    };
    let top_padding = animated_top_padding(inner.height, lines.len(), sent_frame, app.frame);
    for _ in 0..top_padding {
        lines.insert(0, Line::raw(""));
    }
    trim_lines_to_height(&mut lines, inner.height);

    if focused {
        frame.render_widget(Paragraph::new("").style(panel_style), inner);
    } else {
        frame.render_widget(Clear, inner);
    }
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .style(panel_style),
        inner,
    );
}

fn draw_prompt(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if let Some(title) = app.focused_agent_title() {
        let text = Line::from(vec![
            Span::styled("broadcast > ", bold_fg(Color::DarkGray)),
            Span::styled("click this bar for multi-send", dim_fg(Color::DarkGray)),
        ]);
        let helper = if app.prompt.is_empty() {
            format!("target mode: type in {title}; Enter sends only there")
        } else {
            format!("target mode: draft is staged inside {title}")
        };
        frame.render_widget(
            Paragraph::new(vec![
                text,
                Line::from(Span::styled(helper, dim_fg(Color::DarkGray))),
            ]),
            area,
        );
        return;
    }

    let color = Color::Green;
    let text = Line::from(vec![
        Span::styled("broadcast > ", bold_fg(color)),
        Span::raw(&app.prompt),
        Span::styled("█", fg(color)),
    ]);
    let last = app
        .last_prompt
        .as_ref()
        .map(|prompt| {
            format!(
                "last target: {} -> {prompt}",
                app.target_label(&app.active_targets)
            )
        })
        .unwrap_or_else(|| "broadcast mode: Enter sends to every hidden rmux pane".to_owned());
    let paragraph = Paragraph::new(vec![
        text,
        Line::from(Span::styled(last, fg(Color::DarkGray))),
    ]);
    frame.render_widget(paragraph, area);
}

fn hit_test_agent(column: u16, row: u16, agent_area: Rect) -> Option<usize> {
    agent_areas(agent_area)
        .iter()
        .position(|agent_area| rect_contains(*agent_area, column, row))
}

fn selected_background(_accent: Color) -> Color {
    Color::Indexed(235)
}

fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

fn fg(color: Color) -> Style {
    Style::default().fg(color)
}

fn bold_fg(color: Color) -> Style {
    fg(color).add_modifier(Modifier::BOLD)
}

fn dim_fg(color: Color) -> Style {
    fg(color).add_modifier(Modifier::DIM)
}

fn render_logo_lines(logo: &CapturedLogo) -> Vec<Line<'static>> {
    logo.rows
        .iter()
        .map(|row| {
            Line::from(
                row.iter()
                    .map(|cell| Span::styled(cell.symbol.clone(), cell.style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn claude_fallback_logo_lines(accent: Color) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled("✻", bold_fg(accent))),
        Line::from(Span::styled("Claude Code", bold_fg(accent))),
    ]
}

fn update_best_logo(pane: &mut AgentPane) {
    let Some(candidate) = extract_logo(pane.agent, &pane.driver.state().snapshot) else {
        return;
    };
    if pane
        .logo
        .as_ref()
        .map(|logo| candidate.score > logo.score)
        .unwrap_or(true)
    {
        pane.logo = Some(candidate);
    }
}

fn prompt_echo_line(prompt: &str, accent: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled("prompt > ", fg(Color::DarkGray)),
        Span::styled(prompt.to_owned(), fg(accent)),
        Span::styled("█", fg(accent)),
    ])
}

fn status_line(pane: &AgentPane, frame: u64) -> Line<'static> {
    let spinner = SPINNER[((frame / 3) as usize + pane.agent.title.len()) % SPINNER.len()];
    let text = format!("{spinner} racing through rmux stream...");
    Line::from(Span::styled(text, fg(Color::DarkGray)))
}

async fn send_prompt_to_agent(agent: Agent, pane: &Pane, prompt: &str) -> Result<()> {
    let prompt = agent_prompt(prompt);
    if agent_uses_gemini(agent) {
        type_text_slowly(pane, &gemini_safe_prompt(prompt)).await?;
    } else {
        PaneSet::new(vec![pane.clone()])
            .broadcast(Input::text(&bracketed_paste(&prompt)))
            .await?;
    }

    tokio::time::sleep(Duration::from_millis(150)).await;
    pane.keyboard().press("Enter").await?;
    Ok(())
}

fn agent_prompt(prompt: &str) -> String {
    format!("{} {}", prompt.trim(), AGENT_REPLY_DIRECTIVE)
}

fn gemini_safe_prompt(prompt: String) -> String {
    format!(" {}", prompt.replace('!', "！"))
}

async fn type_text_slowly(pane: &Pane, text: &str) -> Result<()> {
    for ch in text.chars() {
        pane.keyboard().type_text(ch.to_string()).await?;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

fn trim_lines_to_height(lines: &mut Vec<Line<'static>>, height: u16) {
    let height = usize::from(height);
    if height == 0 {
        lines.clear();
        return;
    }

    let excess = lines.len().saturating_sub(height);
    if excess > 0 {
        lines.drain(0..excess);
    }
}

fn animated_top_padding(
    available_height: u16,
    line_count: usize,
    sent_frame: Option<u64>,
    current_frame: u64,
) -> u16 {
    let centered = available_height.saturating_sub(line_count as u16) / 2;
    let Some(sent_frame) = sent_frame else {
        return centered;
    };

    let elapsed = current_frame.saturating_sub(sent_frame).min(18) as u16;
    let lift = centered.saturating_mul(elapsed) / 18;
    if centered == 0 {
        0
    } else {
        centered.saturating_sub(lift).max(1)
    }
}

fn remember_visible_output(pane: &mut AgentPane, prompt: &str) {
    for line in visible_output_text(pane, prompt) {
        remember_filtered_output(pane, line);
    }
}

fn remember_output_text(pane: &mut AgentPane, prompt: &str, text: &str) {
    let text = clean_stream_text(text);
    let Some(text) = normalize_output_line(&text, prompt) else {
        return;
    };
    if should_render_output_line(&text, prompt) {
        remember_filtered_output(pane, text);
    }
}

fn remember_filtered_output(pane: &mut AgentPane, line: String) {
    if pane
        .output_lines
        .iter()
        .any(|seen| seen == &line || seen.contains(&line))
    {
        return;
    }

    if let Some(existing) = pane
        .output_lines
        .iter_mut()
        .find(|seen| line.contains(seen.as_str()))
    {
        *existing = line;
    } else {
        pane.output_lines.push(line);
    }

    let excess = pane.output_lines.len().saturating_sub(8);
    if excess > 0 {
        pane.output_lines.drain(0..excess);
    }
}

fn render_output_lines(pane: &AgentPane, prompt: &str, max_lines: usize) -> Vec<Line<'static>> {
    if max_lines == 0 {
        return Vec::new();
    }

    let lines = if pane.output_lines.is_empty() {
        visible_output_text(pane, prompt)
    } else {
        pane.output_lines.clone()
    };
    if lines.is_empty() {
        return Vec::new();
    }

    let start = lines.len().saturating_sub(max_lines);
    lines[start..]
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), fg(Color::Gray))))
        .collect()
}

fn visible_output_text(pane: &AgentPane, prompt: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let snapshot = &pane.driver.state().snapshot;
    for row in 0..snapshot.rows {
        let text = clean_snapshot_text(&snapshot.row_text(row));
        if pane
            .output_baseline
            .as_ref()
            .and_then(|baseline| baseline.get(usize::from(row)))
            .map(|baseline| baseline == &text)
            .unwrap_or(false)
        {
            continue;
        }
        let Some(text) = normalize_output_line(&text, prompt) else {
            continue;
        };
        if should_render_output_line(&text, prompt) {
            if lines.last().map(|last| last == &text).unwrap_or(false) {
                continue;
            }
            lines.push(text);
        }
    }
    lines
}

fn snapshot_rows(snapshot: &PaneSnapshot) -> Vec<String> {
    (0..snapshot.rows)
        .map(|row| clean_snapshot_text(&snapshot.row_text(row)))
        .collect()
}

fn has_blocking_permission_prompt(snapshot: &PaneSnapshot) -> bool {
    (0..snapshot.rows).any(|row| {
        let text = clean_snapshot_text(&snapshot.row_text(row)).to_ascii_lowercase();
        text.contains("permission for the") || text.contains("allow for remainder")
    })
}

fn clean_snapshot_text(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .trim()
        .to_owned()
}

fn clean_stream_text(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            skip_escape_sequence(&mut chars);
        } else if !ch.is_control() {
            cleaned.push(ch);
        }
    }
    cleaned.trim().to_owned()
}

fn clean_stream_text_with_breaks(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            skip_escape_sequence(&mut chars);
            cleaned.push('\n');
        } else if ch == '\r' || ch == '\n' {
            cleaned.push('\n');
        } else if ch == '\t' {
            cleaned.push(' ');
        } else if !ch.is_control() {
            cleaned.push(ch);
        }
    }
    cleaned
}

fn stream_candidate_lines(text: &str) -> Vec<String> {
    clean_stream_text_with_breaks(text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn skip_escape_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    match chars.peek().copied() {
        Some('[') => {
            chars.next();
            for ch in chars.by_ref() {
                if ('@'..='~').contains(&ch) {
                    break;
                }
            }
        }
        Some(']') => {
            chars.next();
            let mut previous_was_escape = false;
            for ch in chars.by_ref() {
                if ch == '\u{7}' || (previous_was_escape && ch == '\\') {
                    break;
                }
                previous_was_escape = ch == '\x1b';
            }
        }
        Some('P' | '_' | '^') => {
            chars.next();
            let mut previous_was_escape = false;
            for ch in chars.by_ref() {
                if previous_was_escape && ch == '\\' {
                    break;
                }
                previous_was_escape = ch == '\x1b';
            }
        }
        Some(_) => {
            chars.next();
        }
        None => {}
    }
}

fn normalize_output_line(text: &str, prompt: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let mut text = strip_clock_stamps(text);
    for _ in 0..4 {
        let stripped = text
            .trim_start_matches(|ch: char| ch.is_whitespace() || LEADING_NOISE_CHARS.contains(ch))
            .trim()
            .to_owned();
        if stripped == text {
            break;
        }
        text = strip_clock_stamps(&stripped);
    }

    text = strip_clock_stamps(&text);
    if text.is_empty() {
        return None;
    }

    if let Some(rest) = strip_thought_status(&text) {
        text = strip_clock_stamps(rest);
        if text.is_empty() {
            return None;
        }
    }

    if text.starts_with("[✗]") || text.starts_with("[✓]") {
        return None;
    }

    if let Some(rest) = text.strip_prefix("assistant. ") {
        if rest.starts_with("How can ") {
            text = rest.to_owned();
        }
    }

    let lower = text.to_ascii_lowercase();
    if is_prompt_echo(&lower, prompt)
        || is_repeated_prompt_echo(&lower, prompt)
        || is_agent_directive_echo(&lower, prompt)
        || is_partial_agent_prompt_echo(&lower, prompt)
    {
        return None;
    }

    Some(text)
}

fn strip_thought_status(text: &str) -> Option<&str> {
    let lower = text.to_ascii_lowercase();
    let index = lower.find("thought for")?;
    let after = &text[index + "thought for".len()..];
    let rest = after.trim_start_matches(|ch: char| {
        ch.is_whitespace() || ch.is_ascii_digit() || matches!(ch, '.' | ':' | 's' | 'S')
    });
    if rest.trim().is_empty() {
        None
    } else {
        Some(rest.trim())
    }
}

fn strip_clock_stamps(text: &str) -> String {
    let mut text = text.trim().to_owned();
    loop {
        let stripped = strip_one_clock_stamp(&text);
        if stripped == text {
            break text;
        }
        text = stripped;
    }
}

fn strip_one_clock_stamp(text: &str) -> String {
    let trimmed = text.trim();
    strip_leading_clock_stamp(trimmed)
        .or_else(|| strip_trailing_clock_stamp(trimmed))
        .map(|rest| rest.trim().to_owned())
        .unwrap_or_else(|| trimmed.to_owned())
}

fn strip_leading_clock_stamp(text: &str) -> Option<&str> {
    let mut parts = text.split_whitespace();
    let time = parts.next()?;
    let ampm = parts.next()?;
    if is_clock_stamp(time, Some(ampm)) {
        text.split_once(ampm).map(|(_, rest)| rest)
    } else {
        None
    }
}

fn strip_trailing_clock_stamp(text: &str) -> Option<&str> {
    let (before_ampm, ampm) = text.rsplit_once(char::is_whitespace)?;
    let (before_time, time) = before_ampm.trim_end().rsplit_once(char::is_whitespace)?;
    if is_clock_stamp(time, Some(ampm)) {
        Some(before_time)
    } else {
        None
    }
}

fn is_clock_stamp(time: &str, ampm: Option<&str>) -> bool {
    if !matches!(ampm.map(|value| value.to_ascii_lowercase()), Some(value) if value == "am" || value == "pm")
    {
        return false;
    }
    let mut parts = time.split(':');
    let Some(hour) = parts.next() else {
        return false;
    };
    let Some(minute) = parts.next() else {
        return false;
    };
    let second = parts.next();
    if parts.next().is_some() {
        return false;
    }
    is_one_or_two_digits(hour) && is_two_digits(minute) && second.map(is_two_digits).unwrap_or(true)
}

fn is_one_or_two_digits(value: &str) -> bool {
    (1..=2).contains(&value.len()) && value.chars().all(|ch| ch.is_ascii_digit())
}

fn is_two_digits(value: &str) -> bool {
    value.len() == 2 && value.chars().all(|ch| ch.is_ascii_digit())
}

fn should_render_output_line(text: &str, prompt: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    let lower = text.to_ascii_lowercase();
    if is_prompt_echo(&lower, prompt)
        || is_repeated_prompt_echo(&lower, prompt)
        || is_agent_directive_echo(&lower, prompt)
        || is_partial_agent_prompt_echo(&lower, prompt)
    {
        return false;
    }

    const NOISE: &str = "openai codex|welcome back|recent activity|no recent activity|sandbox|quota|tokens|model|signed in|plan:|type your message|try \"|for shortcuts|bypass permissions|press enter|do you trust|working with untrusted|claude.md|gemini.md|approval|auth|default|rmux broadcast|rmux-broadcast-demo-work|prompt >|bash:|run /review|implement {feature}|{feature}|find and fix a bug|summarize recent commits|@filename|running command|current changes|directory:|permissions:|permission for the|bash tool|allow for remainder|enter select|esc reject|↑↓ navigate|tip:|yolo|ctrl+y|claude code|gemini cli|mistral vibe|scale plan|hello. what can i help you with|i'm here to help with your coding tasks|what do you need|creating|generating|waiting|thinking|responding|turn completed|completed in|type /help|use /skills|shift+tab|shortcuts|interrupt|mcp server|previous session|session history|gpt-|mistral-|the user said|the user is|simple greeting|friendly|concise|emoji|instruction|greeting the user|greeting me|asking me|this is a|casual|conversational|not a software|engineering task|respond politely|straightforward greeting|acknowledging user|confirming project|reply directly|one short sentence|do not use tools|esc to cancel|readfolder|directory is empty|empty workspace|peer programmer|esc/|ctrl+|/tmp/";
    if NOISE.split('|').any(|needle| lower.contains(needle)) {
        return false;
    }
    if looks_like_clock_or_meter(&lower)
        || looks_like_ui_frame(text)
        || looks_like_status_line(text)
    {
        return false;
    }

    let ascii_words = text.chars().filter(|ch| ch.is_ascii_alphanumeric()).count();
    text.contains("```") || looks_like_code_line(text) || looks_like_answer_line(text, ascii_words)
}

fn is_prompt_echo(lower_text: &str, prompt: &str) -> bool {
    let prompt = prompt.trim().to_ascii_lowercase();
    if prompt.is_empty() {
        return false;
    }

    let text = lower_text
        .trim()
        .trim_start_matches(['>', '$', '#', '❯', '›'])
        .trim();
    text == prompt || (text.ends_with(&prompt) && text.len() <= prompt.len() + 4)
}

fn is_repeated_prompt_echo(lower_text: &str, prompt: &str) -> bool {
    let prompt = prompt.trim().to_ascii_lowercase();
    if prompt.is_empty() {
        return false;
    }

    let normalized_text = lower_text
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let normalized_prompt = prompt
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    if normalized_prompt.is_empty()
        || normalized_text.len() <= normalized_prompt.len()
        || normalized_text.len() % normalized_prompt.len() != 0
    {
        return false;
    }

    normalized_text
        .as_bytes()
        .chunks(normalized_prompt.len())
        .all(|chunk| chunk == normalized_prompt.as_bytes())
}

fn is_agent_directive_echo(lower_text: &str, prompt: &str) -> bool {
    let prompt = prompt.trim().to_ascii_lowercase();
    !prompt.is_empty()
        && lower_text.contains(&prompt)
        && lower_text.contains("reply directly")
        && lower_text.contains("do not use tools")
}

fn is_partial_agent_prompt_echo(lower_text: &str, prompt: &str) -> bool {
    let normalized_text = alphanumeric(lower_text);
    if normalized_text.len() < 8 {
        return false;
    }
    alphanumeric(&agent_prompt(prompt).to_ascii_lowercase()).contains(&normalized_text)
}

fn alphanumeric(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn bracketed_paste(text: &str) -> String {
    let safe_text = text.replace("\x1b[201~", "");
    format!("\x1b[200~{safe_text}\x1b[201~")
}

fn looks_like_clock_or_meter(lower: &str) -> bool {
    let text = lower.trim();
    text.ends_with(" am")
        || text.ends_with(" pm")
        || text.contains('%')
        || text.chars().all(|ch| {
            ch.is_ascii_digit()
                || ch.is_ascii_whitespace()
                || matches!(ch, ':' | '.' | '%' | '|' | '/' | '\\' | '-')
        })
}

fn looks_like_ui_frame(text: &str) -> bool {
    let ui_chars = text
        .chars()
        .filter(|ch| UI_FRAME_CHARS.contains(*ch))
        .count();
    ui_chars >= 3
}

fn looks_like_status_line(text: &str) -> bool {
    text.trim()
        .chars()
        .next()
        .is_some_and(|ch| STATUS_PREFIXES.contains(ch))
}

fn looks_like_code_line(text: &str) -> bool {
    let trimmed = text.trim();
    ["fn ", "let ", "use ", "pub ", "impl ", "::", "=>"]
        .iter()
        .any(|needle| trimmed.contains(needle))
        || trimmed.ends_with(';')
        || (trimmed.contains('{') && trimmed.contains('}'))
}

fn looks_like_answer_line(text: &str, ascii_words: usize) -> bool {
    if ascii_words < 8 {
        return false;
    }

    let trimmed = text.trim();
    trimmed.contains(' ')
        && (trimmed.ends_with('.')
            || trimmed.ends_with('?')
            || trimmed.ends_with('!')
            || trimmed.contains(": ")
            || ascii_words >= 24)
}

fn extract_logo(agent: Agent, snapshot: &PaneSnapshot) -> Option<CapturedLogo> {
    if !snapshot.is_row_major_shape() || snapshot.cols == 0 || snapshot.rows == 0 {
        return None;
    }

    if let Some(mark) = match agent.title {
        "Codex" => extract_codex_mark(snapshot),
        "Vibe" => extract_vibe_mark(snapshot),
        _ => None,
    } {
        return Some(mark);
    }

    let mut seen = vec![false; snapshot.cells.len()];
    let mut best = None::<LogoCandidate>;
    for row in 0..snapshot.rows {
        for col in 0..snapshot.cols {
            let index = cell_index(snapshot, row, col)?;
            if seen[index] || !is_component_cell(snapshot.cell(row, col)?) {
                continue;
            }

            let candidate = collect_component(snapshot, row, col, &mut seen);
            if best
                .as_ref()
                .map(|current| candidate.score > current.score)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
    }

    let candidate = best?;
    crop_logo_candidate(snapshot, &candidate)
}

struct LogoCandidate {
    min_row: u16,
    max_row: u16,
    min_col: u16,
    max_col: u16,
    score: i32,
}

fn collect_component(
    snapshot: &PaneSnapshot,
    start_row: u16,
    start_col: u16,
    seen: &mut [bool],
) -> LogoCandidate {
    let mut stack = vec![(start_row, start_col)];
    let mut cells = Vec::new();
    let mut min_row = start_row;
    let mut max_row = start_row;
    let mut min_col = start_col;
    let mut max_col = start_col;

    while let Some((row, col)) = stack.pop() {
        let Some(index) = cell_index(snapshot, row, col) else {
            continue;
        };
        if seen[index] {
            continue;
        }
        seen[index] = true;

        let Some(cell) = snapshot.cell(row, col) else {
            continue;
        };
        if !is_component_cell(cell) {
            continue;
        }

        cells.push((row, col));
        min_row = min_row.min(row);
        max_row = max_row.max(row);
        min_col = min_col.min(col);
        max_col = max_col.max(col);

        for (next_row, next_col) in [
            (row.wrapping_sub(1), col),
            (row.saturating_add(1), col),
            (row, col.wrapping_sub(1)),
            (row, col.saturating_add(1)),
        ] {
            if next_row < snapshot.rows && next_col < snapshot.cols {
                stack.push((next_row, next_col));
            }
        }
    }

    let score = score_component(snapshot, &cells, min_row, max_row, min_col, max_col);
    LogoCandidate {
        min_row,
        max_row,
        min_col,
        max_col,
        score,
    }
}

fn score_component(
    snapshot: &PaneSnapshot,
    cells: &[(u16, u16)],
    min_row: u16,
    max_row: u16,
    min_col: u16,
    max_col: u16,
) -> i32 {
    let height = i32::from(max_row - min_row + 1);
    let width = i32::from(max_col - min_col + 1);
    let mut non_ascii = 0;
    let mut colored = 0;

    for &(row, col) in cells {
        let Some(cell) = snapshot.cell(row, col) else {
            continue;
        };
        if has_explicit_color(cell) {
            colored += 1;
        }
        for ch in cell.text().chars() {
            if !ch.is_ascii() && !ch.is_whitespace() {
                non_ascii += 1;
            }
        }
    }

    if height < 2 || non_ascii == 0 {
        return i32::MIN / 2;
    }

    let too_wide = width > i32::from(snapshot.cols).max(1) / 3 && width > 32;
    let too_tall = height > 14;
    if too_wide || too_tall {
        return i32::MIN / 2;
    }
    (non_ascii * 18) + (height * 24) + (colored * 2) - (width * 2)
}

fn crop_logo_candidate(snapshot: &PaneSnapshot, candidate: &LogoCandidate) -> Option<CapturedLogo> {
    if candidate.score <= 0 {
        return None;
    }
    crop_logo_area(
        snapshot,
        candidate.min_row,
        candidate.max_row,
        candidate.min_col.saturating_sub(2),
        (candidate.max_col + 2).min(snapshot.cols.saturating_sub(1)),
        candidate.score,
    )
}

fn extract_codex_mark(snapshot: &PaneSnapshot) -> Option<CapturedLogo> {
    let row = (0..snapshot.rows).find(|row| snapshot.row_text(*row).contains("OpenAI Codex"))?;
    let (mut min_col, mut max_col) = row_text_bounds(snapshot, row)?;
    while min_col < max_col && is_box_edge_cell(snapshot.cell(row, min_col)?) {
        min_col += 1;
    }
    while max_col > min_col && is_box_edge_cell(snapshot.cell(row, max_col)?) {
        max_col -= 1;
    }
    crop_logo_area(snapshot, row, row, min_col, max_col, 10_000)
}

fn extract_vibe_mark(snapshot: &PaneSnapshot) -> Option<CapturedLogo> {
    let mut best = None::<LogoCandidate>;
    let mut row = 0;
    while row < snapshot.rows {
        let row_score = vibe_logo_row_score(snapshot, row);
        if row_score < 2 {
            row += 1;
            continue;
        }

        let min_row = row;
        let mut max_row = row;
        let mut score = 0;
        while max_row < snapshot.rows
            && max_row.saturating_sub(min_row) < 3
            && vibe_logo_row_score(snapshot, max_row) >= 2
        {
            score += vibe_logo_row_score(snapshot, max_row);
            max_row += 1;
        }
        max_row = max_row.saturating_sub(1);

        let Some((min_col, max_col)) = vibe_logo_bounds(snapshot, min_row, max_row) else {
            row = max_row.saturating_add(1);
            continue;
        };
        let width = max_col.saturating_sub(min_col).saturating_add(1);
        if width <= 32 {
            let candidate = LogoCandidate {
                min_row,
                max_row,
                min_col,
                max_col,
                score: 9_000 + score,
            };
            if best
                .as_ref()
                .map(|current| candidate.score > current.score)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
        row = max_row.saturating_add(1);
    }

    let candidate = best?;
    crop_logo_area(
        snapshot,
        candidate.min_row,
        candidate.max_row,
        candidate.min_col.saturating_sub(1),
        (candidate.max_col + 1).min(snapshot.cols.saturating_sub(1)),
        candidate.score,
    )
}

fn vibe_logo_row_score(snapshot: &PaneSnapshot, row: u16) -> i32 {
    let mut logo_chars = 0;
    let mut ascii_letters = 0;
    for col in 0..snapshot.cols {
        let Some(cell) = snapshot.cell(row, col) else {
            continue;
        };
        for ch in cell.text().chars() {
            if ch.is_ascii_alphabetic() {
                ascii_letters += 1;
            } else if is_vibe_logo_char(ch) {
                logo_chars += 1;
            }
        }
    }

    if ascii_letters > 4 {
        0
    } else {
        logo_chars
    }
}

fn vibe_logo_bounds(snapshot: &PaneSnapshot, min_row: u16, max_row: u16) -> Option<(u16, u16)> {
    let mut min_col = None::<u16>;
    let mut max_col = None::<u16>;
    for row in min_row..=max_row {
        for col in 0..snapshot.cols {
            let cell = snapshot.cell(row, col)?;
            if cell.text().chars().any(is_vibe_logo_char) {
                min_col = Some(min_col.map(|current| current.min(col)).unwrap_or(col));
                max_col = Some(max_col.map(|current| current.max(col)).unwrap_or(col));
            }
        }
    }
    Some((min_col?, max_col?))
}

fn is_vibe_logo_char(ch: char) -> bool {
    !ch.is_ascii() && !ch.is_whitespace() && !VIBE_EXCLUDED_CHARS.contains(ch)
}

fn crop_logo_area(
    snapshot: &PaneSnapshot,
    min_row: u16,
    max_row: u16,
    min_col: u16,
    max_col: u16,
    score: i32,
) -> Option<CapturedLogo> {
    let mut rows = Vec::new();
    for row in min_row..=max_row {
        let mut cells = Vec::new();
        for col in min_col..=max_col {
            let cell = snapshot.cell(row, col)?;
            cells.push(LogoCell {
                symbol: logo_symbol(cell),
                style: cell_style(cell),
                blank: is_default_blank_cell(cell),
            });
        }
        rows.push(cells);
    }

    Some(CapturedLogo {
        rows: trim_logo(rows)?,
        score,
    })
}

fn row_text_bounds(snapshot: &PaneSnapshot, row: u16) -> Option<(u16, u16)> {
    let mut min_col = None::<u16>;
    let mut max_col = None::<u16>;
    for col in 0..snapshot.cols {
        let cell = snapshot.cell(row, col)?;
        if cell.text().chars().any(|ch| !ch.is_whitespace()) {
            min_col = Some(min_col.map(|current| current.min(col)).unwrap_or(col));
            max_col = Some(max_col.map(|current| current.max(col)).unwrap_or(col));
        }
    }
    Some((min_col?, max_col?))
}

fn trim_logo(mut rows: Vec<Vec<LogoCell>>) -> Option<Vec<Vec<LogoCell>>> {
    while rows
        .first()
        .map(|row| row.iter().all(is_blank_logo_cell))
        .unwrap_or(false)
    {
        rows.remove(0);
    }
    while rows
        .last()
        .map(|row| row.iter().all(is_blank_logo_cell))
        .unwrap_or(false)
    {
        rows.pop();
    }
    if rows.is_empty() {
        return None;
    }

    while rows
        .iter()
        .all(|row| row.first().map(is_blank_logo_cell).unwrap_or(true))
    {
        for row in &mut rows {
            if !row.is_empty() {
                row.remove(0);
            }
        }
    }
    while rows
        .iter()
        .all(|row| row.last().map(is_blank_logo_cell).unwrap_or(true))
    {
        for row in &mut rows {
            row.pop();
        }
    }

    if rows.iter().any(|row| !row.is_empty()) {
        Some(rows)
    } else {
        None
    }
}

fn logo_symbol(cell: &PaneCell) -> String {
    if cell.is_padding() {
        " ".to_owned()
    } else {
        let symbol = glyph_symbol(&cell.glyph);
        if symbol.is_empty() {
            " ".to_owned()
        } else {
            symbol.to_owned()
        }
    }
}

fn is_blank_logo_cell(cell: &LogoCell) -> bool {
    cell.blank
}

fn is_component_cell(cell: &PaneCell) -> bool {
    !cell.is_padding()
        && cell
            .text()
            .chars()
            .any(|ch| !ch.is_ascii() && !ch.is_whitespace())
}

fn has_explicit_color(cell: &PaneCell) -> bool {
    explicit_color(cell.foreground)
        || explicit_color(cell.background)
        || explicit_color(cell.underline)
}

fn explicit_color(color: PaneColor) -> bool {
    !matches!(
        color,
        PaneColor::Default | PaneColor::None | PaneColor::Terminal
    )
}

fn is_default_blank_cell(cell: &PaneCell) -> bool {
    cell.text().chars().all(char::is_whitespace)
        && !has_explicit_color(cell)
        && cell.attributes.is_empty()
}

fn is_box_edge_cell(cell: &PaneCell) -> bool {
    cell.text()
        .chars()
        .all(|ch| ch.is_whitespace() || BOX_EDGE_CHARS.contains(ch))
}

fn cell_index(snapshot: &PaneSnapshot, row: u16, col: u16) -> Option<usize> {
    if row >= snapshot.rows || col >= snapshot.cols {
        return None;
    }
    usize::from(row)
        .checked_mul(usize::from(snapshot.cols))?
        .checked_add(usize::from(col))
}

async fn cleanup() -> Result<()> {
    let Ok(rmux) = demo_rmux_builder()?
        .default_timeout(Duration::from_secs(3))
        .connect()
        .await
    else {
        println!("no rmux daemon for `{SESSION}`");
        return Ok(());
    };
    let session_name = SessionName::new(SESSION)?;

    if rmux.has_session(session_name.clone()).await? {
        rmux.session(session_name).await?.kill().await?;
        println!("removed rmux session `{SESSION}`");
    } else {
        println!("no rmux session named `{SESSION}`");
    }
    let _ = rmux.shutdown().await;

    Ok(())
}

fn check_commands() -> Result<()> {
    if !command_exists("rmux") {
        return Err("missing command in PATH: rmux".into());
    }

    let agents = selected_agents()?;
    println!(
        "rmux is available; using: {}",
        agents
            .iter()
            .map(|agent| agent.title)
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

fn selected_agents() -> Result<Vec<Agent>> {
    let available = AGENTS
        .iter()
        .copied()
        .filter(|agent| {
            agent
                .command
                .first()
                .map(|command| command_exists(command))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    if available.is_empty() {
        return Err(format!(
            "missing agent command in PATH: install at least one of {}",
            AGENTS
                .iter()
                .filter_map(|agent| agent.command.first().copied())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into());
    }

    let mut agents = available.clone();
    while agents.len() < AGENTS.len() {
        let next = available[agents.len() % available.len()];
        agents.push(next);
    }
    agents.truncate(AGENTS.len());
    Ok(agents)
}

async fn launch_agent(pane: &Pane, agent: Agent, cwd: &Path) -> Result<()> {
    let mut spawn = pane
        .spawn(agent.command.iter().copied())
        .cwd(cwd.to_path_buf())
        .kill_existing(true)
        .title(agent.title)
        .keep_alive_on_exit(true);
    if agent_uses_claude(agent) {
        spawn = spawn.env("IS_DEMO", "1");
    }
    spawn.await?;
    Ok(())
}

async fn split_agent(
    parent: &Pane,
    direction: SplitDirection,
    agent: Agent,
    cwd: &Path,
) -> Result<Pane> {
    let mut split = parent
        .split_with(direction)
        .spawn(agent.command.iter().copied())
        .cwd(cwd.to_path_buf())
        .title(agent.title)
        .keep_alive_on_exit(true);
    if agent_uses_claude(agent) {
        split = split.env("IS_DEMO", "1");
    }
    Ok(split.await?)
}

fn agent_uses_claude(agent: Agent) -> bool {
    agent.command.first() == Some(&"claude")
}

fn agent_uses_gemini(agent: Agent) -> bool {
    agent.command.first() == Some(&"gemini")
}

async fn stable_pane(session: &Session, pane: &Pane, agent: Agent) -> Result<Pane> {
    let pane_id = pane
        .id()
        .await?
        .ok_or_else(|| format!("pane for {} disappeared after creation", agent.title))?;
    Ok(session.pane_by_id(pane_id).await?)
}

fn spawn_render_task(
    index: usize,
    pane: Pane,
    tx: mpsc::UnboundedSender<RenderMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let Ok(mut stream) = pane.render_stream().await else {
            return;
        };
        while let Ok(Some(update)) = stream.next().await {
            if tx.send((index, AgentUpdate::Snapshot(update))).is_err() {
                break;
            }
        }
    })
}

fn spawn_line_task(
    index: usize,
    pane: Pane,
    tx: mpsc::UnboundedSender<RenderMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let Ok(mut stream) = pane.output_stream().await else {
            return;
        };
        let mut pending = String::new();
        while let Ok(Some(item)) = stream.next().await {
            let PaneOutputChunk::Bytes { bytes, .. } = item else {
                continue;
            };
            if bytes.is_empty() {
                continue;
            }

            let text = String::from_utf8_lossy(&bytes);
            for line in stream_candidate_lines(&text) {
                if tx
                    .send((index, AgentUpdate::OutputLine(line.clone())))
                    .is_err()
                {
                    return;
                }
                pending.push_str(&line);
                pending.push('\n');
            }

            if pending.len() > 4096 {
                let keep_from = pending
                    .char_indices()
                    .rev()
                    .map(|(index, _)| index)
                    .find(|index| pending.len().saturating_sub(*index) >= 2048)
                    .unwrap_or(0);
                pending.drain(..keep_from);
            }
            for line in stream_candidate_lines(&pending) {
                if tx.send((index, AgentUpdate::OutputLine(line))).is_err() {
                    return;
                }
            }
        }
    })
}

async fn send_startup_keys(pane: &Pane, agent: Agent) -> Result<()> {
    if !agent.startup_keys.is_empty() {
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
        if !should_continue {
            return Ok(());
        }
    }
    for key in agent.startup_keys {
        pane.keyboard().press(*key).await?;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    Ok(())
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
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn demo_rmux_builder() -> Result<rmux_sdk::RmuxBuilder> {
    let builder = Rmux::builder();
    if env::consts::OS == "windows" {
        Ok(builder.windows_pipe(WINDOWS_PIPE))
    } else {
        Ok(builder.unix_socket(demo_socket_path()?))
    }
}

fn demo_work_dir() -> Result<PathBuf> {
    let path = env::temp_dir().join("rmux-broadcast-demo-work");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn demo_socket_path() -> Result<PathBuf> {
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

fn terminal_size_or_default() -> (u16, u16) {
    crossterm::terminal::size().unwrap_or((220, 60))
}

struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableMouseCapture,
            LeaveAlternateScreen,
            cursor::Show
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_vibe_startup_greeting() {
        assert!(!should_render_output_line(
            "Hello. What can I help you with?",
            "write a haiku"
        ));
        assert!(!should_render_output_line(
            "I'm here to help with your coding tasks. What do you need?",
            "write a haiku"
        ));
    }

    #[test]
    fn renders_real_agent_answers() {
        assert!(should_render_output_line(
            "I'm doing great, thanks! Ready to help",
            "Hello how are you?"
        ));
        assert!(should_render_output_line(
            "What are we working on today?",
            "Hello how are you?"
        ));
    }

    #[test]
    fn extracts_cursor_drawn_agent_answers_from_raw_stream() {
        let raw = concat!(
            "\x1b[10;5H◆ Thought for 1.0s",
            "\x1b[11;5HI'm doing well, thanks for asking! Ready  11:31 PM",
            "\x1b[12;5Hto help with any software engineering"
        );
        let lines = stream_candidate_lines(raw);

        assert!(lines
            .iter()
            .any(|line| should_render_output_line(line, "Hello how are you?")));
        assert!(lines
            .iter()
            .any(|line| line.contains("software engineering")));
    }

    #[test]
    fn renders_grok_and_gemini_answer_fragments() {
        assert_eq!(
            normalize_output_line("◆ Thought for 0.6sI'm doing well, thanks.", "Hello"),
            Some("I'm doing well, thanks.".to_owned())
        );
        assert!(should_render_output_line(
            "you with your project today?",
            "Hello how are you?"
        ));
        assert!(should_render_output_line(
            "I am ready to assist you in the workspace.",
            "Hello how are you?"
        ));
        assert_eq!(
            normalize_output_line("✦ I am doing well, thank you.", "Hello"),
            Some("I am doing well, thank you.".to_owned())
        );
        assert_eq!(
            normalize_output_line(
                "Hello how are you? Reply directly in one short sentence. Do not use tools.",
                "Hello how are you?"
            ),
            None
        );
        assert_eq!(
            normalize_output_line("you? Reply directl", "Hello how are you?"),
            None
        );
        assert_eq!(
            normalize_output_line("sentence. Do not use", "Hello how are you?"),
            None
        );
        assert!(!should_render_output_line(
            "The user is asking a casual greeting.",
            "Hello how are you?"
        ));
    }

    #[test]
    fn claude_fallback_logo_uses_agent_accent() {
        let lines = claude_fallback_logo_lines(AGENTS[0].accent);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].content, "✻");
        assert_eq!(lines[0].spans[0].style.fg, Some(AGENTS[0].accent));
        assert!(lines[0].spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn prompt_input_uses_bracketed_paste() {
        assert_eq!(bracketed_paste("hello !"), "\x1b[200~hello !\x1b[201~");
        assert_eq!(
            bracketed_paste("hello \x1b[201~ world"),
            "\x1b[200~hello  world\x1b[201~"
        );
    }

    #[test]
    fn gemini_prompt_avoids_shell_shortcut() {
        assert_eq!(gemini_safe_prompt("hello !".to_owned()), " hello ！");
        assert_eq!(gemini_safe_prompt("!status".to_owned()), " ！status");
    }

    #[test]
    fn trim_lines_preserves_prompt_tail_when_logo_is_tall() {
        let mut lines = vec![
            Line::raw("logo 1"),
            Line::raw("logo 2"),
            Line::raw("logo 3"),
            Line::raw("prompt > hello"),
            Line::raw("waiting"),
        ];

        trim_lines_to_height(&mut lines, 2);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].content, "prompt > hello");
        assert_eq!(lines[1].spans[0].content, "waiting");
    }
}
