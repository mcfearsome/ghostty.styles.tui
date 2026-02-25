use crate::theme::{ConfigResponse, GhosttyConfig};

const BASE_URL: &str = "https://ghostty-style.vercel.app/api/configs";

#[derive(Debug, Clone)]
pub struct FetchParams {
    pub query: Option<String>,
    pub tag: Option<String>,
    pub sort: SortOrder,
    pub page: i32,
    pub dark: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Popular,
    Newest,
    Trending,
}

impl SortOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortOrder::Popular => "popular",
            SortOrder::Newest => "newest",
            SortOrder::Trending => "trending",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SortOrder::Popular => "Popular",
            SortOrder::Newest => "Newest",
            SortOrder::Trending => "Trending",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SortOrder::Popular => SortOrder::Newest,
            SortOrder::Newest => SortOrder::Trending,
            SortOrder::Trending => SortOrder::Popular,
        }
    }
}

impl Default for FetchParams {
    fn default() -> Self {
        Self {
            query: None,
            tag: None,
            sort: SortOrder::Popular,
            page: 1,
            dark: None,
        }
    }
}

pub fn fetch_configs(params: &FetchParams) -> Result<ConfigResponse, String> {
    let client = reqwest::blocking::Client::new();
    let mut url = format!("{}?sort={}&page={}", BASE_URL, params.sort.as_str(), params.page);

    if let Some(ref q) = params.query {
        if !q.is_empty() {
            url.push_str(&format!("&q={}", urlencoding(&q)));
        }
    }
    if let Some(ref tag) = params.tag {
        url.push_str(&format!("&tag={}", tag));
    }
    if let Some(dark) = params.dark {
        url.push_str(&format!("&dark={}", dark));
    }

    let resp = client
        .get(&url)
        .header("User-Agent", "ghostty-styles-tui/0.1")
        .send()
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API error: {}", resp.status()));
    }

    resp.json::<ConfigResponse>()
        .map_err(|e| format!("Parse error: {}", e))
}

#[allow(dead_code)]
pub fn fetch_config_by_id(id: &str) -> Result<GhosttyConfig, String> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/{}", BASE_URL, id);

    let resp = client
        .get(&url)
        .header("User-Agent", "ghostty-styles-tui/0.1")
        .send()
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API error: {}", resp.status()));
    }

    resp.json::<GhosttyConfig>()
        .map_err(|e| format!("Parse error: {}", e))
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}
