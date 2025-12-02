use ansi_to_tui::IntoText as _;
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::style::Color;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListState;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        execute,
        terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use shlex;
use std::collections::VecDeque;
use std::io;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::mpsc,
};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UiMode {
    Normal,
    FilterInput,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LevelFilter {
    All,
    Trace,
    Info,
    Warn,
    Error,
    Fatal,
}

#[allow(dead_code)]
impl LevelFilter {
    fn next(self) -> Self {
        match self {
            LevelFilter::All => LevelFilter::Trace,
            LevelFilter::Trace => LevelFilter::Info,
            LevelFilter::Info => LevelFilter::Warn,
            LevelFilter::Warn => LevelFilter::Error,
            LevelFilter::Error => LevelFilter::Fatal,
            LevelFilter::Fatal => LevelFilter::All,
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            LevelFilter::All => "ALL",
            LevelFilter::Trace => "TRACE",
            LevelFilter::Info => "INFO",
            LevelFilter::Warn => "WARN",
            LevelFilter::Error => "ERROR",
            LevelFilter::Fatal => "FATAL",
        }
    }
}

#[allow(dead_code)]
struct ProcessLog {
    name: String,
    command: String,
    logs: VecDeque<String>,
    scroll: usize,
    tail: bool,
    filter: Option<String>,
}

/// Strip simple ANSI SGR sequences `\x1b[...m` so filtering/search works on raw text.
/// made by chatgpt
#[allow(dead_code)]
fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            // possible CSI
            if i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                i += 2;
                // skip until 'm' or end
                while i < bytes.len() && bytes[i] != b'm' {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'm' {
                    i += 1;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Determine whether a line matches the level filter by searching for the textual level token
/// after stripping ANSI sequences. This assumes your logging_format prints the level as
/// one of TRACE/INFO/WARN/ERROR/FATAL somewhere in the line.
#[allow(dead_code)]
fn matches_level_filter(line: &str, lvl: LevelFilter) -> bool {
    if lvl == LevelFilter::All {
        return true;
    }
    let plain = strip_ansi(line);
    // try a simple word match
    plain.split_whitespace().any(|tok| {
        tok.eq_ignore_ascii_case(LevelFilter::Trace.as_str())
            || tok.eq_ignore_ascii_case(LevelFilter::Info.as_str())
            || tok.eq_ignore_ascii_case(LevelFilter::Warn.as_str())
            || tok.eq_ignore_ascii_case(LevelFilter::Error.as_str())
            || tok.eq_ignore_ascii_case(LevelFilter::Fatal.as_str())
    }) && plain.contains(lvl.as_str())
        || plain.contains(lvl.as_str())
}

/// The UI state per-process for scroll & follow behaviour
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct LogViewState {
    scroll: usize, // index of the line at the top of view
    tail: bool,    // tail mode enabled
}
impl Default for LogViewState {
    fn default() -> Self {
        LogViewState {
            scroll: 0,
            tail: true,
        }
    }
}

pub fn spawn_process(
    name: String,
    command: String,
    sender: mpsc::UnboundedSender<(String, String)>,
) {
    tokio::spawn(async move {
        let parts = match shlex::split(&command) {
            Some(p) => p,
            None => {
                let error_msg = log::logging_format(
                    log::LoggingLevel::Error,
                    &format!("Failed to parse command line for <{}>", name),
                );
                // if the send panics then this is irrecoverable
                sender.send((name, error_msg)).unwrap();
                return;
            }
        };

        let (executable, args) = (&parts[0], &parts[1..]);

        let mut command_builder = tokio::process::Command::new(executable);
        command_builder.args(args);

        let mut child = command_builder
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap_or_else(|_| {
                let error_msg = log::logging_format(
                    log::LoggingLevel::Error,
                    &format!("Failed to spawn process <{}>", name),
                );
                // if the send panics then this is irrecoverable
                sender.send((name.clone(), error_msg.clone())).unwrap();
                // This should generally NEVER panic unless there is some serious issues with the environment
                panic!("{}", error_msg);
            });

        let stdout = child.stdout.take().expect("No stdout");
        let stderr = child.stderr.take().expect("No stderr");

        let name_clone = name.clone();
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                sender_clone.send((name_clone.clone(), line)).unwrap();
            }
        });

        // Read stderr in another task
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                sender
                    .send((name.clone(), format!("[STDERR] {}", line)))
                    .unwrap();
            }
        });
    });
}

