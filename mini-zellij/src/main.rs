use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use ratatui_rmux::{PaneDriver, PaneWidget};
use rmux_sdk::{
    EnsureSession, Pane, RenderUpdate, Rmux, Session, SessionName, SplitDirection, TerminalSizeSpec,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

const SESSION: &str = "mini-zellij";
const SDK_DAEMON_BINARY_ENV: &str = "RMUX_SDK_DAEMON_BINARY";
const WINDOWS_PIPE: &str = r"\\.\pipe\rmux-demo-mini-zellij";
const ZELLIJ_SESSION: &str = "quiet-weasel";
const BAR_BG: Color = Color::Black;
const PANE_BG: Color = Color::Indexed(235);
const TEXT: Color = Color::Indexed(189);
const MUTED: Color = Color::Indexed(245);
const GREEN: Color = Color::Indexed(154);
const ORANGE: Color = Color::Indexed(214);
const TAG_FG: Color = Color::Indexed(233);
const TAG_BG: Color = Color::Indexed(251);
const KEY_RED: Color = Color::Indexed(160);
const BASE_BG: Color = Color::Indexed(238);

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PaneNode {
    Leaf(usize),
    Split {
        axis: SplitAxis,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExitMode {
    Quit,
    Detach,
}

struct WorkspacePane {
    driver: PaneDriver,
}

type RenderMessage = (usize, RenderUpdate);

struct App {
    rmux: Option<Rmux>,
    session: Option<Session>,
    panes: Vec<WorkspacePane>,
    layout: PaneNode,
    active: usize,
    hover: Option<usize>,
    awaiting_prefix: bool,
    render_tx: UnboundedSender<RenderMessage>,
    render_updates: UnboundedReceiver<RenderMessage>,
    render_tasks: Vec<JoinHandle<()>>,
    next_pane_number: usize,
    pane_title: String,
}

fn main() -> Result<()> {
    force_path_rmux_binary();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async_main())
}

fn force_path_rmux_binary() {
    env::set_var(SDK_DAEMON_BINARY_ENV, "rmux");
}

async fn async_main() -> Result<()> {
    match env::args().nth(1).as_deref() {
        Some("check") => check_commands(),
        Some("cleanup") => cleanup().await,
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: mini-zellij [check|cleanup]");
            Ok(())
        }
        None => run_app().await,
    }
}

async fn run_app() -> Result<()> {
    check_commands()?;

    let mut app = App::new().await?;
    let tui_result = run_tui(&mut app).await;

    match tui_result {
        Ok(mode) => app.cleanup(mode).await?,
        Err(error) => {
            let _ = app.cleanup(ExitMode::Quit).await;
            return Err(error);
        }
    }
    Ok(())
}

impl App {
    async fn new() -> Result<Self> {
        let rmux = demo_rmux_builder()?
            .default_timeout(Duration::from_secs(3))
            .connect_or_start()
            .await?;

        let (cols, rows) = terminal_size_or_default();
        let session_name = SessionName::new(SESSION)?;
        let mut attached = false;
        let session = match rmux.session(session_name.clone()).await {
            Ok(existing) if existing.exists().await.unwrap_or(false) => {
                attached = true;
                existing
            }
            _ => {
                rmux.ensure_session(
                    EnsureSession::named(session_name)
                        .create_only()
                        .detached(true)
                        .working_directory(shell_work_dir().to_string_lossy().into_owned())
                        .size(TerminalSizeSpec::new(cols.max(220), rows.max(90))),
                )
                .await?
            }
        };

        tokio::time::sleep(Duration::from_millis(450)).await;
        let (render_tx, render_updates) = mpsc::unbounded_channel();
        let pane_title = zellij_pane_title();
        let mut app = Self {
            rmux: Some(rmux),
            session: Some(session),
            panes: Vec::new(),
            layout: PaneNode::Leaf(0),
            active: 0,
            hover: None,
            awaiting_prefix: false,
            render_tx,
            render_updates,
            render_tasks: Vec::new(),
            next_pane_number: 2,
            pane_title,
        };

        if attached {
            let panes_to_add = {
                let session = app
                    .session
                    .as_ref()
                    .ok_or("rmux session is not available")?;
                let live_panes = session.window(0).panes().await?;
                if live_panes.is_empty() {
                    let _ = session.kill().await;
                    return Err("detached mini-zellij session had no panes; run again".into());
                }
                let mut panes = Vec::with_capacity(live_panes.len());
                for live in &live_panes {
                    panes.push((session.pane_by_id(live.id).await?, live.active));
                }
                panes
            };
            for (pane, active) in panes_to_add {
                let index = app.add_pane(pane).await?;
                if active {
                    app.focus(index);
                }
            }
            app.next_pane_number = app.panes.len() + 1;
            app.layout = layout_for_existing_panes(app.panes.len());
        } else {
            let session = app
                .session
                .as_ref()
                .ok_or("rmux session is not available")?;
            let root = session.pane(0, 0);
            root.resize(TerminalSizeSpec::new(cols.max(220), rows.max(90)))
                .await?;
            launch_shell(&root, "Pane #1").await?;
            let root = find_demo_pane(app.rmux.as_ref(), "Pane #1").await?;
            app.add_pane(root).await?;
        }
        Ok(app)
    }

    async fn cleanup(&mut self, mode: ExitMode) -> Result<()> {
        for task in &self.render_tasks {
            task.abort();
        }
        match mode {
            ExitMode::Quit => {
                if let Some(session) = self.session.take() {
                    let _ = session.kill().await;
                }
                if let Some(rmux) = self.rmux.take() {
                    let _ = rmux.shutdown().await;
                }
            }
            ExitMode::Detach => {
                self.session.take();
                self.rmux.take();
            }
        }
        Ok(())
    }

    async fn add_pane(&mut self, pane: Pane) -> Result<usize> {
        let index = self.panes.len();
        let mut driver = PaneDriver::new(pane.clone());
        let _ = driver.refresh().await;
        self.render_tasks
            .push(spawn_render_task(index, pane, self.render_tx.clone()));
        self.panes.push(WorkspacePane { driver });
        Ok(index)
    }

    async fn split_active(&mut self, axis: SplitAxis) -> Result<()> {
        let title = format!("Pane #{}", self.next_pane_number);
        let direction = match axis {
            SplitAxis::Horizontal => SplitDirection::Right,
            SplitAxis::Vertical => SplitDirection::Down,
        };
        let parent = self.panes[self.active].driver.pane().clone();
        split_shell(&parent, direction, &title).await?;
        let new_pane = find_demo_pane(self.rmux.as_ref(), &title).await?;

        tokio::time::sleep(Duration::from_millis(180)).await;
        let previous = self.active;
        let new_index = self.add_pane(new_pane).await?;
        let replacement = PaneNode::Split {
            axis,
            first: Box::new(PaneNode::Leaf(previous)),
            second: Box::new(PaneNode::Leaf(new_index)),
        };
        replace_leaf(&mut self.layout, previous, replacement);
        self.focus(new_index);
        self.next_pane_number += 1;
        Ok(())
    }

    fn drain_render_updates(&mut self) {
        while let Ok(message) = self.render_updates.try_recv() {
            if let Some(pane) = self.panes.get_mut(message.0) {
                pane.driver.apply_snapshot(message.1.into_snapshot());
            }
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Option<ExitMode> {
        if self.awaiting_prefix {
            self.awaiting_prefix = false;
            match key.code {
                KeyCode::Char('%') => {
                    if let Err(error) = self.split_active(SplitAxis::Horizontal).await {
                        eprintln!("split error: {error}");
                    }
                }
                KeyCode::Char('"') => {
                    if let Err(error) = self.split_active(SplitAxis::Vertical).await {
                        eprintln!("split error: {error}");
                    }
                }
                KeyCode::Char('d') => return Some(ExitMode::Detach),
                KeyCode::Char('q') => return Some(ExitMode::Quit),
                KeyCode::Esc => {}
                _ => self.forward_key(key).await,
            }
            return None;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('b') => {
                    self.awaiting_prefix = true;
                    return None;
                }
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(ExitMode::Quit),
                _ => {}
            }
        }

        match key.code {
            KeyCode::Tab | KeyCode::Right | KeyCode::Down => {
                self.focus_next();
                None
            }
            KeyCode::Left | KeyCode::Up => {
                self.focus_previous();
                None
            }
            _ => {
                self.forward_key(key).await;
                None
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        let root = app_layout(area);
        let target = hit_test(&self.layout, root.body, mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) => self.hover = target,
            MouseEventKind::Down(MouseButton::Left) => {
                self.hover = target;
                if let Some(index) = target {
                    self.focus(index);
                }
            }
            _ => {}
        }
    }

    fn focus_next(&mut self) {
        self.focus((self.active + 1) % self.panes.len().max(1));
    }

    fn focus_previous(&mut self) {
        self.focus(
            self.active
                .checked_sub(1)
                .unwrap_or(self.panes.len().saturating_sub(1)),
        );
    }

    fn focus(&mut self, index: usize) {
        self.active = index;
        self.hover = Some(index);
    }

    async fn forward_key(&mut self, key: KeyEvent) {
        let Some(pane) = self.panes.get(self.active) else {
            return;
        };
        let keyboard = pane.driver.pane().keyboard();
        let result = if let KeyCode::Char(ch) = key.code {
            if key
                .modifiers
                .intersects(KeyModifiers::ALT | KeyModifiers::CONTROL)
            {
                Ok(())
            } else {
                keyboard.type_text(ch.to_string()).await
            }
        } else {
            match key.code {
                KeyCode::Enter => keyboard.press("Enter").await,
                KeyCode::Backspace => keyboard.press("BSpace").await,
                KeyCode::Delete => keyboard.press("Delete").await,
                KeyCode::Esc => keyboard.press("Escape").await,
                KeyCode::Left => keyboard.press("Left").await,
                KeyCode::Right => keyboard.press("Right").await,
                KeyCode::Up => keyboard.press("Up").await,
                KeyCode::Down => keyboard.press("Down").await,
                _ => Ok(()),
            }
        };
        if let Err(error) = result {
            eprintln!("input error: {error}");
        }
    }
}

async fn run_tui(app: &mut App) -> Result<ExitMode> {
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
        terminal.draw(|frame| draw(frame, app))?;

        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Some(mode) = app.handle_key(key).await {
                        return Ok(mode);
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
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Paragraph::new("").style(zbg(PANE_BG)), area);

    let root = app_layout(area);
    draw_top_bar(frame, root.top, app);
    draw_layout(frame, root.body, app, &app.layout);
    draw_bottom_bar(frame, root.bottom, app);
}

#[derive(Clone, Copy)]
struct AppAreas {
    top: Rect,
    body: Rect,
    bottom: Rect,
}

fn app_layout(area: Rect) -> AppAreas {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(area);
    AppAreas {
        top: chunks[0],
        body: chunks[1],
        bottom: chunks[2],
    }
}

fn draw_top_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Paragraph::new("").style(zbg(BAR_BG)), area);
    let chunks = if app.panes.len() > 1 && area.width >= 120 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(22)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(0)])
            .split(area)
    };
    let left = Line::from(vec![
        Span::styled(format!("Zellij ({ZELLIJ_SESSION})"), zstyle(TEXT, BAR_BG)),
        zsep(),
        Span::styled(" Tab #1 ", zbold(BAR_BG, GREEN)),
        zsep(),
    ]);
    frame.render_widget(Paragraph::new(left).style(zbg(BAR_BG)), chunks[0]);

    if app.panes.len() > 1 && area.width >= 120 {
        let right = Line::from(vec![
            Span::styled("Alt <[]>", zstyle(ORANGE, BAR_BG)),
            zsep(),
            Span::styled(" BASE ", zstyle(TEXT, BASE_BG)),
            zsep(),
        ]);
        frame.render_widget(
            Paragraph::new(right)
                .style(zbg(BAR_BG))
                .alignment(Alignment::Right),
            chunks[1],
        );
    }
}

