use std::sync::Arc;

use env_logger;
use log;
use tokio::time;
use tokio::sync::{self, Mutex};

use reticulum::destination::{DestinationName, SingleInputDestination};
use reticulum::destination::link::{Link, LinkEvent};
use reticulum::identity::PrivateIdentity;
use reticulum::iface::udp::UdpInterface;
//use reticulum::iface::tcp_client::TcpClient;
use reticulum::transport::{Transport, TransportConfig};

#[tokio::main]
async fn main() {
  println!("reticulum test client");
  env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
    .init();
  let private_id = PrivateIdentity::new_from_name ("test-client");
  let _destination = {
    let destination = SingleInputDestination::new (
      private_id.clone(), DestinationName::new ("test-server", "app.1"));
    log::info!("DESTINATION: {}", destination.desc.address_hash);
    std::sync::Arc::new (sync::Mutex::new (destination))
  };
  let transport = Transport::new(TransportConfig::new ("client", &private_id, true));
  let address_hash = transport.iface_manager().lock().await.spawn (
    UdpInterface::new ("0.0.0.0:4242", Some ("127.0.0.1:4243")),
    UdpInterface::spawn);
  /*
  let address_hash = transport.iface_manager().lock().await.spawn (
    TcpClient::new ("127.0.0.1:4242"), TcpClient::spawn);
  */
  log::info!("ADDRESS: {}", address_hash);
  let pings = Arc::new(Mutex::new(vec![]));
  let mut announce_recv = transport.recv_announces().await;
  let current_link: Arc<Mutex<Option<Arc<Mutex<Link>>>>> = Arc::new(Mutex::new(None));
  let mut link_loop = async || {
    while let Ok(announce) = announce_recv.recv().await {
      let destination = announce.destination.lock().await;
      log::info!("GOT ANNOUNCE: {}", destination.desc.address_hash);
      let mut current_link = current_link.lock().await;
      if current_link.is_none() {
        let link = transport.link (destination.desc).await;
        log::info!("SET LINK: {}", link.lock().await.id());
        *current_link = Some (link.clone());
      }
      drop(current_link);
    }
  };
  let out_event_loop = async || {
    let mut out_link_events = transport.out_link_events();
    loop {
      match out_link_events.recv().await {
        Ok(link_event) => {
          match link_event.event {
            LinkEvent::Data(payload) => {
              let payload = str::from_utf8(payload.as_slice()).unwrap();
              log::info!("OUT LINK PAYLOAD {} ({}): {}", link_event.address_hash,
                link_event.id, payload);
              if &payload[0..4] == "pong" {
                let n = (&payload[5..]).parse::<u64>().unwrap();
                let mut pings = pings.lock().await;
                let index = pings.iter().position(|x| *x == n).unwrap();
                pings.remove(index);
                log::info!("UNACKED PINGS: {pings:?}");
              } else {
                unreachable!()
              }
            }
            LinkEvent::Activated => {
              log::info!("OUT LINK ACTIVATED {} ({})", link_event.address_hash,
                link_event.id)
            }
            LinkEvent::Closed => {
              log::info!("OUT LINK CLOSED {} ({})", link_event.address_hash,
                link_event.id)
            }
          }
        }
        Err(err) => {
          log::error!("out link error: {err:?}");
          break
        }
      }
    }
    log::info!("OUT LINK LOOP EXIT");
  };
  let in_event_loop = async || {
    let mut in_link_events = transport.in_link_events();
    loop {
      match in_link_events.recv().await {
        Ok(link_event) => {
          match link_event.event {
            LinkEvent::Data(payload) => {
              log::info!("IN LINK PAYLOAD {} ({}): {}", link_event.address_hash,
                link_event.id, str::from_utf8(payload.as_slice()).unwrap())
            }
            LinkEvent::Activated => {
              log::info!("IN LINK ACTIVATED {} ({})", link_event.address_hash,
                link_event.id)
            }
            LinkEvent::Closed => {
              log::info!("IN LINK CLOSED {} ({})", link_event.address_hash,
                link_event.id)
            }
          }
        }
        Err(err) => {
          log::error!("out link error: {err:?}");
          break
        }
      }
    }
    log::info!("IN LINK LOOP EXIT");
  };
  let ping_loop = async || {
    let mut counter = 0;
    loop {
      if let Some(current_link) = current_link.lock().await.as_mut() {
        if counter == 5 {
          let mut link = current_link.lock().await;
          log::info!("CLOSING LINK");
          link.close();
          let destination = link.destination().clone();
          drop(link);
          let link = transport.link (destination).await;
          log::info!("NEW LINK: {}", link.lock().await.id());
          *current_link = link;
        }
        log::info!("SEND PING {counter}");
        pings.lock().await.push(counter);
        let link = current_link.lock().await;
        let packet = link.data_packet(format!("ping {counter}").as_bytes()).unwrap();
        drop(link);
        transport.send_packet(packet).await;
        counter += 1;
      }
      time::sleep(time::Duration::from_secs(2)).await;
    }
  };
  tokio::select!{
    _ = link_loop() => log::info!("link loop exited"),
    _ = out_event_loop() => log::info!("out event loop exited"),
    _ = in_event_loop() => log::info!("in event loop exited"),
    _ = ping_loop() => log::info!("ping loop exited")
  }
}
