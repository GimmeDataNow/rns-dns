use reticulum::iface::tcp_server::TcpServer;
use tokio::time;

use reticulum::destination::link::LinkEvent;
use reticulum::iface::udp::UdpInterface;
use reticulum::{destination::DestinationName, hash::AddressHash};
//use reticulum::iface::tcp_server::TcpServer;
use reticulum::transport::{Transport, TransportConfig};

use crate::types::{self, Connection};

/// The router that handles routing between nodes on the local network. May be connected to other nodes.
pub async fn start_server(
    node_settings: types::NodeSettings,
    destination_settings: types::DestinationConfig,
) {
    log::info!("Starting RNS-DNS");

    let private_id = node_settings.private_identity.extract();

    // the label "router" is entirely cosmetic and does not affect the functionality in any way.
    let mut transport = Transport::new(TransportConfig::new("server", &private_id, true));

    let mut address_hash: AddressHash = AddressHash::new_empty();
    let mut address_hashes: Vec<AddressHash> = Vec::new();

    // add each interface to the node
    for i in &node_settings.interfaces {
        match i {
            Connection::Tcp {
                local_host,
                local_port,
            } => {
                address_hash = transport.iface_manager().lock().await.spawn(
                    TcpServer::new(
                        &format!("{local_host}:{local_port}"),
                        transport.iface_manager(),
                    ),
                    TcpServer::spawn,
                );
                address_hashes.push(address_hash);
            }
            Connection::Udp {
                local_host,
                local_port,
                remote_host,
                remote_port,
            } => {
                address_hash = transport.iface_manager().lock().await.spawn(
                    UdpInterface::new(
                        format!("{local_host}:{local_port}"),
                        Some(format!("{remote_host}:{remote_port}")),
                    ),
                    UdpInterface::spawn,
                );
                address_hashes.push(address_hash);
            }
            _ => todo!(),
        };
        log::info!("New Node address registered: {}", address_hash);
    }

    {
        // these should be the same if the destination was generated using the same private identity
        // let public_key = destination.lock().await.identity.as_identity().public_key;
        let public_key = private_id.as_identity().public_key;
        let url =
            types::generate_node_url(&1, &address_hashes, &public_key, &node_settings.interfaces);
        let qr = qr2term::generate_qr_string(&url).unwrap();
        let mut qr_split: Vec<&str> = qr.split("\n").collect();
        qr_split.pop();
        log::info!("{url}");
        for n in qr_split {
            log::info!("{n}");
        }
    }

    {
        // these should be the same if the destination was generated using the same private identity
        // let public_key = destination.lock().await.identity.as_identity().public_key;
        let url = types::generate_destination_url(
            &1,
            &destination_settings.app_name,
            &destination_settings.application_space,
            &address_hashes,
        );
        let qr = qr2term::generate_qr_string(&url).unwrap();
        let mut qr_split: Vec<&str> = qr.split("\n").collect();
        qr_split.pop();
        log::info!("{url}");
        for n in qr_split {
            log::info!("{n}");
        }
    }

    // only if the destinations match will the link work
    let destination = transport
        .add_destination(
            private_id.clone(),
            DestinationName::new(
                &destination_settings.app_name,          // ~= endpoint
                &destination_settings.application_space, // ~= virtual network
            ),
        )
        .await;

    // the destination hash is used for routing
    let destination_hash = destination.lock().await.desc.address_hash;
    log::info!(
        "New destination {} for the application space {} available: {destination_hash}",
        destination_settings.app_name,
        destination_settings.application_space
    );

    let announce_loop = async || loop {
        log::trace!("SEND ANNOUNCE {}", destination_hash);
        transport.send_announce(&destination, None).await;
        time::sleep(time::Duration::from_secs(15)).await;
    };

    let in_event_loop = async || {
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

                    // link
                    let link_id = link_event.id;
                    let link = transport.find_in_link(&link_id).await.unwrap();
                    let link = link.lock().await;

                    // response
                    let pong = link
                        .data_packet(format!("server-response").as_bytes())
                        .unwrap();
                    drop(link);
                    // send
                    transport.send_packet(pong).await;
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
