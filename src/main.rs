use std::sync::Arc;
use rand::rngs::OsRng;
use reticulum::destination::link::LinkEventData;
use reticulum::iface::tcp_client::TcpClient;
use reticulum::iface::tcp_server::TcpServer;
use reticulum::transport::{Transport, TransportConfig};
use reticulum::destination::{DestinationName, SingleInputDestination};
use reticulum::packet::Packet;
use reticulum::identity::PrivateIdentity;

mod logging;
use logging::{LoggingLevel, logging_function};

#[tokio::main]
async fn main() {
    info!("Server is starting");

    // Create transport and identity
    let identity = PrivateIdentity::new_from_name("weather-server");

    let mut transport = Transport::new(TransportConfig::new("weather", &identity, true));

    // Listen for incoming TCP connections on port 53317
    transport
        .iface_manager()
        .lock()
        .await
        .spawn(
            // Server means router
            TcpClient::new("127.0.0.1:53317"),
            TcpClient::spawn
        );

    // let dest = SingleInputDestination::new(identity, DestinationName::new("hello_world", "responder"));

    // Register a public destination
    let destination = transport
        .add_destination(
            identity,
            DestinationName::new("weather", "service"), // => weather.service
        )
        .await;


    // Announce to network
    let mut rng = Box::new(OsRng::default());
    transport
        .send_packet(destination.lock().await.announce(*rng, None).unwrap())
        .await;

    tokio::spawn(async move {
        // let a = transport.in_link_events().recv().await;
        // match transport.in_link_events().recv().await {
            // Ok(LinkEventData {id, address_hash, event}) => {},
            // Err(_) => {}
        // };
        let receiver = transport.recv_announces();
        let mut receiver = receiver.await;

        let mut links = transport.in_link_events();

        loop {
            // match links.recv().await {
                // Ok(LinkEventData {id, address_hash, event}) => { info!("id: {:?}, address_hash: {:?}, event", id, address_hash); },
                // Err(_) => {}
            // };
            if let Ok(announce) = receiver.recv().await {
                info!("destination announce {}", announce.destination.lock().await.desc.address_hash);
            }
        }

    });

    // Graceful shutdown
    let _ = tokio::signal::ctrl_c().await;
    info!("Server is stopping");
}
