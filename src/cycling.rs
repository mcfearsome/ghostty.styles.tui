use rand::Rng;

use crate::collection::{self, CycleOrder};
use crate::config;
use crate::darkmode;
use crate::theme::GhosttyConfig;

/// Advance to the next theme in the active collection and apply it.
/// Respects the global mode preference to filter themes.
pub fn apply_next() -> Result<String, String> {
    let app_config = collection::load_config();
    let coll_name = app_config
        .active_collection
        .ok_or("No active collection. Run: ghostty-styles collection use <name>")?;

    let mut coll = collection::load_collection(&coll_name)?;

    if coll.themes.is_empty() {
        return Err(format!("Collection '{}' is empty", coll_name));
    }

    // Clamp current_index in case the collection was modified externally
    if coll.current_index >= coll.themes.len() {
        coll.current_index = 0;
    }

    // Resolve mode filter
    let want_dark: Option<bool> = app_config.mode_preference.as_ref().and_then(|pref| {
        darkmode::resolve_mode(pref, &app_config.dark_after, &app_config.light_after)
    });

    // Build list of eligible indices
    let eligible: Vec<usize> = if let Some(dark) = want_dark {
        let filtered: Vec<usize> = coll
            .themes
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_dark == dark)
            .map(|(i, _)| i)
            .collect();
        if filtered.is_empty() {
            eprintln!(
                "[warning] No {} themes in '{}', ignoring mode filter",
                if dark { "dark" } else { "light" },
                coll_name
            );
            (0..coll.themes.len()).collect()
        } else {
            filtered
        }
    } else {
        (0..coll.themes.len()).collect()
    };

    // Find current position within eligible list
    let current_eligible_pos = eligible
        .iter()
        .position(|&i| i == coll.current_index)
        .unwrap_or(0);

    let next_eligible_pos = match coll.order {
        CycleOrder::Sequential => (current_eligible_pos + 1) % eligible.len(),
        CycleOrder::Shuffle => {
            let mut rng = rand::thread_rng();
            if eligible.len() == 1 {
                0
            } else {
                let mut next = current_eligible_pos;
                while next == current_eligible_pos {
                    next = rng.gen_range(0..eligible.len());
                }
                next
            }
        }
    };

    let next_index = eligible[next_eligible_pos];
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

    let mode_label = want_dark
        .map(|d| if d { " [dark]" } else { " [light]" })
        .unwrap_or("");
    Ok(format!("Applied '{}' from '{}'{}", theme_entry.title, coll_name, mode_label))
}
