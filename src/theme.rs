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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_valid_with_hash() {
        assert_eq!(GhosttyConfig::parse_hex("#ff00aa"), Some((255, 0, 170)));
    }

    #[test]
    fn parse_hex_valid_without_hash() {
        assert_eq!(GhosttyConfig::parse_hex("ff00aa"), Some((255, 0, 170)));
    }

    #[test]
    fn parse_hex_black() {
        assert_eq!(GhosttyConfig::parse_hex("#000000"), Some((0, 0, 0)));
    }

    #[test]
    fn parse_hex_white() {
        assert_eq!(GhosttyConfig::parse_hex("#ffffff"), Some((255, 255, 255)));
    }

    #[test]
    fn parse_hex_invalid_length() {
        assert_eq!(GhosttyConfig::parse_hex("#fff"), None);
    }

    #[test]
    fn parse_hex_invalid_chars() {
        assert_eq!(GhosttyConfig::parse_hex("#gggggg"), None);
    }

    #[test]
    fn parse_hex_empty() {
        assert_eq!(GhosttyConfig::parse_hex(""), None);
    }

    fn make_theme(bg: &str, fg: &str, palette: Vec<&str>) -> GhosttyConfig {
        GhosttyConfig {
            id: String::new(),
            slug: String::new(),
            title: String::new(),
            description: None,
            raw_config: String::new(),
            background: bg.to_string(),
            foreground: fg.to_string(),
            cursor_color: None,
            cursor_text: None,
            selection_bg: None,
            selection_fg: None,
            palette: palette.into_iter().map(String::from).collect(),
            font_family: None,
            font_size: None,
            cursor_style: None,
            bg_opacity: None,
            is_dark: true,
            tags: Vec::new(),
            source_url: None,
            author_name: None,
            author_url: None,
            is_featured: false,
            vote_count: 0,
            view_count: 0,
            download_count: 0,
        }
    }

    #[test]
    fn bg_color_valid() {
        let t = make_theme("#1a1b26", "#c0caf5", vec![]);
        assert_eq!(t.bg_color(), ratatui::style::Color::Rgb(26, 27, 38));
    }

    #[test]
    fn bg_color_invalid_returns_black() {
        let t = make_theme("not-a-color", "#c0caf5", vec![]);
        assert_eq!(t.bg_color(), ratatui::style::Color::Black);
    }

    #[test]
    fn fg_color_invalid_returns_white() {
        let t = make_theme("#1a1b26", "bad", vec![]);
        assert_eq!(t.fg_color(), ratatui::style::Color::White);
    }

    #[test]
    fn palette_color_valid() {
        let t = make_theme("#000", "#fff", vec!["#ff0000", "#00ff00"]);
        assert_eq!(t.palette_color(0), ratatui::style::Color::Rgb(255, 0, 0));
    }

    #[test]
    fn palette_color_out_of_bounds() {
        let t = make_theme("#000", "#fff", vec![]);
        assert_eq!(t.palette_color(5), ratatui::style::Color::Reset);
    }

    #[test]
    fn config_response_deserialize() {
        let json = r##"{
            "configs": [{
                "id": "1", "slug": "test", "title": "Test Theme",
                "rawConfig": "background = #000", "background": "#000000",
                "foreground": "#ffffff", "palette": [], "isDark": true,
                "tags": ["dark"], "isFeatured": false, "voteCount": 5,
                "viewCount": 100, "downloadCount": 50
            }],
            "total": 1, "page": 1, "perPage": 20, "totalPages": 1
        }"##;
        let resp: ConfigResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total, 1);
        assert_eq!(resp.configs.len(), 1);
        assert_eq!(resp.configs[0].title, "Test Theme");
        assert!(resp.configs[0].is_dark);
    }
}
