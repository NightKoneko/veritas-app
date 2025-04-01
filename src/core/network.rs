use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::{io::AsyncReadExt, sync::Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use anyhow::{anyhow, Error, Result};
use crate::core::models::Packet;


#[derive(Debug)]
pub enum ConnectionStatus {
    Connected,
    Failed(String),
}

#[derive(Debug)]
pub struct NetworkClient {
    pub stream: Arc<Mutex<Option<TcpStream>>>,
}

impl NetworkClient {
    pub fn new() -> Self {
        Self {
            stream: Arc::new(Mutex::new(None))
        }
    }

    // Can we handle this better?
    pub async fn start_connection(
        &mut self,
        status_tx: &mpsc::Sender<ConnectionStatus>,
        server_addr: &Arc<Mutex<String>>,
        server_port: &Arc<Mutex<String>>
    ) -> bool {
        let addr = format!(
            "{}:{}",
            server_addr.lock().await,
            server_port.lock().await
        );
        // Try connecting
        match self.connect(&addr).await {
            Ok(is_connected) => {
                if is_connected {
                    status_tx.send(ConnectionStatus::Connected).await.unwrap();
                }
                return true;
            },
            Err(e) => {
                status_tx.send(ConnectionStatus::Failed(e.to_string())).await.unwrap();
                return false;
            },
        }
    }

    pub async fn connect(&mut self, addr: &str) -> Result<bool> {
        let mut stream_lock = self.stream.lock().await;

        // If a connection already exists, return early
        if stream_lock.is_some() {
            return Ok(false);
        }

        // Otherwise, create a new connection
        let stream = TcpStream::connect(addr).await?;
        *stream_lock = Some(stream);

        Ok(true)
    }

    pub async fn disconnect(&mut self) {
        let mut stream_lock = self.stream.lock().await;
        *stream_lock = None;
    }

    pub async fn start_receiving(&mut self, tx: &mpsc::Sender<Packet>) -> Result<()>{    
        let mut stream_lock = self.stream.lock().await;
        let stream = stream_lock.as_mut().ok_or_else(|| anyhow!("Not connected"))?;
        
        let mut size_buf = [0u8; 4];
        stream.read_exact(&mut size_buf).await?;

        let size = u32::from_ne_bytes(size_buf) as usize;

        let mut packet_buf = vec![0u8; size];
        stream.read_exact(&mut packet_buf).await?;

        let packet = serde_json::from_slice::<Packet>(&packet_buf)?;
        if packet.r#type != "Heartbeat" {
            tx.send(packet).await?;
        }
        Ok(())
    }

    // Should ping again?
    pub async fn try_pinging(self: &mut NetworkClient, retries: &mut usize, max_retries: usize, timeout_duration: &mut Duration, initial_timeout: &Duration) -> bool {
        *retries += 1;
        println!("Retries: {}", retries);
        if *retries > max_retries {
            println!("Disconnected: {}", retries);
            self.disconnect().await;
            return false;
        }
        thread::sleep(*timeout_duration);
        *timeout_duration = *initial_timeout * 2u32.pow(*retries as u32);
        return true;
    }
}
