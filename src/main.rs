use clap::{Arg, ArgAction, ArgGroup};
use reticulum::destination::{DestinationName,SingleInputDestination};
use reticulum::identity::PrivateIdentity;
use reticulum::iface::tcp_client::TcpClient;
use reticulum::iface::tcp_server::TcpServer;
use reticulum::transport::{Transport, TransportConfig};
use shlex;
use std::io;
use std::time::Duration;
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend, crossterm::{execute, terminal::{self, EnterAlternateScreen, LeaveAlternateScreen}}, layout::{Constraint, Direction, Layout}, widgets::{Block, Borders, List, ListItem, Paragraph}, Terminal
};
use tokio::{io::{AsyncBufReadExt, BufReader}, sync::mpsc};
use rand_core::OsRng;

#[macro_use]
pub mod logging;
use logging::{logging_format, logging_function, LoggingLevel};

#[tokio::main]
async fn main() {
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
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("cli")
                .short('c')
                .long("cli")
                .help("Prevent the visual mode from starting, used to start the smaller services")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("router")
                .short('r')
                .long("router")
                .help("Starts a reticulum network instance which routes trafic for the device")
                .requires("cli")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("dns")
                .short('d')
                .long("dns")
                .help("Starts the dns server which responds to querys and link requests")
                .requires("cli")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("options")
                .short('o')
                .long("option")
                .help("Set critical options")
                .num_args(1..)
        )
        .group(
            ArgGroup::new("router or dns")
                .args(["router","dns"])
                .required(false)
        )
        .get_matches();
    

    if args.get_flag("cli") {
        info!("You have selected cmd mode");
        let options: Vec<_> = args
            .get_many::<String>("options")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(); 
        trace!("-o is set to : <{}>", options.concat());

        if args.get_flag("router") {
            info!("Router is now starting");
            routing_node_service(None, None, "router".to_owned(), None, None).await.unwrap();
            // 202.61.243.41
            // target_port = 4965
        }

    } else {
        info!("You have selected visual mode");
        visual_mode().await.unwrap();
    }
}

struct ProcessLog {
    name: String,
    command: String,
    logs: Vec<String>,
}

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
        ProcessLog { name: "prog1".to_string(), command: "cargo run -- -c -r".to_owned(), logs: vec![] },
        ProcessLog { name: "prog2".to_string(), command: "echo 'H'".to_owned(), logs: vec![] },
    ];

    // Spawn async tasks for each process
    for proc in &processes {
        spawn_process(proc.name.clone(), proc.command.clone(), tx.clone());
    }

    let mut selected = 0;

    loop {
        // Drain channel for new logs
        while let Ok((name, line)) = rx.try_recv() {
            if let Some(p) = processes.iter_mut().find(|p| p.name == name) {
                p.logs.push(line);
                if p.logs.len() > 1000 { p.logs.remove(0); } // keep buffer size fixed
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
            let items: Vec<ListItem> = processes.iter().map(|p| ListItem::new(p.name.clone())).collect();
            let mut state = ratatui::widgets::ListState::default();
            state.select(Some(selected));

            let process_list = List::new(items.clone())
                .block(Block::default()
                .borders(Borders::ALL)
                .title("Processes")
                .border_type(ratatui::widgets::BorderType::Rounded)
            );
            f.render_stateful_widget(process_list, left_chunks[0], &mut state);

            let process_list = List::new(items)
                .block(Block::default()
                .borders(Borders::ALL)
                .title("Execute Command")
                .border_type(ratatui::widgets::BorderType::Rounded)
            );
            f.render_stateful_widget(process_list, left_chunks[1], &mut state);


            // Log box
            let log_text = processes[selected].logs.join("\n");
            let logs = Paragraph::new(log_text)
                .block(Block::default()
                .borders(Borders::ALL)
                .title("Logs")
                .border_type(ratatui::widgets::BorderType::Rounded)
            );
            f.render_widget(logs, main_chunks[1]);
        })?;

        // Input handling
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => if selected > 0 { selected -= 1; },
                    KeyCode::Down => if selected < processes.len() - 1 { selected += 1; },
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}