fn draw_layout(frame: &mut Frame<'_>, area: Rect, app: &App, node: &PaneNode) {
    match node {
        PaneNode::Leaf(index) => draw_pane(frame, area, app, *index),
        PaneNode::Split {
            axis,
            first,
            second,
        } => {
            let direction = match axis {
                SplitAxis::Horizontal => Direction::Horizontal,
                SplitAxis::Vertical => Direction::Vertical,
            };
            let chunks = Layout::default()
                .direction(direction)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            draw_layout(frame, chunks[0], app, first);
            draw_layout(frame, chunks[1], app, second);
        }
    }
}

fn draw_pane(frame: &mut Frame<'_>, area: Rect, app: &App, index: usize) {
    let active = app.active == index;
    let hovered = app.hover == Some(index);
    let border = if active {
        GREEN
    } else if hovered {
        ORANGE
    } else {
        Color::Black
    };
    let bottom_title = if hovered && !active {
        Some(Line::from(Span::styled(
            " Alt <click> - group, Alt <Right-Click> - ungroup all ",
            zstyle(ORANGE, PANE_BG),
        )))
    } else if active && app.panes.len() > 1 {
        Some(Line::from(Span::styled(
            " Ctrl <MouseScroll> or <drag borders> to resize ",
            zstyle(GREEN, PANE_BG),
        )))
    } else {
        None
    };

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(zstyle(border, PANE_BG))
        .style(zstyle(TEXT, PANE_BG))
        .title(Span::styled(
            app.panes
                .get(index)
                .map(|_| app.pane_title.as_str())
                .unwrap_or(" "),
            zstyle(border, PANE_BG),
        ));
    if let Some(title) = bottom_title {
        block = block.title_bottom(title);
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(pane) = app.panes.get(index) else {
        return;
    };
    let widget = PaneWidget::new(pane.driver.state()).base_style(zstyle(TEXT, PANE_BG));
    frame.render_widget(widget, inner);
}

fn draw_bottom_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    frame.render_widget(Paragraph::new("").style(zbg(BAR_BG)), area);

    let left = menu_line(
        Span::styled("Ctrl +", zstyle(TEXT, BAR_BG)),
        &[
            ("g", "LOCK", false),
            ("p", "PANE", app.awaiting_prefix),
            ("t", "TAB", false),
            ("n", "RESIZE", false),
            ("h", "MOVE", false),
            ("s", "SEARCH", false),
            ("o", "SESSION", false),
            ("q", "QUIT", false),
        ],
    );

    let right = if app.panes.len() > 1 && area.width >= 140 {
        let mut right_spans = vec![Span::styled("Alt +", zstyle(ORANGE, BAR_BG))];
        push_key_tags(&mut right_spans, &[("n", "New Pane", false)]);
        right_spans.push(Span::styled(
            " <←↓↑→> Change Focus ",
            zstyle(TAG_FG, TAG_BG),
        ));
        push_key_tags(
            &mut right_spans,
            &[("+-", "Resize", false), ("f", "Floating", false)],
        );
        Some(Line::from(right_spans))
    } else if app.panes.len() == 1 && area.width >= 120 {
        Some(menu_line(
            Span::styled("Alt +", zstyle(ORANGE, BAR_BG)),
            &[("n", "New Pane", false), ("f", "Floating", false)],
        ))
    } else {
        None
    };

    let right_width = right
        .as_ref()
        .map(|line| {
            let measured = u16::try_from(line.width())
                .unwrap_or(u16::MAX)
                .min(area.width);
            let visual_floor = if app.panes.len() > 1 { 108 } else { 42 };
            measured.max(visual_floor).min(area.width)
        })
        .unwrap_or(0);
    let left_area = Rect::new(
        area.x,
        area.y,
        area.width.saturating_sub(right_width),
        area.height,
    );
    frame.render_widget(Paragraph::new(left).style(zbg(BAR_BG)), left_area);

    if let Some(right) = right {
        let right_area = Rect::new(
            area.x + area.width.saturating_sub(right_width),
            area.y,
            right_width,
            area.height,
        );
        frame.render_widget(Paragraph::new(right).style(zbg(BAR_BG)), right_area);
    }
}

