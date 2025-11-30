use ansi_to_tui::IntoText as _;
use clap::{Arg, ArgAction, ArgGroup};
use ratatui::crossterm::event::{self, Event, KeyCode};
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

mod server;
use colored;

#[tokio::main]
async fn main() {
    colored::control::set_override(true);
    let args = clap::Command::new("rns-dns")
        .author("GimmeDataNow - Github account")
        .version("0.0.1")
        .about("A simple DNS server for the reticulum network stack")
        .arg(
            Arg::new("tui")
                .short('v')
                .long("visual")
                .help("TUI mode (Default)")
                .conflicts_with("cli")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cli")
                .short('c')
                .long("cli")
                .help("Prevent the visual mode from starting, used to start the smaller services")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("router")
                .short('r')
                .long("router")
                .help("Starts a reticulum network instance which routes trafic for the device")
                .requires("cli")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dns")
                .short('d')
                .long("dns")
                .help("Starts the dns server which responds to querys and link requests")
                .requires("cli")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("options")
                .short('o')
                .long("option")
                .help("Set critical options")
                .num_args(1..),
        )
        .arg(
            Arg::new("experimental")
                .short('1')
                .long("experimental")
                .help("experimental feature")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("experimental2")
                .short('2')
                .long("experimental2")
                .help("experimental feature")
                .action(ArgAction::SetTrue),
        )
        .group(
            ArgGroup::new("router or dns")
                .args(["router", "dns"])
                .required(false),
        )
        .get_matches();

    if args.get_flag("experimental") {
        server::start_server().await;
    }
    if args.get_flag("experimental2") {
        server::start_server().await;
    }
    if args.get_flag("cli") {
        log::info!("You have selected cmd mode");
        let options: Vec<_> = args
            .get_many::<String>("options")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default();
        log::trace!("-o is set to : <{}>", options.concat());

        if args.get_flag("router") {
            log::info!("Router is now starting");
            // server::start_server().await;
        }
    } else {
        log::info!("You have selected visual mode");
        visual_mode().await.unwrap();
        // visual_mode_old().await.unwrap();
    }
}

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
    scroll_top: usize, // index of the line at the top of view
    follow: bool,      // tail mode enabled
}
impl Default for LogViewState {
    fn default() -> Self {
        LogViewState {
            scroll_top: 0,
            follow: true,
        }
    }
}

// async fn visual_mode_old() -> Result<(), Box<dyn std::error::Error>> {
//     // Terminal setup
//     ratatui::crossterm::terminal::enable_raw_mode()?;
//     let stdout = io::stdout();
//     let backend = CrosstermBackend::new(stdout);
//     let mut terminal = Terminal::new(backend)?;
//     execute!(
//         terminal.backend_mut(),
//         EnterAlternateScreen,
//         terminal::Clear(terminal::ClearType::All)
//     )?;

//     // Channels for sending logs from tasks to main loop
//     let (tx, mut rx) = mpsc::unbounded_channel();

//     // List of processes
//     let mut processes: Vec<ProcessLog> = vec![
//         // ProcessLog {
//         // name: "router".to_string(),
//         // command: "cargo run -- -c --router".to_owned(),
//         // logs: vec![],
//         // },
//         ProcessLog {
//             name: "server".to_string(),
//             command: "cargo run -- -c --experimental".to_owned(),
//             logs: vec![],
//         },
//         ProcessLog {
//             name: "client".to_string(),
//             command: "cargo run -- -c --experimental2".to_owned(),
//             logs: vec![],
//         },
//         ProcessLog {
//             name: "ping loop".to_string(),
//             command: "ping 127.0.0.1".to_owned(),
//             logs: vec![],
//         },
//     ];

//     // Spawn async tasks for each process
//     for proc in &processes {
//         spawn_process(proc.name.clone(), proc.command.clone(), tx.clone());
//     }

//     let mut selected = 0;

//     loop {
//         // Drain channel for new logs
//         while let Ok((name, line)) = rx.try_recv() {
//             if let Some(p) = processes.iter_mut().find(|p| p.name == name) {
//                 p.logs.push(line);
//                 if p.logs.len() > 1000 {
//                     p.logs.remove(0);
//                 } // keep buffer size fixed
//             }
//         }

//         // Draw UI
//         terminal.draw(|f| {
//             let size = f.area();
//             let main_chunks = Layout::default()
//                 .direction(Direction::Horizontal)
//                 .constraints([Constraint::Percentage(20), Constraint::Min(10)].as_ref())
//                 .split(size);

//             let left_chunks = Layout::default()
//                 .direction(Direction::Vertical)
//                 .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
//                 .split(main_chunks[0]);

