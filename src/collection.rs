use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionTheme {
    pub slug: String,
    pub title: String,
    pub is_dark: bool,
    pub raw_config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub themes: Vec<CollectionTheme>,
    pub current_index: usize,
    pub order: CycleOrder,
    pub interval: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CycleOrder {
    Sequential,
    Shuffle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_collection: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_collection: None,
        }
    }
}

/// Base directory: ~/.config/ghostty-styles/
pub fn base_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ghostty-styles")
}

pub fn collections_dir() -> PathBuf {
    base_dir().join("collections")
}

pub fn config_path() -> PathBuf {
    base_dir().join("config.json")
}

pub fn pid_path() -> PathBuf {
    base_dir().join("daemon.pid")
}

pub fn ensure_dirs() -> Result<(), String> {
    fs::create_dir_all(collections_dir()).map_err(|e| format!("Failed to create dirs: {}", e))
}

pub fn load_config() -> AppConfig {
    config_path()
        .exists()
        .then(|| {
            fs::read_to_string(config_path())
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        })
        .flatten()
        .unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    ensure_dirs()?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| format!("Failed to write config: {}", e))
}

pub fn load_collection(name: &str) -> Result<Collection, String> {
    let path = collections_dir().join(format!("{}.json", name));
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read collection '{}': {}", name, e))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse collection '{}': {}", name, e))
}

pub fn save_collection(collection: &Collection) -> Result<(), String> {
    ensure_dirs()?;
    let path = collections_dir().join(format!("{}.json", collection.name));
    let json = serde_json::to_string_pretty(collection).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Failed to write collection: {}", e))
}

pub fn list_collections() -> Vec<String> {
    let dir = collections_dir();
    if !dir.exists() {
        return Vec::new();
    }
    fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.strip_suffix(".json").map(|n| n.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn delete_collection(name: &str) -> Result<(), String> {
    let path = collections_dir().join(format!("{}.json", name));
    fs::remove_file(path).map_err(|e| format!("Failed to delete collection '{}': {}", name, e))
}

pub fn create_collection(name: &str) -> Result<Collection, String> {
    let collection = Collection {
        name: name.to_string(),
        themes: Vec::new(),
        current_index: 0,
        order: CycleOrder::Sequential,
        interval: None,
    };
    save_collection(&collection)?;
    Ok(collection)
}
