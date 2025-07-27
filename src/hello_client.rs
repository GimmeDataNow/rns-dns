// examples/hello_client.rs

use reticulum::destination::link::{LinkEvent, LinkStatus};
use reticulum::destination::DestinationName;
use reticulum::identity::PrivateIdentity;
use reticulum::iface::tcp_client::TcpClient;
use reticulum::transport::{Transport, TransportConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;

use rns_dns::{info, warn, error, trace, fatal};
use rns_dns::logging::{logging_function, LoggingLevel};


#[tokio::main]
async fn main() {
    // 1. Initialize logger to show informational messages.
    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    self::info!("--- Reticulum HELLO Client ---");

    // 2. Create a new Transport instance with a default configuration.
    // let transport_config = TransportConfig::default();
    let ident = PrivateIdentity::new_from_name("dns-server");
    let transport_config = TransportConfig::new("server", &ident, true);
    let transport = Arc::new(Mutex::new(Transport::new(transport_config)));

    // 3. Spawn a TCP client interface and connect to the server's address.
    transport
        .lock()
        .await
        .iface_manager()
        .lock()
        .await
        .spawn(TcpClient::new("127.0.0.1:53317"), TcpClient::spawn);
    self::info!("TCP client connecting to 127.0.0.1:53317...");


    // 4. Subscribe to announcement events to discover the server.
    let mut announce_receiver = transport.lock().await.recv_announces().await;
    self::info!("Listening for server announcements...");

    // The destination name must match the one used by the server.
    let server_dest_name = DestinationName::new("hello", "server");
    let mut server_link = None;

    // 5. Wait for an announcement from the correct service.
    // We'll use a timeout to avoid waiting forever if the server isn't running.
    let discovery_timeout = time::timeout(Duration::from_secs(60), async {
        loop {
            if let Ok(announce) = announce_receiver.recv().await {
                let dest = announce.destination.lock().await;
                // Check if the announced destination has the name we're looking for.
                if dest.desc.name.hash == server_dest_name.hash {
                    self::info!(
                        "Discovered server destination: {}",
                        dest.desc.address_hash
                    );

                    // 6. Once found, create a Link to the server's destination.
                    let link = transport.lock().await.link(dest.desc).await;
                    server_link = Some(link);
                    break;
                }
            }
        }
    })
    .await;

    if discovery_timeout.is_err() || server_link.is_none() {
        self::error!("Failed to discover the server within 30 seconds. Is the server running?");
        return;
    }

    let link = server_link.unwrap();
    let mut link_events = transport.lock().await.out_link_events();

    // 7. Wait for the Link to become active.
    self::info!("Waiting for link to become active...");
    let link_activation_timeout = time::timeout(Duration::from_secs(10), async {
        loop {
            // First, check the link status directly.
            if link.lock().await.status() == LinkStatus::Active {
                break;
            }
            // If not active, wait for an "Activated" event.
            if let Ok(event_data) = link_events.recv().await {
                if event_data.id == *link.lock().await.id() {
                    if let LinkEvent::Activated = event_data.event {
                        self::info!("Link {} is now active!", event_data.id);
                        break;
                    }
                }
            }
        }
    })
    .await;

    if link_activation_timeout.is_err() {
        self::error!("Link did not become active within 10 seconds. Exiting.");
        return;
    }

    // 8. Send the "HELLO" message to the server over the active link.
    self::info!("Sending 'HELLO' to the server...");
    let hello_packet = link.lock().await.data_packet(b"HELLO").unwrap();
    transport.lock().await.send_packet(hello_packet).await;

    // 9. Wait for the server's response.
    self::info!("Waiting for a response from the server...");
    let response_timeout = time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(event_data) = link_events.recv().await {
                // Ensure the event is for our link.
                if event_data.id == *link.lock().await.id() {
                    // We are looking for a Data event.
                    if let LinkEvent::Data(payload) = event_data.event {
                        let response = String::from_utf8_lossy(payload.as_slice());
                        self::info!("Received response from server: {}", response);
                        break;
                    }
                }
            }
        }
    })
    .await;

    if response_timeout.is_err() {
        self::error!("Did not receive a response within 10 seconds.");
    }

    self::info!("Client finished.");
}