fn zsep() -> Span<'static> {
    Span::styled(" Ξ", zstyle(MUTED, BAR_BG))
}

fn zstyle(fg: Color, bg: Color) -> Style {
    Style::default().fg(fg).bg(bg)
}

fn zbg(bg: Color) -> Style {
    Style::default().bg(bg)
}

fn zbold(fg: Color, bg: Color) -> Style {
    zstyle(fg, bg).add_modifier(Modifier::BOLD)
}

fn push_sep(spans: &mut Vec<Span<'static>>) {
    spans.push(zsep());
}

fn menu_line(prefix: Span<'static>, tags: &[(&'static str, &'static str, bool)]) -> Line<'static> {
    let mut spans = vec![prefix];
    push_key_tags(&mut spans, tags);
    Line::from(spans)
}

fn push_key_tags(spans: &mut Vec<Span<'static>>, tags: &[(&'static str, &'static str, bool)]) {
    for &(key, label, active) in tags {
        push_sep(spans);
        push_key_tag(spans, key, label, active);
    }
    push_sep(spans);
}

fn push_key_tag(
    spans: &mut Vec<Span<'static>>,
    key: &'static str,
    label: &'static str,
    active: bool,
) {
    let style = if active {
        zbold(BAR_BG, GREEN)
    } else {
        zstyle(TAG_FG, TAG_BG)
    };
    let key_style = if active {
        zbold(BAR_BG, GREEN)
    } else {
        zbold(KEY_RED, TAG_BG)
    };

    spans.push(Span::styled(" <", style));
    spans.push(Span::styled(key, key_style));
    spans.push(Span::styled(format!("> {label} "), style));
}

fn replace_leaf(node: &mut PaneNode, target: usize, replacement: PaneNode) -> bool {
    match node {
        PaneNode::Leaf(index) if *index == target => {
            *node = replacement;
            true
        }
        PaneNode::Leaf(_) => false,
        PaneNode::Split { first, second, .. } => {
            if replace_leaf(first, target, replacement.clone()) {
                true
            } else {
                replace_leaf(second, target, replacement)
            }
        }
    }
}

fn layout_for_existing_panes(count: usize) -> PaneNode {
    match count {
        0 | 1 => PaneNode::Leaf(0),
        2 => PaneNode::Split {
            axis: SplitAxis::Horizontal,
            first: Box::new(PaneNode::Leaf(0)),
            second: Box::new(PaneNode::Leaf(1)),
        },
        3 => PaneNode::Split {
            axis: SplitAxis::Horizontal,
            first: Box::new(PaneNode::Leaf(0)),
            second: Box::new(PaneNode::Split {
                axis: SplitAxis::Vertical,
                first: Box::new(PaneNode::Leaf(1)),
                second: Box::new(PaneNode::Leaf(2)),
            }),
        },
        _ => (1..count).fold(PaneNode::Leaf(0), |layout, index| PaneNode::Split {
            axis: SplitAxis::Horizontal,
            first: Box::new(layout),
            second: Box::new(PaneNode::Leaf(index)),
        }),
    }
}

fn hit_test(node: &PaneNode, area: Rect, column: u16, row: u16) -> Option<usize> {
    if !rect_contains(area, column, row) {
        return None;
    }
    match node {
        PaneNode::Leaf(index) => Some(*index),
        PaneNode::Split {
            axis,
            first,
            second,
        } => {
            let direction = match axis {
                SplitAxis::Horizontal => Direction::Horizontal,
                SplitAxis::Vertical => Direction::Vertical,
            };
            let chunks = Layout::default()
                .direction(direction)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            hit_test(first, chunks[0], column, row)
                .or_else(|| hit_test(second, chunks[1], column, row))
        }
    }
}

fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

async fn launch_shell(pane: &Pane, title: &str) -> Result<()> {
    pane.spawn(["bash", "-i"])
        .cwd(shell_work_dir())
        .kill_existing(true)
        .title(title)
        .keep_alive_on_exit(true)
        .await?;
    Ok(())
}

async fn split_shell(parent: &Pane, direction: SplitDirection, title: &str) -> Result<Pane> {
    Ok(parent
        .split_with(direction)
        .spawn(["bash", "-i"])
        .cwd(shell_work_dir())
        .title(title)
        .keep_alive_on_exit(true)
        .await?)
}

async fn find_demo_pane(rmux: Option<&Rmux>, title: &str) -> Result<Pane> {
    Ok(rmux
        .ok_or("rmux facade is not available")?
        .find_panes()
        .session(SESSION)
        .title(title)
        .one()
        .await?)
}

fn spawn_render_task(
    index: usize,
    pane: Pane,
    tx: UnboundedSender<RenderMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let Ok(mut stream) = pane.render_stream().await else {
            return;
        };
        while let Ok(Some(update)) = stream.next().await {
            if tx.send((index, update)).is_err() {
                break;
            }
        }
    })
}

