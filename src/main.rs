use std::sync::{Arc};
use tokio::sync::Mutex;
use rand::rngs::OsRng;
use reticulum::destination::link::LinkEventData;
use reticulum::iface::tcp_client::TcpClient;
use reticulum::iface::tcp_server::TcpServer;
use reticulum::transport::{Transport, TransportConfig};
use reticulum::destination::DestinationName;
use reticulum::identity::PrivateIdentity;

#[macro_use]
pub mod logging;
use logging::{LoggingLevel, logging_function};

mod echo_server;
mod hello_client;

#[tokio::main]
async fn main() {
    info!("Server is starting");

    // Create transport and identity
    let identity = PrivateIdentity::new_from_name("weather-server");

    let transport = Transport::new(TransportConfig::new("weather", &identity, true));
    let transport = Arc::new(Mutex::new(transport));

    // Listen for incoming TCP connections on port 53317
    transport
        .lock().await
        .iface_manager()
        .lock()
        .await
        .spawn(
            // Server means router
            TcpClient::new("127.0.0.1:53317"),
            TcpClient::spawn
        );

    // Register a public destination
    let destination = transport
        .lock().await
        .add_destination(
            identity,
            DestinationName::new("weather", "service"), // => weather.service
        )
        .await;
    info!("Server destination is: {}", destination.lock().await.desc.address_hash);

    // Announce to network
    let rng = Box::new(OsRng::default());
    transport
        .lock().await
        .send_packet(destination.lock().await.announce(*rng, None).unwrap())
        .await;

    let t1 = Arc::clone(&transport);
    tokio::spawn(async move {
        let binding = t1.lock().await;
        let receiver = binding.recv_announces();
        let mut receiver = receiver.await;

        loop {
            if let Ok(announce) = receiver.recv().await {
                info!("destination announce {}", announce.destination.lock().await.desc.address_hash);
            }
        }

    });

    let t2 = Arc::clone(&transport);
    tokio::spawn(async move {
        match t2.lock().await.in_link_events().recv().await {
            Ok(a) => {info!("received: {:?}", a.address_hash);},
            Err(_) => {error!("error at t2");}, 
        };
        
    });

    // Graceful shutdown
    let _ = tokio::signal::ctrl_c().await;
    info!("Server is stopping");
}