/// Returns a Vec<String> of lines that pass the level & text filter.
/// IMPORTANT: returns the original strings (including ANSI sequences).
fn get_visible_logs(
    proc: &ProcessLog,
    level_filter: LevelFilter,
    text_filter: &str,
    strip_ansi_toggle: bool, // NEW toggle
) -> Vec<String> {
    let mut out = Vec::new();
    let search = text_filter.to_lowercase();

    for ln in proc.logs.iter() {
        // 1) Level filter (use stripped text for check)
        if !matches_level_filter(ln, level_filter) {
            continue;
        }

        // 2) Text filtering (case-insensitive), checked against stripped text
        if !search.is_empty() {
            let plain = strip_ansi(ln);
            if !plain.to_lowercase().contains(&search) {
                continue;
            }
        }

        // 3) Output either:
        //    - original line with ANSI codes, or
        //    - stripped text
        if strip_ansi_toggle {
            out.push(strip_ansi(ln));
        } else {
            out.push(ln.clone());
        }
    }

    out
}

fn copy_logs_to_clipboard(
    clipboad: &mut ClipboardContext,
    lines: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // let mut ctx = ClipboardContext::new().unwrap();
    // let mut clipboard = arboard::Clipboard::new()?;
    let text = lines.join("\n");
    clipboad.set_contents(text).unwrap();
    // log::info!("setting");
    // clipboard.set_text(text)?;
    Ok(())
}

