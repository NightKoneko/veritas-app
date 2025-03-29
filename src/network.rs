use std::error;
use std::sync::Arc;
use std::io::Read;
use tokio::{io::AsyncReadExt, sync::Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use anyhow::{Result, anyhow};
use crate::models::Packet;

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

    pub async fn start_receiving(&mut self, tx: &mpsc::Sender<Packet>) -> std::result::Result<(), tokio::io::Error> {    
        let mut stream_lock = self.stream.lock().await;
        let stream = stream_lock.as_mut().ok_or_else(|| anyhow!("Not connected")).unwrap();
        

        let mut size_buf = [0u8; 4];
        stream.read(&mut size_buf).await?;

        let size = u32::from_ne_bytes(size_buf) as usize;
        
        let mut packet_buf = vec![0u8; size];
        stream.read(&mut packet_buf).await?;

        if let Ok(packet_str) = String::from_utf8(packet_buf) {
            if let Ok(packet) = serde_json::from_str(&packet_str) {
                tx.send(packet).await.unwrap();
            }
        }
        Ok(())
    }
}
