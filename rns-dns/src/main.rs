use clap::{Arg, ArgAction, ArgGroup};
use colored;

use crate::types::{Connection, NodeSettings};

mod tui;

mod client;
mod router;
mod server;
mod types;
mod utilites;

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
            Arg::new("client")
                .short('t')
                .long("client")
                .help("Starts a test client")
                .requires("cli")
                .conflicts_with("router")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("router")
                .short('r')
                .long("router")
                .help("Starts a reticulum network instance which routes trafic for the device")
                .requires("cli")
                .conflicts_with("client")
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
                .requires("cli")
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
        .arg(
            Arg::new("experimental3")
                .short('3')
                .long("experimental3")
                .help("experimental feature")
                .action(ArgAction::SetTrue),
        )
        .group(
            ArgGroup::new("router or dns")
                .args(["router", "dns"])
                .required(false),
        )
        .get_matches();

    // if args.get_flag("experimental3") {
    //     server::start_server().await;
    // }
    // if args.get_flag("experimental2") {
    //     client::client().await;
    // }
    if args.get_flag("cli") {
        // let options: Vec<_> = args
        //     .get_many::<String>("options")
        //     .map(|vals| vals.cloned().collect())
        //     .unwrap_or_default();
        // log::trace!("-o is set to : <{}>", options.concat());
        if args.get_flag("client") {
            let udp = Connection::Udp {
                local_host: "0.0.0.0".to_string(),
                local_port: 4242,
                remote_host: "127.0.0.1".to_string(),
                remote_port: 4243,
            };
            let tcp = Connection::new_tcp("127.0.0.1".to_string(), 53317);
            let node_settings = NodeSettings::new(
                vec![tcp],
                types::PrivateIdentity::FromString("client-test".to_owned()),
            );
            let destination_config =
                types::DestinationConfig::new("node.config".to_owned(), "infra".to_owned());
            client::client(node_settings, destination_config).await;
        }

        if args.get_flag("router") {
            let udp = Connection::Udp {
                local_host: "0.0.0.0".to_string(),
                local_port: 4243,
                remote_host: "127.0.0.1".to_string(),
                remote_port: 4242,
            };
            let tcp = Connection::new_tcp("0.0.0.0".to_string(), 53317);
            let node_settings = NodeSettings::new(
                vec![udp, tcp],
                types::PrivateIdentity::FromString("router-test".to_owned()),
            );
            let destination_config =
                types::DestinationConfig::new("node.config".to_owned(), "infra".to_owned());
            router::start_router(node_settings, destination_config).await;
        }

        if args.get_flag("dns") {
            let udp = Connection::Udp {
                local_host: "0.0.0.0".to_string(),
                local_port: 4243,
                remote_host: "127.0.0.1".to_string(),
                remote_port: 4242,
            };
            let tcp = Connection::new_tcp("0.0.0.0".to_string(), 53317);
            let node_settings = NodeSettings::new(
                vec![udp, tcp],
                types::PrivateIdentity::FromString("router-test".to_owned()),
            );
            let destination_config =
                types::DestinationConfig::new("node.config".to_owned(), "infra".to_owned());
            server::node::start_server(node_settings, destination_config).await;
        }
    } else {
        log::info!("You have selected visual mode");
        tui::tui().await.unwrap();
    }
}
