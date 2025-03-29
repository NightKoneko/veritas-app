use std::sync::{Arc};

use tokio::sync::{mpsc, Mutex, MutexGuard};

use crate::{models::Packet, network::NetworkClient};

#[derive(Debug)]
pub enum ConnectionStatus {
    Connected,
    Failed(String),
}

pub async fn start_connection(
    payload_tx: &mpsc::Sender<Packet>,
    status_tx: &mpsc::Sender<ConnectionStatus>,
    server_addr: &Arc<std::sync::Mutex<String>>,
    server_port: &Arc<std::sync::Mutex<String>>,
) {
    let mut client = NetworkClient::new();
    loop {
        let addr = format!(
            "{}:{}",
            server_addr.clone().try_lock().unwrap(),
            server_port.clone().try_lock().unwrap()
        );
        // Try connecting
        match client.connect(&addr).await {
            Ok(is_connected) => {
                if is_connected {
                    status_tx
                    .send(ConnectionStatus::Connected)
                    .await.unwrap();

                    // On success
                    loop {
                        let res = client.start_receiving(payload_tx).await;
                        if res.is_err() {
                            // TODO: Add warning
                            status_tx
                                .send(ConnectionStatus::Failed("Disconnected from server".to_string()))
                                .await.unwrap();
                            client.disconnect().await;
                            break;
                        }
                    }
    
                }
            },
            Err(e) => {
                status_tx
                    .send(ConnectionStatus::Failed(e.to_string()))
                    .await.unwrap();
            },
        }    
    }
}