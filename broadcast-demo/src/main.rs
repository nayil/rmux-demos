use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
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
    EnsureSession, Pane, PaneCell, PaneColor, PaneSet, PaneSnapshot, RenderUpdate, Rmux, Session,
    SessionName, SplitDirection, TerminalSizeSpec,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::task::JoinHandle;

const SESSION: &str = "broadcast-demo";
const SPINNER: &[&str] = &["|", "/", "-", "\\"];
const LEADING_NOISE_CHARS: &str = "●•✓◆■✻✽✶✢⠋⠙⠹⠸⠼⠴⠦⠧⠇⠃┃│>❯›-\\/|";
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
        Color::Rgb(255, 129, 102),
    ),
    agent(
        "Codex",
        &["codex", "--dangerously-bypass-approvals-and-sandbox"],
        &["Enter"],
        Color::Rgb(77, 171, 247),
    ),
    agent(
        "Gemini",
        &["gemini", "--skip-trust", "--approval-mode", "yolo"],
        &[],
        Color::Rgb(137, 180, 250),
    ),
    agent("Vibe", &["vibe", "--trust"], &[], Color::Rgb(166, 227, 161)),
    agent(
        "Grok",
        &["grok", "--always-approve"],
        &[],
        Color::Rgb(249, 226, 175),
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

type RenderMessage = (usize, RenderUpdate);

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
    last_prompt_dismissal: Instant,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
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
        let rmux = Rmux::builder()
            .unix_socket(demo_socket_path()?)
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

        let second = split_agent(&root, SplitDirection::Right, AGENTS[1], &work_dir).await?;
        split_agent(&second, SplitDirection::Right, AGENTS[2], &work_dir).await?;
        split_agent(&root, SplitDirection::Down, AGENTS[3], &work_dir).await?;
        split_agent(&second, SplitDirection::Down, AGENTS[4], &work_dir).await?;
        launch_agent(&root, AGENTS[0], &work_dir).await?;

        let panes = discover_agent_panes(&rmux).await?;

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
            render_tasks.push(spawn_render_task(index, pane, render_tx.clone()));
            agent_panes.push(AgentPane {
                agent,
                driver,
                logo,
                output_baseline: None,
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
        while let Ok(message) = self.render_updates.try_recv() {
            let Some(pane) = self.panes.get_mut(message.0) else {
                continue;
            };
            pane.driver.apply_snapshot(message.1.into_snapshot());
            if self.last_prompt.is_none() {
                update_best_logo(pane);
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
        let panes = self.target_panes(targets);
        if panes.is_empty() {
            self.status = "broadcast error: no target pane selected".to_owned();
            return false;
        }

        let keyboard = PaneSet::new(panes).keyboard();
        let sent = async {
            keyboard.type_text(prompt).await?;
            tokio::time::sleep(Duration::from_millis(250)).await;
            keyboard.press("Enter").await
        };
        if let Err(error) = sent.await {
            self.status = format!("broadcast error: {error}");
            return false;
        }
        true
    }

    async fn capture_output_baselines(&mut self) {
        for pane in &mut self.panes {
            if let Ok(snapshot) = pane.driver.pane().snapshot().await {
                pane.driver.apply_snapshot(snapshot);
            }
            pane.output_baseline = Some(snapshot_rows(&pane.driver.state().snapshot));
        }
    }

    fn target_panes(&self, targets: &[usize]) -> Vec<Pane> {
        targets
            .iter()
            .filter_map(|index| self.panes.get(*index))
            .map(|pane| pane.driver.pane().clone())
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
                    if self.send_prompt_to_targets(&targets, &prompt).await {
                        self.last_prompt = Some(prompt);
                        self.active_targets = targets;
                        self.sent_frame = Some(self.frame);
                        self.status = match self.focused_agent_title() {
                            Some(title) => format!("sent to {title}"),
                            None => "sent to all agents".to_owned(),
                        };
                    }
                }
                self.prompt.clear();
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

    if let Some(logo) = &pane.logo {
        lines.extend(render_logo_lines(logo));
    } else {
        lines.push(Line::from(Span::styled(
            "capturing native startup mark",
            fg(Color::DarkGray),
        )));
    }

    lines.push(Line::raw(""));
    let receives_staged_prompt = app
        .focused_agent
        .map(|focused| focused == index)
        .unwrap_or(true);
    let received_last_prompt = app.active_targets.contains(&index);
    if let Some(last_prompt) = app.last_prompt.as_deref() {
        if !app.prompt.is_empty() && receives_staged_prompt {
            lines.push(prompt_echo_line(&app.prompt, pane.agent.accent));
        } else if app.prompt.is_empty() && received_last_prompt {
            let output_lines = live_output_lines(
                pane,
                last_prompt,
                inner.height.saturating_sub(lines.len() as u16) as usize,
            );
            if output_lines.is_empty() {
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

fn selected_background(accent: Color) -> Color {
    match accent {
        Color::Rgb(red, green, blue) => Color::Rgb(red / 4, green / 4, blue / 4),
        _ => Color::Rgb(24, 24, 24),
    }
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

fn live_output_lines(pane: &AgentPane, prompt: &str, max_lines: usize) -> Vec<Line<'static>> {
    if max_lines == 0 {
        return Vec::new();
    }

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

    if lines.is_empty() {
        return Vec::new();
    }

    let start = lines.len().saturating_sub(max_lines);
    lines[start..]
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), fg(Color::Gray))))
        .collect()
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

    if text.starts_with("[✗]") || text.starts_with("[✓]") {
        return None;
    }

    if let Some(rest) = text.strip_prefix("assistant. ") {
        if rest.starts_with("How can ") {
            text = rest.to_owned();
        }
    }

    let lower = text.to_ascii_lowercase();
    if is_prompt_echo(&lower, prompt) || is_repeated_prompt_echo(&lower, prompt) {
        return None;
    }

    Some(text)
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
    if is_prompt_echo(&lower, prompt) || is_repeated_prompt_echo(&lower, prompt) {
        return false;
    }

    const NOISE: &str = "openai codex|welcome back|recent activity|no recent activity|workspace|sandbox|quota|tokens|model|signed in|plan:|type your message|try \"|for shortcuts|bypass permissions|press enter|do you trust|working with untrusted|claude.md|gemini.md|approval|auth|default|rmux broadcast|rmux-broadcast-demo-work|prompt >|bash:|run /review|implement {feature}|{feature}|find and fix a bug|summarize recent commits|@filename|running command|current changes|directory:|permissions:|permission for the|bash tool|allow for remainder|enter select|esc reject|↑↓ navigate|tip:|yolo|ctrl+y|claude code|gemini cli|mistral vibe|scale plan|hello. what can i help you with|i'm here to help with your coding tasks|what do you need|thought|creating|generating|waiting|thinking|responding|turn completed|completed in|type /help|use /skills|shift+tab|shortcuts|interrupt|mcp server|previous session|session history|gpt-|mistral-|the user said|simple greeting|friendly|concise|emoji|instruction|greeting the user|esc to cancel|readfolder|directory is empty|empty workspace|peer programmer|esc/|ctrl+|/tmp/";
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
    Some(
        usize::from(row)
            .checked_mul(usize::from(snapshot.cols))?
            .checked_add(usize::from(col))?,
    )
}

async fn cleanup() -> Result<()> {
    let Ok(rmux) = Rmux::builder()
        .unix_socket(demo_socket_path()?)
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
    let mut missing = Vec::new();
    if !command_exists("rmux") {
        missing.push("rmux");
    }

    for agent in AGENTS {
        if let Some(command) = agent
            .command
            .first()
            .filter(|command| !command_exists(command))
        {
            missing.push(command);
        }
    }

    if missing.is_empty() {
        println!("rmux and all agent commands are available");
        Ok(())
    } else {
        Err(format!("missing commands in PATH: {}", missing.join(", ")).into())
    }
}

async fn launch_agent(pane: &Pane, agent: Agent, cwd: &Path) -> Result<()> {
    pane.spawn(agent.command.iter().copied())
        .cwd(cwd.to_path_buf())
        .kill_existing(true)
        .title(agent.title)
        .keep_alive_on_exit(true)
        .await?;
    Ok(())
}

async fn split_agent(
    parent: &Pane,
    direction: SplitDirection,
    agent: Agent,
    cwd: &Path,
) -> Result<Pane> {
    Ok(parent
        .split_with(direction)
        .spawn(agent.command.iter().copied())
        .cwd(cwd.to_path_buf())
        .title(agent.title)
        .keep_alive_on_exit(true)
        .await?)
}

async fn discover_agent_panes(rmux: &Rmux) -> Result<Vec<(Agent, Pane)>> {
    let mut panes = Vec::with_capacity(AGENTS.len());
    for agent in AGENTS {
        let pane = rmux.get_pane_by_title(agent.title).await?;
        panes.push((*agent, pane));
    }
    Ok(panes)
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
        loop {
            match stream.next().await {
                Ok(Some(update)) => {
                    if tx.send((index, update)).is_err() {
                        break;
                    }
                }
                Ok(None) | Err(_) => break,
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
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
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
}
