use std::sync::mpsc;
use std::thread;

use crate::api::{self, FetchParams, SortOrder};
use crate::preview::{self, SavedColors};
use crate::theme::{ConfigResponse, GhosttyConfig};

pub const AVAILABLE_TAGS: &[&str] = &[
    "dark",
    "light",
    "minimal",
    "colorful",
    "retro",
    "pastel",
    "high-contrast",
    "monochrome",
    "warm",
    "cool",
    "neon",
];

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Browse,
    Detail,
    Confirm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    TagSelect,
}

pub enum BgMessage {
    ConfigsLoaded(Result<ConfigResponse, String>),
}

pub struct App {
    pub screen: Screen,
    pub input_mode: InputMode,
    pub themes: Vec<GhosttyConfig>,
    pub selected: usize,
    pub list_offset: usize,
    pub search_input: String,
    pub active_query: Option<String>,
    pub active_tag: Option<String>,
    pub tag_cursor: usize,
    pub sort: SortOrder,
    pub dark_filter: Option<bool>,
    pub page: i32,
    pub total_pages: i32,
    pub total_results: i32,
    pub loading: bool,
    pub error: Option<String>,
    pub osc_preview_active: bool,
    pub saved_colors: Option<SavedColors>,
    pub status_message: Option<String>,
    pub should_quit: bool,
    pub bg_rx: mpsc::Receiver<BgMessage>,
    pub bg_tx: mpsc::Sender<BgMessage>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            screen: Screen::Browse,
            input_mode: InputMode::Normal,
            themes: Vec::new(),
            selected: 0,
            list_offset: 0,
            search_input: String::new(),
            active_query: None,
            active_tag: None,
            tag_cursor: 0,
            sort: SortOrder::Popular,
            dark_filter: None,
            page: 1,
            total_pages: 0,
            total_results: 0,
            loading: false,
            error: None,
            osc_preview_active: false,
            saved_colors: None,
            status_message: None,
            should_quit: false,
            bg_rx: rx,
            bg_tx: tx,
        }
    }

    pub fn selected_theme(&self) -> Option<&GhosttyConfig> {
        self.themes.get(self.selected)
    }

    pub fn trigger_fetch(&mut self) {
        self.loading = true;
        self.error = None;
        let params = FetchParams {
            query: self.active_query.clone(),
            tag: self.active_tag.clone(),
            sort: self.sort,
            page: self.page,
            dark: self.dark_filter,
        };
        let tx = self.bg_tx.clone();
        thread::spawn(move || {
            let result = api::fetch_configs(&params);
            let _ = tx.send(BgMessage::ConfigsLoaded(result));
        });
    }

    pub fn poll_background(&mut self) {
        while let Ok(msg) = self.bg_rx.try_recv() {
            match msg {
                BgMessage::ConfigsLoaded(Ok(resp)) => {
                    self.themes = resp.configs;
                    self.total_pages = resp.total_pages;
                    self.total_results = resp.total;
                    self.page = resp.page;
                    self.selected = 0;
                    self.list_offset = 0;
                    self.loading = false;
                }
                BgMessage::ConfigsLoaded(Err(e)) => {
                    self.error = Some(e);
                    self.loading = false;
                }
            }
        }
    }

    pub fn select_next(&mut self) {
        if !self.themes.is_empty() {
            self.selected = (self.selected + 1).min(self.themes.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn next_page(&mut self) {
        if self.page < self.total_pages {
            self.page += 1;
            self.trigger_fetch();
        }
    }

    pub fn prev_page(&mut self) {
        if self.page > 1 {
            self.page -= 1;
            self.trigger_fetch();
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.page = 1;
        self.trigger_fetch();
    }

    pub fn toggle_dark_filter(&mut self) {
        self.dark_filter = match self.dark_filter {
            None => Some(true),
            Some(true) => Some(false),
            Some(false) => None,
        };
        self.page = 1;
        self.trigger_fetch();
    }

    pub fn submit_search(&mut self) {
        self.active_query = if self.search_input.is_empty() {
            None
        } else {
            Some(self.search_input.clone())
        };
        self.page = 1;
        self.input_mode = InputMode::Normal;
        self.trigger_fetch();
    }

    pub fn select_tag(&mut self) {
        if self.tag_cursor < AVAILABLE_TAGS.len() {
            let tag = AVAILABLE_TAGS[self.tag_cursor];
            if self.active_tag.as_deref() == Some(tag) {
                self.active_tag = None;
            } else {
                self.active_tag = Some(tag.to_string());
            }
            self.page = 1;
            self.input_mode = InputMode::Normal;
            self.trigger_fetch();
        }
    }

    pub fn toggle_osc_preview(&mut self) {
        if self.osc_preview_active {
            // Restore colors
            if let Some(ref saved) = self.saved_colors {
                preview::restore_colors(saved);
            }
            self.osc_preview_active = false;
            self.saved_colors = None;
            self.status_message = Some("Preview off - colors restored".into());
        } else if let Some(theme) = self.themes.get(self.selected) {
            // Save and apply
            self.saved_colors = Some(preview::save_current_colors());
            preview::apply_osc_preview(theme);
            self.osc_preview_active = true;
            self.status_message = Some(format!("Live preview: {}", theme.title));
        }
    }

    pub fn apply_theme(&mut self) {
        if let Some(theme) = self.themes.get(self.selected).cloned() {
            match crate::config::apply_theme(&theme) {
                Ok(path) => {
                    self.status_message = Some(format!("Applied '{}' to {}", theme.title, path));
                    self.screen = Screen::Browse;
                }
                Err(e) => {
                    self.status_message = Some(format!("Error: {}", e));
                    self.screen = Screen::Browse;
                }
            }
        }
    }

    /// Ensure OSC colors are restored before exiting.
    pub fn cleanup(&mut self) {
        if self.osc_preview_active {
            if let Some(ref saved) = self.saved_colors {
                preview::restore_colors(saved);
            }
        }
    }
}
