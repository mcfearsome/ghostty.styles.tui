use rand::Rng;

use crate::collection::{self, CycleOrder};
use crate::config;
use crate::theme::GhosttyConfig;

/// Advance to the next theme in the active collection and apply it.
pub fn apply_next() -> Result<String, String> {
    let app_config = collection::load_config();
    let coll_name = app_config
        .active_collection
        .ok_or("No active collection. Run: ghostty-styles collection use <name>")?;

    let mut coll = collection::load_collection(&coll_name)?;

    if coll.themes.is_empty() {
        return Err(format!("Collection '{}' is empty", coll_name));
    }

    let next_index = match coll.order {
        CycleOrder::Sequential => (coll.current_index + 1) % coll.themes.len(),
        CycleOrder::Shuffle => {
            let mut rng = rand::thread_rng();
            if coll.themes.len() == 1 {
                0
            } else {
                let mut next = coll.current_index;
                while next == coll.current_index {
                    next = rng.gen_range(0..coll.themes.len());
                }
                next
            }
        }
    };

    let theme_entry = &coll.themes[next_index];

    // Build a minimal GhosttyConfig to use with apply_theme.
    // Only raw_config and title are used by apply_theme.
    let ghost_config = GhosttyConfig {
        id: String::new(),
        slug: theme_entry.slug.clone(),
        title: theme_entry.title.clone(),
        description: None,
        raw_config: theme_entry.raw_config.clone(),
        background: String::new(),
        foreground: String::new(),
        cursor_color: None,
        cursor_text: None,
        selection_bg: None,
        selection_fg: None,
        palette: Vec::new(),
        font_family: None,
        font_size: None,
        cursor_style: None,
        bg_opacity: None,
        is_dark: theme_entry.is_dark,
        tags: Vec::new(),
        source_url: None,
        author_name: None,
        author_url: None,
        is_featured: false,
        vote_count: 0,
        view_count: 0,
        download_count: 0,
    };

    config::apply_theme(&ghost_config)?;

    coll.current_index = next_index;
    collection::save_collection(&coll)?;

    Ok(format!("Applied '{}' from '{}'", theme_entry.title, coll_name))
}
