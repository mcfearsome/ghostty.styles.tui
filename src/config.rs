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

/// Filter out color-related config lines, keeping comments, blank lines, and non-color keys.
pub(crate) fn filter_color_keys(content: &str) -> String {
    let filtered_lines: Vec<&str> = content
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
    filtered_lines.join("\n")
}

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
        let backup_path = config_path.with_file_name("config.bak");
        fs::copy(&config_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }

    // Filter out existing color-related lines
    let mut new_config = filter_color_keys(&existing);
    if !new_config.ends_with('\n') && !new_config.is_empty() {
        new_config.push('\n');
    }
    new_config.push_str(&format!("\n# Theme: {}\n", theme.title));
    new_config.push_str(&theme.raw_config);
    if !new_config.ends_with('\n') {
        new_config.push('\n');
    }

    fs::write(&config_path, &new_config).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(config_path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_removes_background() {
        let input = "background = #1a1b26\nfont-size = 14";
        let result = filter_color_keys(input);
        assert!(!result.contains("background"));
        assert!(result.contains("font-size = 14"));
    }

    #[test]
    fn filter_removes_palette() {
        let input = "palette = 0=#000000\npalette = 1=#ff0000\nfont-family = Fira Code";
        let result = filter_color_keys(input);
        assert!(!result.contains("palette"));
        assert!(result.contains("font-family = Fira Code"));
    }

    #[test]
    fn filter_removes_all_color_keys() {
        let input = "background = #000\nforeground = #fff\ncursor-color = #f00\ncursor-text = #0f0\nselection-background = #00f\nselection-foreground = #ff0\ncursor-style = block\nbackground-opacity = 0.9";
        let result = filter_color_keys(input);
        assert_eq!(result.trim(), "");
    }

    #[test]
    fn filter_keeps_comments() {
        let input = "# This is a comment\nbackground = #000";
        let result = filter_color_keys(input);
        assert!(result.contains("# This is a comment"));
        assert!(!result.contains("background = #000"));
    }

    #[test]
    fn filter_keeps_empty_lines() {
        let input = "font-size = 14\n\nfont-family = Fira Code";
        let result = filter_color_keys(input);
        assert!(result.contains("font-size = 14"));
        assert!(result.contains("font-family = Fira Code"));
        assert!(result.contains("\n\n"));
    }

    #[test]
    fn filter_keeps_non_color_keys() {
        let input = "font-size = 14\nwindow-padding-x = 10\nfont-family = Fira Code";
        let result = filter_color_keys(input);
        assert_eq!(result, input);
    }

    #[test]
    fn filter_mixed_content() {
        let input = "# My config\nfont-size = 14\nbackground = #1a1b26\nforeground = #c0caf5\n\nwindow-padding-x = 10\npalette = 0=#15161e";
        let result = filter_color_keys(input);
        assert!(result.contains("# My config"));
        assert!(result.contains("font-size = 14"));
        assert!(result.contains("window-padding-x = 10"));
        assert!(!result.contains("background = #1a1b26"));
        assert!(!result.contains("foreground = #c0caf5"));
        assert!(!result.contains("palette"));
    }
}
