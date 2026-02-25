use std::fs;
use std::path::PathBuf;

use crate::theme::GhosttyConfig;

/// Get the path to the Ghostty config file.
pub fn ghostty_config_path() -> Option<PathBuf> {
    // macOS: ~/Library/Application Support/com.mitchellh.ghostty/config
    // Linux: ~/.config/ghostty/config
    if cfg!(target_os = "macos") {
        dirs::home_dir().map(|h| {
            h.join("Library")
                .join("Application Support")
                .join("com.mitchellh.ghostty")
                .join("config")
        })
    } else {
        dirs::config_dir().map(|c| c.join("ghostty").join("config"))
    }
}

/// Color-related config keys that we'll replace when applying a theme.
const COLOR_KEYS: &[&str] = &[
    "background",
    "foreground",
    "cursor-color",
    "cursor-text",
    "selection-background",
    "selection-foreground",
    "palette",
    "cursor-style",
    "background-opacity",
];

/// Apply a theme's raw config to the Ghostty config file.
/// Creates a backup before modifying.
pub fn apply_theme(theme: &GhosttyConfig) -> Result<String, String> {
    let config_path = ghostty_config_path().ok_or("Could not determine Ghostty config path")?;

    // Read existing config or start fresh
    let existing = if config_path.exists() {
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?
    } else {
        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        String::new()
    };

    // Create backup
    if config_path.exists() {
        let backup_path = config_path.with_extension("config.bak");
        fs::copy(&config_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }

    // Filter out existing color-related lines
    let filtered_lines: Vec<&str> = existing
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return true;
            }
            let key = trimmed.split('=').next().unwrap_or("").trim();
            !COLOR_KEYS.iter().any(|k| key == *k)
        })
        .collect();

    // Build new config
    let mut new_config = filtered_lines.join("\n");
    if !new_config.ends_with('\n') && !new_config.is_empty() {
        new_config.push('\n');
    }
    new_config.push_str(&format!("\n# Theme: {}\n", theme.title));
    new_config.push_str(&theme.raw_config);
    if !new_config.ends_with('\n') {
        new_config.push('\n');
    }

    fs::write(&config_path, &new_config)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(config_path.display().to_string())
}