fn spawn_process(name: String, command: String, sender: mpsc::UnboundedSender<(String, String)>) {

    tokio::spawn(async move {
        let parts = match shlex::split(&command) {
            Some(p) => p,
            None => {
                let error_msg = logging_format(LoggingLevel::Error, &format!("Failed to parse command line for <{}>", name));
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
                let error_msg = logging_format(LoggingLevel::Error, &format!("Failed to spawn process <{}>", name));
                // if the send panics then this is irrecoverable
                sender.send((name.clone(), error_msg.clone())).unwrap();
                // This should generally NEVER panic unless there is some serious issues with the environment
                panic!("{}", error_msg);
            });

        let stdout = child.stdout.take().expect("No stdout");
        let stderr = child.stderr.take().expect("No stderr");
        
        let name_clone = name.clone();
        let sender_clone = sender.clone();

        // let reader = BufReader::new(stdout);
        // let mut lines = reader.lines();

        // while let Ok(Some(line)) = lines.next_line().await {
            // sender.send((name.clone(), line)).unwrap();
        // }
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
                sender.send((name.clone(), format!("[STDERR] {}", line))).unwrap();
            }
        });
    });
}

pub async fn routing_node_service(
    sender: Option<mpsc::UnboundedSender<(String, String)>>,
    identity: Option<String>,
    name: String,
    remote_host: Option<String>,
    remote_port: Option<u16>
) -> Result<(), Box<dyn std::error::Error>> {

    info!("Starting routing node: {}", name);

    // create ident
    let ident = PrivateIdentity::new_from_name(&name);
    // group all spawned services by this app under "rns-dns"
    let transport_config = TransportConfig::new("rns-dns", &ident, true);
    let transport = Transport::new(transport_config);

    // extract the values
    let fallback_host = "127.0.0.1".to_string();
    let remote_host = remote_host.as_ref().unwrap_or(&fallback_host);
    let remote_port = remote_port.unwrap_or(53317);

    let _ = transport.iface_manager().lock().await.spawn(
        TcpServer::new(format!("{}:{}", remote_port, remote_port), transport.iface_manager()),
        TcpServer::spawn,
    );
    
    info!("Routing node listening on {remote_host}:{remote_port}");

    let _ = transport.iface_manager().lock().await.spawn(
        TcpClient::new(format!("{}:{}", "reticulum.betweentheborders.com", "4242")),
        TcpClient::spawn,
    );

    // Subscribe to announcement events to discover the server.
    let re = transport.recv_announces();
    let mut announce_receiver = re.await;
    info!("Listening for server announcements...");

    let dest_name = DestinationName::new("rns-dns", "receiv-announce");
    let announce_dest = SingleInputDestination::new(ident, dest_name);

    loop {
        info!("3");
        let announcement = announce_dest.announce(OsRng, None).expect("failed to create an announce");
        info!("4");

        transport.send_packet(announcement).await;
        info!("sent");


        match announce_receiver.try_recv() {
            Ok(announce) => {
                let dest = announce.destination.lock().await;
            
                info!("Announce info is: {:?}", dest.desc.address_hash);

                // if the send panics then this is irrecoverable
                match &sender {
                    Some(sender) => {
                        let msg = logging_format(LoggingLevel::Info, &format!("Announce info is: {:?}", dest.desc.address_hash));
                        // sender.send((name.clone(), msg.clone())).unwrap();
                        if let Err(e) = sender.send((name.clone(), msg.clone())) {
                            error!("Failed to send log message: {:?}", e); // This will catch the panic
                        }
                    },
                    None => {
                        info!("Announce info is: {:?}", dest.desc.address_hash);
                    }
                }    
            },
            Err(_) => {}
        };

        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}