//             // Process list
//             let items: Vec<ListItem> = processes
//                 .iter()
//                 .map(|p| ListItem::new(p.name.clone()))
//                 .collect();
//             let mut state = ratatui::widgets::ListState::default();
//             state.select(Some(selected));

//             let process_list = List::new(items.clone()).block(
//                 Block::default()
//                     .borders(Borders::ALL)
//                     .title("Processes")
//                     .border_type(ratatui::widgets::BorderType::Rounded),
//             );
//             f.render_stateful_widget(process_list, left_chunks[0], &mut state);

//             let process_list = List::new(items).block(
//                 Block::default()
//                     .borders(Borders::ALL)
//                     .title("Execute Command")
//                     .border_type(ratatui::widgets::BorderType::Rounded),
//             );
//             f.render_stateful_widget(process_list, left_chunks[1], &mut state);

//             let raw_text = processes[selected].logs.join("\n");
//             let parsed = raw_text.into_text().unwrap();

//             let logs = Paragraph::new(parsed).block(
//                 Block::default()
//                     .borders(Borders::ALL)
//                     .title("Logs")
//                     .border_type(ratatui::widgets::BorderType::Rounded),
//             );

//             f.render_widget(logs, main_chunks[1]);
//         })?;

//         // Input handling
//         if event::poll(std::time::Duration::from_millis(100))? {
//             if let Event::Key(key) = event::read()? {
//                 match key.code {
//                     KeyCode::Up => {
//                         if selected > 0 {
//                             selected -= 1;
//                         }
//                     }
//                     KeyCode::Down => {
//                         if selected < processes.len() - 1 {
//                             selected += 1;
//                         }
//                     }
//                     KeyCode::Char('q') => break,
//                     _ => {}
//                 }
//             }
//         }
//     }
//     execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
//     terminal::disable_raw_mode()?;

//     Ok(())
// }

