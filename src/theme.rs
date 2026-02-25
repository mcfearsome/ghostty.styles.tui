use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhosttyConfig {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub raw_config: String,
    pub background: String,
    pub foreground: String,
    pub cursor_color: Option<String>,
    pub cursor_text: Option<String>,
    pub selection_bg: Option<String>,
    pub selection_fg: Option<String>,
    pub palette: Vec<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub cursor_style: Option<String>,
    pub bg_opacity: Option<f64>,
    pub is_dark: bool,
    pub tags: Vec<String>,
    pub source_url: Option<String>,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub is_featured: bool,
    pub vote_count: i32,
    pub view_count: i32,
    pub download_count: i32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub configs: Vec<GhosttyConfig>,
    pub total: i32,
    pub page: i32,
    pub per_page: i32,
    pub total_pages: i32,
}

impl GhosttyConfig {
    /// Parse a hex color string like "#ff00aa" into (r, g, b)
    pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b))
    }

    pub fn bg_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.background)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Black)
    }

    pub fn fg_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.foreground)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn palette_color(&self, index: usize) -> ratatui::style::Color {
        self.palette
            .get(index)
            .and_then(|hex| Self::parse_hex(hex))
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Reset)
    }
}
