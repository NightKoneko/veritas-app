use std::io::Read;
use std::net::TcpStream;
use tokio::sync::mpsc;
use anyhow::{Result, anyhow};
use crate::models::Packet;

pub struct NetworkClient {
    stream: Option<TcpStream>,
    running: bool,
}

impl NetworkClient {
    pub fn new() -> Self {
        Self {
            stream: None,
            running: false,
        }
    }

    pub fn connect(&mut self, addr: &str) -> Result<()> {
        self.stream = Some(TcpStream::connect(addr)?);
        self.running = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.running = false;
        self.stream = None;
    }

    pub fn start_receiving(&mut self, tx: mpsc::Sender<Packet>) -> Result<()> {
        let mut stream = self.stream.as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?
            .try_clone()?;
            
        stream.set_nonblocking(false)?;
        
        std::thread::spawn(move || {
            loop {
                let mut size_buf = [0u8; 4];
                if stream.read_exact(&mut size_buf).is_err() {
                    break;
                }
                let size = u32::from_le_bytes(size_buf) as usize;
                
                let mut packet_buf = vec![0u8; size];
                if stream.read_exact(&mut packet_buf).is_err() {
                    break;
                }

                if let Ok(packet_str) = String::from_utf8(packet_buf) {
                    if let Ok(packet) = serde_json::from_str(&packet_str) {
                        if tx.blocking_send(packet).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
