// use env_logger;
// use log;
use tokio::time;

use reticulum::destination::DestinationName;
use reticulum::destination::link::LinkEvent;
use reticulum::identity::PrivateIdentity;
use reticulum::iface::udp::UdpInterface;
//use reticulum::iface::tcp_server::TcpServer;
use reticulum::transport::{Transport, TransportConfig};

pub async fn start_server() {
    log::trace!("");
    log::info!("Starting Reticlum Server");
    // This should generally be OsRng unless there is some good reason to keep using the same identity.
    // for example: the current idea for the dns server would require authorities to keep the same identity.
    let private_id = PrivateIdentity::new_from_name("test-server");

    // the label "server" is entirely cosmetic and does not affect the functionality in any way.
    let mut transport = Transport::new(TransportConfig::new("server", &private_id, true));
    let destination = transport
        .add_destination(
            private_id.clone(),
            // this destination will only be able to communicate with other destination that share this exact
            // destination name.
            // The first argument is repersentative of an endpoint.
            // The second argument is the application space/virtual network the destination is part of.
            DestinationName::new("rns-dns", "infra"),
        )
        .await;
    // the destination hash is used for routing
    let destination_hash = destination.lock().await.desc.address_hash;
    log::info!("DESTINATION: {destination_hash}");
    let address_hash = transport.iface_manager().lock().await.spawn(
        UdpInterface::new("0.0.0.0:4243", Some("127.0.0.1:4242")),
        UdpInterface::spawn,
    );
    /*
    let address_hash = transport.iface_manager().lock().await.spawn (
      TcpServer::new("0.0.0.0:4242", transport.iface_manager()),
      TcpServer::spawn);
    */
    log::info!("ADDRESS: {}", address_hash);
    let announce_loop = async || loop {
        log::info!("SEND ANNOUNCE {}", destination_hash);
        transport.send_announce(&destination, None).await;
        time::sleep(time::Duration::from_secs(2)).await;
    };
    let out_event_loop = async || {
        let mut out_link_events = transport.out_link_events();
        while let Ok(link_event) = out_link_events.recv().await {
            match link_event.event {
                LinkEvent::Data(payload) => {
                    log::info!(
                        "OUT LINK PAYLOAD {} ({}): {}",
                        link_event.address_hash,
                        link_event.id,
                        str::from_utf8(payload.as_slice()).unwrap()
                    )
                }
                LinkEvent::Activated => {
                    log::info!(
                        "OUT LINK ACTIVATED {} ({})",
                        link_event.address_hash,
                        link_event.id
                    )
                }
                LinkEvent::Closed => {
                    log::info!(
                        "OUT LINK CLOSED {} ({})",
                        link_event.address_hash,
                        link_event.id
                    )
                }
            }
        }
        log::info!("OUT LINK LOOP EXIT")
    };
    let in_event_loop = async || {
        let mut next_ping = 0;
        let mut missed_pings = vec![];
        let mut in_link_events = transport.in_link_events();
        while let Ok(link_event) = in_link_events.recv().await {
            match link_event.event {
                LinkEvent::Data(payload) => {
                    let payload = str::from_utf8(payload.as_slice()).unwrap();
                    log::info!(
                        "IN LINK PAYLOAD {} ({}): {}",
                        link_event.address_hash,
                        link_event.id,
                        payload
                    );
                    log::info!("MISSED PINGS: {:?}", missed_pings);
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
                    log::info!(
                        "IN LINK ACTIVATED {} ({})",
                        link_event.address_hash,
                        link_event.id
                    )
                }
                LinkEvent::Closed => {
                    log::info!(
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
      _ = out_event_loop() => log::info!("out event loop exited"),
      _ = in_event_loop() => log::info!("in event loop exited"),
    }
}