pub async fn tui() -> Result<(), Box<dyn std::error::Error>> {
    // allow for copy
    // Terminal setup
    let mut ctx = ClipboardContext::new().unwrap();
    ratatui::crossterm::terminal::enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All)
    )?;

    // Channels for sending logs from tasks to main loop
    let (tx, mut rx) = mpsc::unbounded_channel();

    // List of processes
    let mut processes: Vec<ProcessLog> = vec![
        // ProcessLog {
        // name: "server".to_string(),
        // command: "cargo run -- -c --experimental".to_owned(),
        // logs: VecDeque::with_capacity(10000),
        // scroll: 0,
        // tail: true,
        // filter: None,
        // },
        ProcessLog {
            name: "client".to_string(),
            command: "cargo run -- -c --experimental2".to_owned(),
            logs: VecDeque::with_capacity(10000),
            scroll: 0,
            tail: true,
            filter: None,
        },
        ProcessLog {
            name: "router".to_string(),
            command: "cargo run -- -c --experimental3".to_owned(),
            logs: VecDeque::with_capacity(10000),
            scroll: 0,
            tail: true,
            filter: None,
        },
        ProcessLog {
            name: "ping loop".to_string(),
            command: "ping 127.0.0.1".to_owned(),
            logs: VecDeque::with_capacity(10000),
            scroll: 0,
            tail: true,
            filter: None,
        },
    ];

    // Per-process view state
    let mut views: Vec<LogViewState> = vec![LogViewState::default(); processes.len()];

    // Spawn async tasks for each process
    for proc in &processes {
        spawn_process(proc.name.clone(), proc.command.clone(), tx.clone());
    }

    let mut selected = 0usize;
    let mut ui_mode = UiMode::Normal;
    let mut filter_input = String::new();
    let mut level_filter = LevelFilter::All;

    // For List state (left list)
    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    loop {
        // Drain channel for new logs
        while let Ok((name, line)) = rx.try_recv() {
            if let Some(p) = processes.iter_mut().find(|p| p.name == name) {
                p.logs.push_back(line);

                if p.logs.len() > 10000 {
                    p.logs.pop_front();
                }

                // auto-follow mode: reset to bottom
                if p.tail {
                    p.scroll = 0;
                }
            };
        }

        // Draw UI
        terminal.draw(|f| {
            let size = f.area();

            // split into left and right area
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Min(10)].as_ref())
                .split(size);

            // split left area
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(main_chunks[0]);

            let items: Vec<ListItem> = processes
                .iter()
                .enumerate()
                .map(|(_i, p)| ListItem::new(p.name.clone()))
                .collect();

            list_state.select(Some(selected));
            let proc_list = List::new(items.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue))
                        .title(Line::from(vec![
                            Span::styled("┐", Style::default()),
                            Span::styled("Process List", Style::default().fg(Color::White)),
                            Span::styled("┌", Style::default()),
                        ]))
                        .title_bottom(
                            Line::from(vec![
                                Span::styled("┘", Style::default()),
                                Span::styled("↑", Style::default().fg(Color::LightRed)),
                                Span::styled(" select ", Style::default().fg(Color::White)),
                                Span::styled("↓", Style::default().fg(Color::LightRed)),
                                Span::styled("└", Style::default()),
                            ])
                            .right_aligned(),
                        )
                        .border_type(ratatui::widgets::BorderType::Rounded),
                )
                .highlight_style(Style::default().reversed().fg(Color::Blue))
                .highlight_symbol(" ▶ ");
            f.render_stateful_widget(proc_list, left_chunks[0], &mut list_state);

            let cmd_list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Execute Command")
                    .border_type(ratatui::widgets::BorderType::Rounded),
            );
            f.render_stateful_widget(cmd_list, left_chunks[1], &mut list_state);

            // ---- LOG AREA ----
            let view_height = main_chunks[1].height.saturating_sub(2) as usize;

            // Build filtered AND ANSI-preserving visible lines
            let visible =
                get_visible_logs(&processes[selected], level_filter, &filter_input, false);

            // scroll / tail logic (0 = bottom)
            let vs = &mut views[selected];
            if vs.tail {
                vs.scroll = 0;
            }

            let total = visible.len();
            let end = total.saturating_sub(vs.scroll);
            let start = end.saturating_sub(view_height);
            let slice: &[String] = if start < end && start < total && end <= total {
                &visible[start..end]
            } else {
                &[]
            };

            // join visible slice (keeps ANSI) and parse it into styled text for ratatui
            let joined = slice.join("\n");
            let parsed = joined.into_text().unwrap();

            let logs_block = Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![
                    Span::styled("┐", Style::default()),
                    Span::styled("Logs", Style::default().fg(Color::White)),
                    Span::styled("┌─┐", Style::default()),
                    Span::styled("Filter <", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}", filter_input),
                        Style::default().bold().fg(Color::White),
                    ),
                    Span::styled(">", Style::default().fg(Color::White)),
                    Span::styled("┌", Style::default()),
                    Span::styled("┐", Style::default()),
                    Span::styled("Level ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}", level_filter.as_str()),
                        Style::default()
                            .fg(match level_filter {
                                LevelFilter::All => Color::White,
                                LevelFilter::Trace => Color::Magenta,
                                LevelFilter::Info => Color::Blue,
                                LevelFilter::Warn => Color::Yellow,
                                LevelFilter::Error => Color::Red,
                                LevelFilter::Fatal => Color::Black,
                            })
                            .bg(match level_filter {
                                LevelFilter::Fatal => Color::Red,
                                _ => Color::default(),
                            }),
                    ),
                    Span::styled("┌", Style::default()),
                    Span::styled("┐", Style::default()),
                    Span::styled("Tail ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}", vs.tail),
                        Style::default().fg(match vs.tail {
                            true => Color::Blue,
                            false => Color::Red,
                        }),
                    ),
                    Span::styled("┌", Style::default()),
                ]))
                .title_bottom(
                    Line::from(vec![
                        Span::styled("┘", Style::default()),
                        Span::styled("Filter ", Style::default().fg(Color::White)),
                        Span::styled("/", Style::default().fg(Color::LightRed)),
                        Span::styled("|", Style::default().fg(Color::White)),
                        Span::styled("esc", Style::default().fg(Color::LightRed)),
                        Span::styled("|", Style::default().fg(Color::White)),
                        Span::styled("󰌑", Style::default().fg(Color::LightRed)),
                        //
                        Span::styled("└", Style::default()),
                        Span::styled("┘", Style::default()),
                        //
                        Span::styled("Logging ", Style::default().fg(Color::White)),
                        Span::styled("l", Style::default().fg(Color::LightRed)),
                        //
                        Span::styled("└", Style::default()),
                        Span::styled("┘", Style::default()),
                        //
                        Span::styled("Tail ", Style::default().fg(Color::White)),
                        Span::styled("t", Style::default().fg(Color::LightRed)),
                        //
                        Span::styled("└", Style::default()),
                        Span::styled("┘", Style::default()),
                        //
                        Span::styled("Copy ", Style::default().fg(Color::White)),
                        Span::styled("c", Style::default().fg(Color::LightRed)),
                        Span::styled("└", Style::default()),
                    ])
                    .right_aligned(),
                )
                .border_type(ratatui::widgets::BorderType::Rounded);

            // Reserve a small column for scrollbar (optional)
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                .split(main_chunks[1]);

            let paragraph = Paragraph::new(parsed).block(logs_block.clone());
            f.render_widget(paragraph, columns[0]);
        })?;

        // Input handling
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match ui_mode {
                    UiMode::Normal => match key.code {
                        KeyCode::Up => {
                            if selected > 0 {
                                selected -= 1;
                                list_state.select(Some(selected));
                            }
                        }
                        KeyCode::Down => {
                            if selected < processes.len() - 1 {
                                selected += 1;
                                list_state.select(Some(selected));
                            }
                        }
                        KeyCode::PageUp => {
                            // scroll up by page
                            // let view_h = views[selected].scroll.saturating_sub(1);
                            // prefer to move up by visible height if possible
                            let step = 10usize; // fallback step when view height unknown
                            views[selected].tail = false; // manual scroll pauses tail
                            views[selected].scroll = views[selected].scroll.saturating_add(step);
                        }
                        KeyCode::PageDown => {
                            // scroll down by page
                            views[selected].tail = false;
                            // naive step
                            let step = 10usize;
                            views[selected].scroll = views[selected].scroll.saturating_sub(step);
                        }
                        KeyCode::Home => {
                            views[selected].tail = false;
                            views[selected].scroll = 0;
                        }
                        KeyCode::End => {
                            views[selected].tail = true;
                        }
                        KeyCode::Char('c') => {
                            let vis_logs = get_visible_logs(
                                &processes[selected],
                                level_filter,
                                &filter_input,
                                true,
                            );
                            terminal::disable_raw_mode()?;
                            copy_logs_to_clipboard(&mut ctx, &vis_logs)?;
                            terminal::enable_raw_mode()?;
                        }
                        KeyCode::Char('t') => {
                            // toggle tail mode
                            views[selected].tail = !views[selected].tail;
                        }
                        KeyCode::Char('/') => {
                            // enter filter input mode
                            ui_mode = UiMode::FilterInput;
                            filter_input.clear();
                        }
                        KeyCode::Char('l') => {
                            // cycle level filter
                            level_filter = level_filter.next();
                        }
                        KeyCode::Char('q') => {
                            // quit
                            break;
                        }
                        _ => {}
                    },
                    UiMode::FilterInput => match key.code {
                        KeyCode::Enter => {
                            // apply and exit filter input
                            ui_mode = UiMode::Normal;
                        }
                        KeyCode::Esc => {
                            // cancel filter input
                            filter_input.clear();
                            ui_mode = UiMode::Normal;
                        }
                        KeyCode::Backspace => {
                            filter_input.pop();
                        }
                        KeyCode::Char(c) => {
                            filter_input.push(c);
                        }
                        _ => {}
                    },
                }
            }
        }

        // small sleep to avoid busy loop
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // teardown
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