fn spawn_process(name: String, command: String, sender: mpsc::UnboundedSender<(String, String)>) {
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

#[allow(dead_code)]
async fn visual_mode() -> Result<(), Box<dyn std::error::Error>> {
    ratatui::crossterm::terminal::enable_raw_mode()?;
    // Terminal setup
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
        ProcessLog {
            name: "server".to_string(),
            command: "cargo run -- -c --experimental".to_owned(),
            logs: VecDeque::with_capacity(10000),
            scroll: 0,
            tail: true,
            filter: None,
        },
        ProcessLog {
            name: "client".to_string(),
            command: "cargo run -- -c --experimental2".to_owned(),
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
            if let Some((idx, _)) = processes.iter().enumerate().find(|(_, p)| p.name == name) {
                processes[idx].logs.push(line);
                // cap buffer
                let log_len = processes[idx].logs.len();
                if log_len > 10000 {
                    let remove_count = log_len - 10000;
                    processes[idx].logs.drain(0..remove_count);
                }
                // if processes[idx].logs.len() > 10000 {
                //     // drop oldest
                //     processes[idx]
                //         .logs
                //         .drain(0..(processes[idx].logs.len() - 10000));
                // }
                // If follow is enabled for this process, update scroll to show bottom.
                if views[idx].follow {
                    // We'll adjust scroll during draw based on view height.
                    // keep scroll_top as a sentinel (use 0 to indicate "stick to bottom" isn't reliable),
                    // but set to a large value to indicate follow-enabled
                    // -> we'll implement follow by setting scroll_top after measuring view height.
                }
            }
        }

        // Draw UI
        terminal.draw(|f| {
            let size = f.area();

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Min(10)].as_ref())
                .split(size);

            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(main_chunks[0]);

            // Process list
            let items: Vec<ListItem> = processes
                .iter()
                .map(|p| ListItem::new(p.name.clone()))
                .collect();
            list_state.select(Some(selected));

            let process_list = List::new(items.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Processes")
                    .border_type(ratatui::widgets::BorderType::Rounded),
            );
            f.render_stateful_widget(process_list, left_chunks[0], &mut list_state);

            let process_list2 = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Execute Command")
                    .border_type(ratatui::widgets::BorderType::Rounded),
            );
            f.render_stateful_widget(process_list2, left_chunks[1], &mut list_state);

            // Build filtered lines for selected process
            let all_lines = &processes[selected].logs;
            // Apply level filter and textual filter (raw, on stripped ANSI)
            let mut filtered: Vec<String> = Vec::with_capacity(all_lines.len());
            for ln in all_lines.iter() {
                // check level
                if !matches_level_filter(ln, level_filter) {
                    continue;
                }
                // check text filter (match against stripped ANSI)
                let plain = strip_ansi(ln);
                if !filter_input.is_empty() {
                    if !plain.to_lowercase().contains(&filter_input.to_lowercase()) {
                        continue;
                    }
                }
                filtered.push(ln.clone());
            }

            // compute available height for logs area (minus borders)
            let logs_area = main_chunks[1];
            // inner height approximation: subtract 2 for block borders/title
            let view_height = if logs_area.height >= 2 {
                (logs_area.height - 2) as usize
            } else {
                0usize
            };

            // Update follow behavior and scroll_top if follow is enabled
            if views[selected].follow {
                // set scroll_top such that bottom is visible
                if filtered.len() > view_height {
                    views[selected].scroll_top = filtered.len().saturating_sub(view_height);
                } else {
                    views[selected].scroll_top = 0;
                }
            } else {
                // ensure scroll_top within range
                if filtered.len() <= view_height {
                    views[selected].scroll_top = 0;
                } else {
                    views[selected].scroll_top = std::cmp::min(
                        views[selected].scroll_top,
                        filtered.len().saturating_sub(view_height),
                    );
                }
            }

            // Build Text by parsing ANSI for the visible window only (improves perf)
            let start = views[selected].scroll_top;
            let end = std::cmp::min(start + view_height.max(1), filtered.len());
            // Join only visible lines with newline so parse_ansi retains line breaks
            let visible_slice = if filtered.is_empty() {
                "".to_string()
            } else {
                filtered[start..end].join("\n")
            };
            // let parsed = parse_ansi(&visible_slice);
            let parsed = visible_slice.into_text().unwrap();

            // Title shows current filters
            let title = format!(
                "Logs [{}] Filter:'{}' Level:{} Tail:{}",
                processes[selected].name,
                filter_input,
                level_filter.as_str(),
                if views[selected].follow { "ON" } else { "OFF" }
            );

            let logs_block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_type(ratatui::widgets::BorderType::Rounded);

            // render logs (without scrollbar first)
            let paragraph = Paragraph::new(parsed).block(logs_block.clone());
            // We will reserve one column on the right for a scrollbar; create an inner area for text
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                .split(main_chunks[1]);

            f.render_widget(paragraph, chunks[0]);

            // Draw a simple scrollbar in chunks[1]
            // scrollbar height equals available height inside block
            let total_lines = filtered.len();
            let bar_height = if chunks[1].height >= 2 {
                (chunks[1].height - 2) as usize
            } else {
                0usize
            };

            // build scrollbar lines
            let mut bar_lines: Vec<String> = Vec::new();
            if total_lines == 0 || bar_height == 0 {
                for _ in 0..bar_height {
                    bar_lines.push(" ".to_string());
                }
            } else {
                // compute thumb size and position
                let ratio = (view_height as f64) / (total_lines as f64);
                let thumb_size = std::cmp::max(1, (ratio * (bar_height as f64)).round() as usize);
                let max_top = total_lines.saturating_sub(view_height);
                let pos = if max_top == 0 {
                    0usize
                } else {
                    // position fraction of scroll_top / max_top
                    let frac = (views[selected].scroll_top as f64) / (max_top as f64);
                    (frac * ((bar_height - thumb_size) as f64)).round() as usize
                };
                for i in 0..bar_height {
                    if i >= pos && i < pos + thumb_size {
                        bar_lines.push("█".to_string());
                    } else {
                        bar_lines.push(" ".to_string());
                    }
                }
            }

            let bar_paragraph = Paragraph::new(bar_lines.join("\n"))
                .block(Block::default().borders(Borders::ALL).title("Scroll"));
            f.render_widget(bar_paragraph, chunks[1]);
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
                            let view_h = views[selected].scroll_top.saturating_sub(1);
                            // prefer to move up by visible height if possible
                            let step = 10usize; // fallback step when view height unknown
                            views[selected].follow = false; // manual scroll pauses tail
                            views[selected].scroll_top =
                                views[selected].scroll_top.saturating_sub(step);
                        }
                        KeyCode::PageDown => {
                            // scroll down by page
                            views[selected].follow = false;
                            // naive step
                            let step = 10usize;
                            views[selected].scroll_top =
                                views[selected].scroll_top.saturating_add(step);
                        }
                        KeyCode::Home => {
                            views[selected].follow = false;
                            views[selected].scroll_top = 0;
                        }
                        KeyCode::End => {
                            // go to bottom and resume follow
                            // follow becomes true
                            views[selected].follow = true;
                        }
                        KeyCode::Char('t') => {
                            // toggle tail mode
                            views[selected].follow = !views[selected].follow;
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
