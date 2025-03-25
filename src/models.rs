use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Deserialize)]
pub struct Avatar {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DamageData {
    pub attacker: Avatar,
    pub damage: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TurnData {
    pub avatars: Vec<Avatar>,
    pub avatars_damage: Vec<i64>,
    pub total_damage: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Packet {
    pub r#type: String,
    pub data: serde_json::Value,
}

#[derive(Debug)]
pub struct DataBuffer {
    inner: Mutex<DataBufferInner>,
}

#[derive(Debug, Clone, Default)]
pub struct DataBufferInner {
    pub rows: Vec<Vec<i64>>,
    pub column_names: Vec<String>,
    pub total_damage: HashMap<String, i64>,
    pub turn_damage: Vec<HashMap<String, i64>>,
    pub current_turn: HashMap<String, i64>,
}

impl DataBuffer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(DataBufferInner::default())
        }
    }

    pub fn try_lock(&self) -> Option<std::sync::MutexGuard<'_, DataBufferInner>> {
        self.inner.try_lock().ok()
    }
}
