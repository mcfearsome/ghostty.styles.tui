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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ModePreference {
    Dark,
    Light,
    AutoOs,
    AutoTime,
}

impl ModePreference {
    pub fn label(&self) -> &'static str {
        match self {
            ModePreference::Dark => "dark",
            ModePreference::Light => "light",
            ModePreference::AutoOs => "auto-os",
            ModePreference::AutoTime => "auto-time",
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            ModePreference::Dark => Some(ModePreference::Light),
            ModePreference::Light => Some(ModePreference::AutoOs),
            ModePreference::AutoOs => Some(ModePreference::AutoTime),
            ModePreference::AutoTime => None,
        }
    }
}

fn default_dark_after() -> String {
    "19:00".to_string()
}
fn default_light_after() -> String {
    "07:00".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_collection: Option<String>,
    #[serde(default)]
    pub mode_preference: Option<ModePreference>,
    #[serde(default = "default_dark_after")]
    pub dark_after: String,
    #[serde(default = "default_light_after")]
    pub light_after: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_collection: None,
            mode_preference: None,
            dark_after: default_dark_after(),
            light_after: default_light_after(),
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

pub fn normalize_collection_name(name: &str) -> Option<String> {
    let normalized = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn path_from_slug(slug: &str) -> PathBuf {
    collections_dir().join(format!("{}.json", slug))
}

fn collection_file_paths() -> Vec<PathBuf> {
    let dir = collections_dir();
    if !dir.exists() {
        return Vec::new();
    }
    fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
                .collect()
        })
        .unwrap_or_default()
}

fn find_path_by_collection_name(name: &str) -> Option<PathBuf> {
    collection_file_paths().into_iter().find(|path| {
        fs::read_to_string(path)
            .ok()
            .and_then(|data| serde_json::from_str::<Collection>(&data).ok())
            .is_some_and(|c| c.name == name)
    })
}

fn find_path_by_normalized_name(normalized_name: &str) -> Option<PathBuf> {
    collection_file_paths().into_iter().find(|path| {
        let stem_matches = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(normalize_collection_name)
            .as_deref()
            == Some(normalized_name);
        if stem_matches {
            return true;
        }
        fs::read_to_string(path)
            .ok()
            .and_then(|data| serde_json::from_str::<Collection>(&data).ok())
            .and_then(|c| normalize_collection_name(&c.name))
            .as_deref()
            == Some(normalized_name)
    })
}

fn resolve_existing_path(name: &str) -> Result<PathBuf, String> {
    if let Some(normalized) = normalize_collection_name(name) {
        if let Some(path) = find_path_by_normalized_name(&normalized) {
            return Ok(path);
        }
    }
    if let Some(path) = find_path_by_collection_name(name) {
        return Ok(path);
    }
    Err(format!("Collection '{}' not found", name))
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
    let path = resolve_existing_path(name)?;
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read collection '{}': {}", name, e))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse collection '{}': {}", name, e))
}

pub fn save_collection(collection: &Collection) -> Result<(), String> {
    ensure_dirs()?;
    let normalized_name = normalize_collection_name(&collection.name)
        .ok_or("Collection name must contain at least one letter or number")?;
    let path = find_path_by_collection_name(&collection.name)
        .or_else(|| find_path_by_normalized_name(&normalized_name))
        .unwrap_or_else(|| path_from_slug(&normalized_name));
    let json = serde_json::to_string_pretty(collection).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Failed to write collection: {}", e))
}

