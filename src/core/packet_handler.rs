use std::{fs::{self, File}, sync::Arc};

use csv::Writer;
use tokio::sync::{mpsc, Mutex, MutexGuard};

use crate::{core::message_logger::MessageLogger, core::models::{DamageData, DataBuffer, KillData, Packet, SetupData, TurnData, TurnBeginData}};

use super::models::{DataBufferInner, ErrorData, SkillData};

pub struct PacketHandler {
    message_logger: Arc<Mutex<MessageLogger>>,
    data_buffer: Arc<DataBuffer>,
    csv_writer: Option<Writer<File>>,
    // Seems unnecessary atm
    current_file: String,
}

impl PacketHandler {
    pub fn new(message_logger: Arc<Mutex<MessageLogger>>, data_buffer: Arc<DataBuffer>) -> Self {
        Self {
            message_logger,
            data_buffer,
            csv_writer: None,
            current_file: String::new(),
        }
    }

    pub async fn handle_packets(&mut self, payload_rx: &mut mpsc::Receiver<Packet>) -> bool {
        let messager_logger_clone = self.message_logger.clone();
        let mut message_logger_lock = messager_logger_clone.lock().await;
        let data_buffer_clone = self.data_buffer.clone();
        let data_buffer_lock = data_buffer_clone.lock().await.unwrap();
        let mut is_there_update = true;
        match payload_rx.try_recv() {
            Ok(packet) => {
                match packet.r#type.as_str() {
                    "OnSetBattleLineup" => self.handle_lineup(packet.data, message_logger_lock, data_buffer_lock),
                    // TODO: Handle packet
                    "OnBattleBegin" => self.handle_battle_begin(packet.data, message_logger_lock, data_buffer_lock),
                    "OnTurnBegin" => self.handle_turn_begin(packet.data, message_logger_lock, data_buffer_lock),
                    "OnDamage" => self.handle_damage(packet.data, message_logger_lock, data_buffer_lock),
                    "OnTurnEnd" => self.handle_turn_end(packet.data, message_logger_lock, data_buffer_lock),
                    "OnKill" => self.handle_kill(packet.data, message_logger_lock, data_buffer_lock),
                    "OnBattleEnd" => self.handle_battle_end(message_logger_lock, data_buffer_lock),
                    "OnUseSkill" => self.handle_on_skill_use(packet.data, message_logger_lock, data_buffer_lock),
                    "Error" => self.handle_error(packet.data, message_logger_lock, data_buffer_lock),
                    // TODO: OnUpdateWave, OnUpdateCycle
                    _ => {
                        is_there_update = false;
                        message_logger_lock.log(&format!("Unknown packet type: {}", packet.r#type))
                    },
                }    
            },
            Err(_) => is_there_update = false,
        }
        is_there_update
    }

    fn handle_turn_begin(
        &mut self,
        data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        if let Ok(turn_data) = serde_json::from_value::<TurnBeginData>(data) {
            data_buffer.current_av = turn_data.action_value;
            message_logger.log(&format!("Turn begin - AV: {:.2}", turn_data.action_value));
        }
    }
    
    fn handle_turn_end(
        &mut self,
        data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        if let Ok(turn_data) = serde_json::from_value::<TurnData>(data) {
            for (avatar, &damage) in turn_data.avatars.iter().zip(turn_data.turn_info.avatars_turn_damage.iter()) {
                // If key doesn't exist, create
                if !data_buffer.current_turn.contains_key(&avatar.name) {
                    data_buffer.current_turn.insert(avatar.name.clone(), 0.0);
                }
                if damage > 0.0 {
                    message_logger.log(&format!(
                        "Turn summary - {}: {} damage",
                        avatar.name, damage
                    ));
                }
            }
            message_logger.log(&format!("Total turn damage: {}", turn_data.turn_info.total_damage));

            let current_av = (*data_buffer).current_av;
            
            let current = data_buffer.current_turn.clone();

            let len = data_buffer.av_history.len();
            if len > 0 {
                // New AV
                if data_buffer.av_history[len - 1] != current_av {
                    data_buffer.av_history.push(current_av);
                    data_buffer.av_damage.push(current.clone());    
                }
                // Same AV as before
                else {
                    let map = &mut data_buffer.av_damage[len - 1];
                    let current_copy = current.clone();
                    for (k, v) in map {
                        let dmg = current_copy.get(k).unwrap();
                        *v += *dmg;
                    }
                }
            }
            else {
                data_buffer.av_history.push(current_av);
                data_buffer.av_damage.push(current.clone());
            }
            
            
            data_buffer.turn_damage.push(current);
            if current_av > 0.0 {
                data_buffer.update_dpav(current_av);
            }

            data_buffer.current_turn.clear();
    }
    }
    
    fn handle_lineup(
        &mut self,
        data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        if let Ok(lineup_data) = serde_json::from_value::<SetupData>(data) {

            let names: Vec<String> = lineup_data.avatars.iter().map(|a| a.name.clone()).collect();
            
            fs::create_dir_all("damage_logs").unwrap_or_else(|e| {
                message_logger.log(&format!("Failed to create damage_logs directory: {}", e));
            });
    
            let filename = format!("HSR_{}.csv", chrono::Local::now().format("%Y%m%d_%H%M%S"));
            let path = format!("damage_logs/{}", filename);
            
            match File::create(&path) {
                Ok(file) => {
                    self.csv_writer = Some(Writer::from_writer(file));
                    self.current_file = path.clone();
                    
                    if let Some(writer) = &mut self.csv_writer {
                        if let Err(e) = writer.write_record(&names) {
                            message_logger.log(&format!("Failed to write CSV headers: {}", e));
                        }
                    }

                    data_buffer.init_characters(&names);
                    data_buffer.rows.clear();

                    message_logger.log(&format!("Created CSV: {}", filename));
                    message_logger.log(&format!("Headers: {:?}", names));
                }
                Err(e) => {
                    message_logger.log(&format!("Failed to create CSV file: {}", e));
                }
            }
        }
    }
    
    fn handle_battle_begin(&mut self,
        _data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut _data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        message_logger.log("Battle started");
    }
    
    fn handle_damage(
        &mut self,
        data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        if let Ok(damage_data) = serde_json::from_value::<DamageData>(data) {
        let attacker = damage_data.attacker.name.clone();
        let damage = damage_data.damage;
        
        if damage > 0.0 {
            message_logger.log(&format!("{} dealt {} damage", attacker, damage));
        }
        
        let mut should_write = false;
        let mut row = vec![0.0; data_buffer.column_names.len()];
        
        if let Some(idx) = data_buffer.column_names.iter().position(|name| name == &attacker) {
            row[idx] = damage;
            *data_buffer.total_damage.entry(attacker.clone()).or_insert(0.0) += damage;
            *data_buffer.current_turn.entry(attacker.clone()).or_insert(0.0) += damage;
            should_write = true;
        }
        data_buffer.rows.push(row.clone());
    
        if should_write {
            if let Some(writer) = &mut self.csv_writer {
                let _ = writer.write_record(&row.iter().map(|&x| x.to_string()).collect::<Vec<_>>());
                let _ = writer.flush();
            }
        }
    }
    }
    
    fn handle_kill(
        &mut self,
        data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut _data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        if let Ok(kill_data) = serde_json::from_value::<KillData>(data) {
            message_logger.log(&format!("{} has killed", kill_data.attacker.name));
        }
    }
    
    fn handle_battle_end(
        &mut self,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        let final_turn_data = if !data_buffer.current_turn.is_empty() {
            let total_damage: f64 = data_buffer.current_turn.values().sum();
            let final_turn = data_buffer.current_turn.clone();
            let av = data_buffer.current_av;

            data_buffer.update_dpav(av);
            data_buffer.turn_damage.push(final_turn.clone());

            Some((final_turn, total_damage))
        } else {
            None
        };
    
        if let Some((final_turn, total_damage)) = final_turn_data {
            for (name, damage) in final_turn {
                if damage > 0.0 {
                    message_logger.log(&format!(
                        "Final turn summary - {}: {} damage",
                        name, damage
                    ));
                }
            }
            message_logger.log(&format!("Final turn total damage: {}", total_damage));
        }
    
        self.csv_writer = None;
        message_logger.log("Battle ended - CSV file closed");
    }

    fn handle_on_skill_use(
        &mut self,
        data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut _data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        if let Ok(skill_data) = serde_json::from_value::<SkillData>(data) {
            message_logger.log(&format!("{} used {}", skill_data.avatar, skill_data.skill));
        }    
    }


    fn handle_error(
        &mut self,
        data: serde_json::Value,
        mut message_logger: MutexGuard<'_, MessageLogger>,
        mut _data_buffer: MutexGuard<'_, DataBufferInner>
    ) {
        if let Ok(error) = serde_json::from_value::<ErrorData>(data) {
            message_logger.log(&format!("{}", error.msg));
        }
    }
}