use crate::models::get_base_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_model: String,
    pub system_prompt: String,
    pub temperature: f32,
    #[serde(default = "default_cpu_threads")]
    pub cpu_threads: usize,
}

fn default_cpu_threads() -> usize {
    match std::thread::available_parallelism() {
        Ok(n) => {
            let count = n.get();
            if count > 4 { count - 1 } else { count.max(2) }
        }
        Err(_) => 4,
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_model: String::new(),
            system_prompt: "You are a helpful, direct, and concise AI assistant. Provide clean, correct, working answers directly.".to_string(),
            temperature: 0.3,
            cpu_threads: default_cpu_threads(),
        }
    }
}

pub fn get_config_path() -> PathBuf {
    get_base_dir().join("potato_config.json")
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&data) {
                return config;
            }
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) {
    let path = get_config_path();
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = fs::write(path, json);
    }
}