pub fn list_collections() -> Vec<String> {
    let mut names: Vec<String> = collection_file_paths()
        .into_iter()
        .filter_map(|path| {
            fs::read_to_string(&path)
                .ok()
                .and_then(|data| serde_json::from_str::<Collection>(&data).ok())
                .map(|c| normalize_collection_name(&c.name).unwrap_or(c.name))
                .or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(normalize_collection_name)
                })
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

pub fn delete_collection(name: &str) -> Result<(), String> {
    let path = resolve_existing_path(name)?;
    fs::remove_file(path).map_err(|e| format!("Failed to delete collection '{}': {}", name, e))
}

pub fn create_collection(name: &str) -> Result<Collection, String> {
    let normalized = normalize_collection_name(name);
    let normalized = match normalized {
        Some(n) => n,
        None => {
            if name.trim().is_empty() {
                return Err("Collection name cannot be empty".to_string());
            }
            return Err("Collection name must contain at least one letter or number".to_string());
        }
    };
    if find_path_by_normalized_name(&normalized).is_some() {
        return Err(format!("Collection '{}' already exists", normalized));
    }

    // Also guard against any legacy files keyed by exact display name.
    let trimmed = name.trim();
    if !trimmed.is_empty() && find_path_by_collection_name(trimmed).is_some() {
        return Err(format!("Collection '{}' already exists", normalized));
    }

    let collection = Collection {
        name: normalized,
        themes: Vec::new(),
        current_index: 0,
        order: CycleOrder::Sequential,
        interval: None,
    };
    save_collection(&collection)?;
    Ok(collection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_preference_labels() {
        assert_eq!(ModePreference::Dark.label(), "dark");
        assert_eq!(ModePreference::Light.label(), "light");
        assert_eq!(ModePreference::AutoOs.label(), "auto-os");
        assert_eq!(ModePreference::AutoTime.label(), "auto-time");
    }

    #[test]
    fn mode_preference_next_chain() {
        let mut pref = Some(ModePreference::Dark);
        pref = pref.unwrap().next();
        assert_eq!(pref, Some(ModePreference::Light));
        pref = pref.unwrap().next();
        assert_eq!(pref, Some(ModePreference::AutoOs));
        pref = pref.unwrap().next();
        assert_eq!(pref, Some(ModePreference::AutoTime));
        let end = pref.unwrap().next();
        assert_eq!(end, None);
    }

    #[test]
    fn cycle_order_serde_roundtrip() {
        let seq = serde_json::to_string(&CycleOrder::Sequential).unwrap();
        assert_eq!(seq, "\"sequential\"");
        let shuf = serde_json::to_string(&CycleOrder::Shuffle).unwrap();
        assert_eq!(shuf, "\"shuffle\"");

        let parsed: CycleOrder = serde_json::from_str("\"sequential\"").unwrap();
        assert!(matches!(parsed, CycleOrder::Sequential));
        let parsed: CycleOrder = serde_json::from_str("\"shuffle\"").unwrap();
        assert!(matches!(parsed, CycleOrder::Shuffle));
    }

    #[test]
    fn mode_preference_serde_roundtrip() {
        for pref in [
            ModePreference::Dark,
            ModePreference::Light,
            ModePreference::AutoOs,
            ModePreference::AutoTime,
        ] {
            let json = serde_json::to_string(&pref).unwrap();
            let parsed: ModePreference = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, pref);
        }
    }

    #[test]
    fn mode_preference_serde_kebab_case() {
        assert_eq!(
            serde_json::to_string(&ModePreference::AutoOs).unwrap(),
            "\"auto-os\""
        );
        assert_eq!(
            serde_json::to_string(&ModePreference::AutoTime).unwrap(),
            "\"auto-time\""
        );
    }

    #[test]
    fn app_config_default() {
        let config = AppConfig::default();
        assert!(config.active_collection.is_none());
        assert!(config.mode_preference.is_none());
        assert_eq!(config.dark_after, "19:00");
        assert_eq!(config.light_after, "07:00");
    }

    #[test]
    fn collection_theme_serde_roundtrip() {
        let theme = CollectionTheme {
            slug: "test-theme".to_string(),
            title: "Test Theme".to_string(),
            is_dark: true,
            raw_config: "background = #000".to_string(),
        };
        let json = serde_json::to_string(&theme).unwrap();
        let parsed: CollectionTheme = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.slug, "test-theme");
        assert_eq!(parsed.title, "Test Theme");
        assert!(parsed.is_dark);
    }

    #[test]
    fn app_config_serde_with_mode() {
        let config = AppConfig {
            active_collection: Some("favorites".to_string()),
            mode_preference: Some(ModePreference::AutoOs),
            dark_after: "20:00".to_string(),
            light_after: "06:00".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.active_collection.as_deref(), Some("favorites"));
        assert_eq!(parsed.mode_preference, Some(ModePreference::AutoOs));
        assert_eq!(parsed.dark_after, "20:00");
    }

    #[test]
    fn normalize_collection_name_basic() {
        assert_eq!(
            normalize_collection_name("My Themes"),
            Some("my-themes".to_string())
        );
    }

    #[test]
    fn normalize_collection_name_sanitizes_path_chars() {
        assert_eq!(
            normalize_collection_name("../../Favorites!!!"),
            Some("favorites".to_string())
        );
    }

    #[test]
    fn normalize_collection_name_empty_rejected() {
        assert_eq!(normalize_collection_name("___---"), None);
    }
}
