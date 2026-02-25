use std::fs;
use std::process::Command;

use crate::collection;
use crate::config;
use crate::creator::CreatorState;

/// Derive a URL-friendly slug from a title string.
///
/// Lowercases the input, replaces non-alphanumeric characters with hyphens,
/// deduplicates consecutive hyphens, and strips leading/trailing hyphens.
pub fn slug_from_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Export a theme as a `.conf` file to `~/.config/ghostty-styles/themes/<slug>.conf`.
///
/// Creates the themes directory if it does not exist. Returns the absolute path
/// to the written file on success.
pub fn export_theme(state: &CreatorState) -> Result<String, String> {
    let themes_dir = collection::base_dir().join("themes");
    fs::create_dir_all(&themes_dir)
        .map_err(|e| format!("Failed to create themes directory: {}", e))?;

    let slug = slug_from_title(&state.title);
    if slug.is_empty() {
        return Err("Theme title is empty — cannot generate file name".to_string());
    }

    let file_path = themes_dir.join(format!("{}.conf", slug));
    let raw_config = state.build_raw_config();

    fs::write(&file_path, &raw_config)
        .map_err(|e| format!("Failed to write theme file: {}", e))?;

    Ok(file_path.display().to_string())
}

/// Apply the creator's current theme to the Ghostty config file.
///
/// Builds a `GhosttyConfig` from the `CreatorState` and delegates to
/// `config::apply_theme`. Returns the config file path on success.
pub fn apply_created_theme(state: &CreatorState) -> Result<String, String> {
    let ghostty_config = state.build_preview_config();
    config::apply_theme(&ghostty_config)
}

/// Open a URL in the user's default browser.
///
/// Uses `open` on macOS and `xdg-open` on Linux.
pub fn open_url(url: &str) -> Result<(), String> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    Command::new(program)
        .arg(url)
        .spawn()
        .map_err(|e| format!("Failed to open URL with {}: {}", program, e))?;

    Ok(())
}

/// Export the theme to a `.conf` file and open the upload page in the browser.
///
/// Returns a user-facing message indicating the saved path and that the upload
/// page has been opened.
pub fn upload_theme(state: &CreatorState) -> Result<String, String> {
    let path = export_theme(state)?;

    open_url("https://ghostty-style.vercel.app/upload")?;

    Ok(format!(
        "Config saved to {}. Upload page opened — drag the file to submit.",
        path
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic() {
        assert_eq!(slug_from_title("My Cool Theme"), "my-cool-theme");
    }

    #[test]
    fn slug_special_characters() {
        assert_eq!(slug_from_title("Nord (Dark)!"), "nord-dark");
    }

    #[test]
    fn slug_leading_trailing_hyphens() {
        assert_eq!(slug_from_title("--hello--world--"), "hello-world");
    }

    #[test]
    fn slug_consecutive_non_alnum() {
        assert_eq!(slug_from_title("a---b___c"), "a-b-c");
    }

    #[test]
    fn slug_empty_title() {
        assert_eq!(slug_from_title(""), "");
    }

    #[test]
    fn slug_all_special() {
        assert_eq!(slug_from_title("!@#$%"), "");
    }

    #[test]
    fn slug_unicode() {
        // Non-ASCII alphanumerics are preserved
        assert_eq!(slug_from_title("café theme"), "caf-theme");
    }

    #[test]
    fn slug_numbers() {
        assert_eq!(slug_from_title("Theme 42"), "theme-42");
    }

    #[test]
    fn export_theme_empty_title_fails() {
        let mut state = CreatorState::new("test");
        state.title = String::new();
        let result = export_theme(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }
}
