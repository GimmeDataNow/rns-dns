// #[macro_use]
// use crate::{info, LoggingLevel, logging_function};
// #[macro_use]
// extern crate rns_dns;
// use crate::logging;
// use rns_dns::logging::{info, warn, error, trace, fatal};

use reticulum::iface::tcp_client::TcpClient;
// #[macro_use]
// extern crate rns_dns;
use rns_dns::{info, warn, error, trace, fatal};
use rns_dns::logging::{logging_function, LoggingLevel};


use reticulum::destination::{link::LinkEvent, DestinationName};
use reticulum::identity::PrivateIdentity;
use reticulum::iface::tcp_server::TcpServer;
use reticulum::transport::{Transport, TransportConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    // 1. Initialize logger

    self::info!("--- Reticulum HELLO Server ---");

    // 2. Create a default Transport configuration
    // let transport_config = TransportConfig::default();
    let transport_config = Transport::new(TransportConfig::new(
        "server",
        &PrivateIdentity::new_from_name("weather-server"),
        true,
    ));
    let transport = Arc::new(Mutex::new(transport_config));
    // let iface_inst = transport.lock().await.iface_manager().clone();


    // 3. Spawn a TCP server interface to allow clients to connect
    // This will listen for incoming TCP connections on port 4242
    transport
        .lock()
        .await
        .iface_manager()
        .lock()
        .await
        .spawn(
            // TcpServer::new("0.0.0.0:53317", iface_inst),
            // TcpServer::spawn,
            TcpClient::new("127.0.0.1:53317"),
            TcpClient::spawn,
        );
    self::info!("TCP server listening on 0.0.0.0:53317");

    // 4. Create a persistent Identity for the server
    // Using a fixed name ensures the server has the same address every time it starts
    let identity = PrivateIdentity::new_from_name("hello-server-identity");

    // 5. Create a destination for the "hello" service
    let destination_name = DestinationName::new("hello", "server");
    let destination = transport
        .lock()
        .await
        .add_destination(identity, destination_name)
        .await;

    self::info!(
        "Server destination created with address: {}",
        destination.lock().await.desc.address_hash
    );

    // 6. Announce the server's destination periodically so clients can find it
    // We'll spawn a separate async task for this
    {
        let transport = transport.clone();
        let destination = destination.clone();
                        transport
                    .lock()
                    .await
                    .send_announce(&destination, Some(b"Hello Server"))
                    .await;

        tokio::spawn(async move {
            loop {
                self::info!("Announcing server destination...");
                transport
                    .lock()
                    .await
                    .send_announce(&destination, Some(b"Hello Server"))
                    .await;
                // Announce every 5 minutes
                tokio::time::sleep(Duration::from_secs(15)).await;
            }
        });
    }

    // 7. Listen for incoming Link events
    // This is how the server will receive data from clients
    let mut link_events = transport.lock().await.in_link_events();
        // .await;
    self::info!("Waiting for incoming client links...");

    loop {
        // Wait for the next event on an established link
        if let Ok(event_data) = link_events.recv().await {
            match event_data.event {
                // We are interested in Data events
                LinkEvent::Data(payload) => {
                    let received_data = payload.as_slice();
                    self::info!(
                        "Received {} bytes on link {}",
                        payload.len(),
                        event_data.id
                    );

                    // 8. Check if the received data is "HELLO"
                    if received_data == b"HELLO" {
                        self::info!("Received 'HELLO', sending response...");

                        // Find the link the data came from
                        if let Some(link) =
                            transport.lock().await.find_in_link(&event_data.id).await
                        {
                            // Create a response packet with "HELLO THERE"
                            let response_packet =
                                link.lock().await.data_packet(b"HELLO THERE").unwrap();

                            // Send the response back over the same link
                            transport.lock().await.send_packet(response_packet).await;
                            self::info!("Sent 'HELLO THERE' response.");
                        }
                    }
                }
                LinkEvent::Activated => {
                    self::info!("Link {} activated", event_data.id);
                }
                LinkEvent::Closed => {
                    self::info!("Link {} closed", event_data.id);
                }
            }
        }
    }
}