fn shell_work_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| env::temp_dir())
}

fn zellij_pane_title() -> String {
    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_owned());
    let host = hostname().unwrap_or_else(|| "host".to_owned());
    let cwd = display_cwd(&shell_work_dir());
    format!(" {user}@{host}: {cwd} ")
}

fn display_cwd(path: &Path) -> String {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return path.display().to_string();
    };
    match path.strip_prefix(&home) {
        Ok(relative) if relative.as_os_str().is_empty() => "~".to_owned(),
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => path.display().to_string(),
    }
}

fn hostname() -> Option<String> {
    let output = Command::new("hostname").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let host = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!host.is_empty()).then_some(host)
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
    terminal::size().unwrap_or((220, 90))
}

fn check_commands() -> Result<()> {
    let mut missing = Vec::new();
    for command in ["bash"] {
        if !command_exists(command) {
            missing.push(command);
        }
    }
    if !command_exists("rmux") {
        missing.push("rmux");
    }
    if missing.is_empty() {
        println!("rmux and bash are available");
        Ok(())
    } else {
        Err(format!("missing commands in PATH: {}", missing.join(", ")).into())
    }
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
        .arg(format!("command -v {command}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn cleanup() -> Result<()> {
    let rmux = demo_rmux_builder()?
        .default_timeout(Duration::from_secs(2))
        .connect_or_start()
        .await?;
    if let Ok(session) = rmux.session(SessionName::new(SESSION)?).await {
        let _ = session.kill().await;
    }
    let _ = rmux.shutdown().await;
    println!("mini-zellij rmux session cleaned up");
    Ok(())
}

fn demo_rmux_builder() -> Result<rmux_sdk::RmuxBuilder> {
    let builder = Rmux::builder();
    if env::consts::OS == "windows" {
        Ok(builder.windows_pipe(WINDOWS_PIPE))
    } else {
        Ok(builder.unix_socket(demo_socket_path()?))
    }
}

struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            cursor::Show
        );
    }
}
