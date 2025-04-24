use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::core::updater::VeritasVersion;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub game_path: Option<String>,
    pub dll_version: Option<String>,
    pub version_type: Option<VeritasVersion>,
}

impl Config {
    pub fn load() -> Self {
        if let Some(config_path) = get_config_path() {
            if let Ok(contents) = fs::read_to_string(config_path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Default::default()
    }

    pub fn save(&self) {
        if let Some(config_path) = get_config_path() {
            if let Ok(contents) = serde_json::to_string_pretty(self) {
                let _ = fs::create_dir_all(config_path.parent().unwrap());
                let _ = fs::write(config_path, contents);
            }
        }
    }
}

fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "veritas", "veritas-app")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
}
