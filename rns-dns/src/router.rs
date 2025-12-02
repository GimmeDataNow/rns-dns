// use env_logger;
// use log;
use tokio::time;

use reticulum::destination::DestinationName;
use reticulum::destination::link::LinkEvent;
use reticulum::identity::PrivateIdentity;
use reticulum::iface::udp::UdpInterface;
//use reticulum::iface::tcp_server::TcpServer;
use rand_core::OsRng;
use reticulum::transport::{Transport, TransportConfig};

use crate::types;

///
/// The router that handles routing between nodes on the local network. May be connected to other nodes.
/// Node config
/// destination config for the config broadcast
pub async fn start_router(
    settings: types::NodeSettings,
    config_destination: types::DestinationConfig,
) {
    log::info!("Starting Reticlum Router");
    // This should generally be OsRng unless there is some good reason to keep using the same identity.
    let private_id = match settings.private_identity {
        types::PrivateIdentity::Rand => PrivateIdentity::new_from_rand(OsRng),
        types::PrivateIdentity::FromString(s) => PrivateIdentity::new_from_name(&s),
        types::PrivateIdentity::FromHexString(s) => PrivateIdentity::new_from_hex_string(&s)
            .expect("failed to convert hex string to private identity"),
    };

    // the label "router" is entirely cosmetic and does not affect the functionality in any way.
    // rebroadcast set to false
    let mut transport = Transport::new(TransportConfig::new("router", &private_id, true));

    let local_connection = match settings.local_connection {
        #[allow(unreachable_patterns)]
        Some(c) => match c {
            types::Connection::NormalInternet((ip, port)) => {
                format!("{ip}:{port}")
            }
            _ => todo!(),
        },
        None => format!("0.0.0.0:4243"),
    };

    let remote_connection = match settings.remote_connection {
        #[allow(unreachable_patterns)]
        Some(c) => match c {
            types::Connection::NormalInternet((ip, port)) => Some(format!("{ip}:{port}")),
            _ => todo!(),
        },
        None => None,
    };

    let address_hash = transport.iface_manager().lock().await.spawn(
        UdpInterface::new("0.0.0.0:4243", Some("127.0.0.1:4242")),
        UdpInterface::spawn,
    );
    log::info!("Node address is: {}", address_hash);

    // only if the destinations match will the link work
    let destination = transport
        .add_destination(
            private_id.clone(),
            // DestinationName::new(
            //     &config_destination.app_name,          // ~= endpoint
            //     &config_destination.application_space, // ~= virtual network
            // ),
            DestinationName::new("test-server", "app.1"),
        )
        .await;

    // the destination hash is used for routing
    let destination_hash = destination.lock().await.desc.address_hash;
    log::info!(
        "New destination {} for the application space {} available: {destination_hash}",
        config_destination.app_name,
        config_destination.application_space
    );
    let announce_loop = async || loop {
        log::trace!("SEND ANNOUNCE {}", destination_hash);
        transport.send_announce(&destination, None).await;
        time::sleep(time::Duration::from_secs(2)).await;
    };
    let in_event_loop = async || {
        let mut next_ping = 0;
        let mut missed_pings = vec![];
        let mut in_link_events = transport.in_link_events();
        while let Ok(link_event) = in_link_events.recv().await {
            match link_event.event {
                LinkEvent::Data(payload) => {
                    let payload = str::from_utf8(payload.as_slice()).unwrap();
                    log::trace!(
                        "IN LINK PAYLOAD {} ({}): {}",
                        link_event.address_hash,
                        link_event.id,
                        payload
                    );
                    log::trace!("MISSED PINGS: {:?}", missed_pings);
                    if &payload[0..4] == "ping" {
                        let n = (&payload[5..]).parse::<u64>().unwrap();
                        if n != next_ping {
                            while next_ping < n {
                                missed_pings.push(next_ping);
                                next_ping += 1;
                            }
                        }
                        next_ping = n + 1;
                        let link_id = link_event.id;
                        let link = transport.find_in_link(&link_id).await.unwrap();
                        let link = link.lock().await;
                        let pong = link.data_packet(format!("pong {n}").as_bytes()).unwrap();
                        drop(link);
                        transport.send_packet(pong).await;
                    } else {
                        unreachable!()
                    }
                }
                LinkEvent::Activated => {
                    log::trace!(
                        "IN LINK ACTIVATED {} ({})",
                        link_event.address_hash,
                        link_event.id
                    )
                }
                LinkEvent::Closed => {
                    log::trace!(
                        "IN LINK CLOSED {} ({})",
                        link_event.address_hash,
                        link_event.id
                    )
                }
            }
        }
        log::info!("IN LINK LOOP EXIT")
    };
    tokio::select! {
      _ = announce_loop() => log::info!("announce loop exited"),
      _ = in_event_loop() => log::info!("in event loop exited"),
    }
}
