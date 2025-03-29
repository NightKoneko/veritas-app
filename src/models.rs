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
    pub damage: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TurnData {
    pub avatars: Vec<Avatar>,
    pub avatars_damage: Vec<f32>,
    pub total_damage: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TurnBeginData {
    pub action_value: f32,
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
    pub rows: Vec<Vec<f32>>,
    pub column_names: Vec<String>,
    pub total_damage: HashMap<String, f32>,
    pub turn_damage: Vec<HashMap<String, f32>>,
    pub current_turn: HashMap<String, f32>,
    pub current_av: f32,
    pub av_history: Vec<f32>,
    pub total_dpav: f32,
    pub dpav_history: Vec<f32>,
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

impl DataBufferInner {
    pub fn init_characters(&mut self, names: &[String]) {
        self.column_names = names.to_vec();
        self.total_damage = names.iter().map(|name| (name.clone(), 0.0)).collect();
        self.current_turn = names.iter().map(|name| (name.clone(), 0.0)).collect();
        self.turn_damage.clear();
        self.av_history.clear();
        self.current_av = 0.0;
        self.total_dpav = 0.0;
        self.dpav_history.clear();
    }

    pub fn update_dpav(&mut self, turn_damage: f32, av: f32) {
        if av > 0.0 {
            let dpav = turn_damage / av;
            self.dpav_history.push(dpav);
            
            let total_damage: f32 = self.total_damage.values().sum();
            let total_av: f32 = self.av_history.iter().sum();
            self.total_dpav = if total_av > 0.0 { total_damage / total_av } else { 0.0 };
        }
    }
}
