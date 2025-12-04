use std::sync::Arc;

use reticulum::iface::tcp_client::TcpClient;
use tokio::sync::{self, Mutex};
use tokio::time;

use reticulum::destination::link::{Link, LinkEvent};
use reticulum::destination::{DestinationName, SingleInputDestination};
use reticulum::identity::PrivateIdentity;
use reticulum::iface::udp::UdpInterface;
//use reticulum::iface::tcp_client::TcpClient;
use reticulum::hash::AddressHash;
use reticulum::transport::{Transport, TransportConfig};

use crate::types;

pub async fn client(
    node_settings: types::NodeSettings,
    destination_settings: types::DestinationConfig,
) {
    log::info!("Reticulum test client");
    let private_id = node_settings.private_identity.extract();
    let transport = Transport::new(TransportConfig::new("client", &private_id, false));

    let mut address_hash: AddressHash = AddressHash::new_empty();
    let mut address_hashes: Vec<AddressHash> = Vec::new();

    for i in &node_settings.interfaces {
        match i {
            types::Connection::Tcp {
                local_host,
                local_port,
            } => {
                address_hash = transport.iface_manager().lock().await.spawn(
                    TcpClient::new(&format!("{local_host}:{local_port}")),
                    TcpClient::spawn,
                );
                address_hashes.push(address_hash);
            }
            types::Connection::Udp {
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

    /*
    let address_hash = transport.iface_manager().lock().await.spawn (
      TcpClient::new ("127.0.0.1:4242"), TcpClient::spawn);
    */
    let pings = Arc::new(Mutex::new(vec![]));
    let mut announce_recv = transport.recv_announces().await;
    let current_link: Arc<Mutex<Option<Arc<Mutex<Link>>>>> = Arc::new(Mutex::new(None));
    let mut link_loop = async || {
        while let Ok(announce) = announce_recv.recv().await {
            let destination = announce.destination.lock().await;
            log::trace!("GOT ANNOUNCE: {}", destination.desc.address_hash);
            let mut current_link = current_link.lock().await;
            if current_link.is_none() {
                let link = transport.link(destination.desc).await;
                log::trace!("SET LINK: {}", link.lock().await.id());
                *current_link = Some(link.clone());
            }
            drop(current_link);
        }
    };
    let out_event_loop = async || {
        let mut out_link_events = transport.out_link_events();
        loop {
            match out_link_events.recv().await {
                Ok(link_event) => match link_event.event {
                    LinkEvent::Data(payload) => {
                        let payload = str::from_utf8(payload.as_slice()).unwrap();
                        log::trace!("{}", payload);
                        // log::trace!(
                        //     "OUT LINK PAYLOAD {} ({}): {}",
                        //     link_event.address_hash,
                        //     link_event.id,
                        //     payload
                        // );
                        // if &payload[0..4] == "pong" {
                        //     let n = (&payload[5..]).parse::<u64>().unwrap();
                        //     let mut pings = pings.lock().await;
                        //     let index = pings.iter().position(|x| *x == n).unwrap();
                        //     pings.remove(index);
                        //     log::trace!("UNACKED PINGS: {pings:?}");
                        // } else {
                        //     log::error!("unreachable code");
                        //     // unreachable!()
                        // }
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
                },
                Err(err) => {
                    log::error!("out link error: {err:?}");
                    break;
                }
            }
        }
        log::info!("OUT LINK LOOP EXIT");
    };
    let in_event_loop = async || {
        let mut in_link_events = transport.in_link_events();
        loop {
            match in_link_events.recv().await {
                Ok(link_event) => match link_event.event {
                    LinkEvent::Data(payload) => {
                        log::info!(
                            "IN LINK PAYLOAD {} ({}): {}",
                            link_event.address_hash,
                            link_event.id,
                            str::from_utf8(payload.as_slice()).unwrap()
                        )
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
                },
                Err(err) => {
                    log::error!("out link error: {err:?}");
                    break;
                }
            }
        }
        log::info!("IN LINK LOOP EXIT");
    };
    let ping_loop = async || {
        let mut counter = 0;
        loop {
            if let Some(current_link) = current_link.lock().await.as_mut() {
                // if counter == 5 {
                //     let mut link = current_link.lock().await;
                //     log::info!("CLOSING LINK");
                //     link.close();
                //     let destination = link.destination().clone();
                //     drop(link);
                //     let link = transport.link(destination).await;
                //     log::info!("NEW LINK: {}", link.lock().await.id());
                //     *current_link = link;
                // }
                log::trace!("SEND PING {counter}");
                pings.lock().await.push(counter);
                let link = current_link.lock().await;
                let packet = link
                    .data_packet(format!("ping {counter}").as_bytes())
                    .unwrap();
                drop(link);
                transport.send_packet(packet).await;
                counter += 1;
            }
            time::sleep(time::Duration::from_secs(2)).await;
        }
    };
    tokio::select! {
      _ = link_loop() => log::info!("link loop exited"),
      _ = out_event_loop() => log::info!("out event loop exited"),
      _ = in_event_loop() => log::info!("in event loop exited"),
      _ = ping_loop() => log::info!("ping loop exited")
    }
}
