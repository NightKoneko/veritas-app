use serde::Deserialize;
use std::{collections::HashMap, fmt};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Deserialize)]
pub struct Avatar {
    pub name: String,
}

impl fmt::Display for Avatar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    pub name: String,
    pub r#type: String,
}

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.r#type, self.name)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillData {
    pub avatar: Avatar,
    pub skill: Skill 
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorData {
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DamageData {
    pub attacker: Avatar,
    pub damage: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TurnData {
    pub avatars: Vec<Avatar>,
    pub avatars_damage: Vec<f64>,
    pub total_damage: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TurnBeginData {
    pub action_value: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Packet {
    pub r#type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct KillData {
    pub attacker: Avatar,
}

#[derive(Debug, Deserialize)]
pub struct SetupData {
    pub avatars: Vec<Avatar>,
}

#[derive(Debug)]
pub struct DataBuffer {
    inner: Mutex<DataBufferInner>,
}

#[derive(Debug, Clone, Default)]
pub struct DataBufferInner {
    pub rows: Vec<Vec<f64>>,
    pub column_names: Vec<String>,
    pub total_damage: HashMap<String, f64>,
    pub av_damage: Vec<HashMap<String, f64>>,
    pub turn_damage: Vec<HashMap<String, f64>>,
    pub current_turn: HashMap<String, f64>,
    pub current_av: f64,
    pub av_history: Vec<f64>,
    pub total_dpav: f64,
    pub dpav_history: Vec<f64>,
}

impl DataBuffer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(DataBufferInner::default())
        }
    }

    pub async fn lock(&self) -> Result<tokio::sync::MutexGuard<'_, DataBufferInner>, tokio::sync::TryLockError> {
        Ok(self.inner.lock().await)
    }

    pub fn try_lock(&self) -> Result<tokio::sync::MutexGuard<'_, DataBufferInner>, tokio::sync::TryLockError> {
        self.inner.try_lock()
    }


    pub fn blocking_lock(&self) -> tokio::sync::MutexGuard<'_, DataBufferInner> {
        self.inner.blocking_lock()
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

    pub fn update_dpav(&mut self, av: f64) {
        if av > 0.0 {
            let total_damage: f64 = self.total_damage.values().sum();
            let dpav = total_damage / av;
            self.dpav_history.push(dpav);
            
            self.total_dpav = dpav;
        }
    }
}