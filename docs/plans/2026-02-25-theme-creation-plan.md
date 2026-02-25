# Theme Creation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a full-screen theme creator with HSL color picker, mouse support, palette auto-generation, and export/upload to the ghostty-style website.

**Architecture:** New `Screen::Create` and `Screen::CreateMeta` with a `CreatorState` struct owning all editor state. HSL↔Hex color math in `creator.rs`, rendering in `ui/creator.rs` and `ui/create_meta.rs`, export/upload in `export.rs`. Mouse capture enabled globally but only handled on the Create screen.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28 (mouse events), no new crate dependencies

---

### Task 1: CreatorState data model and HSL↔Hex math

**Files:**
- Create: `src/creator.rs`

**Context:** This is the core data model for the theme editor. All color state is stored as HSL. The struct needs to support 22 color fields (bg, fg, cursor-color, cursor-text, selection-bg, selection-fg, palette 0-15), track which field is selected, what edit mode is active, and whether the user has made manual palette edits.

**Step 1: Create `src/creator.rs` with the data model and color conversion**

```rust
/// Color stored as HSL (hue 0-360, saturation 0-100, lightness 0-100)
#[derive(Debug, Clone, Copy)]
pub struct HslColor {
    pub h: f64, // 0.0 - 360.0
    pub s: f64, // 0.0 - 100.0
    pub l: f64, // 0.0 - 100.0
}

impl HslColor {
    pub fn new(h: f64, s: f64, l: f64) -> Self {
        Self {
            h: h.clamp(0.0, 360.0),
            s: s.clamp(0.0, 100.0),
            l: l.clamp(0.0, 100.0),
        }
    }

    /// Convert HSL to RGB (r, g, b each 0-255)
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        let h = self.h / 360.0;
        let s = self.s / 100.0;
        let l = self.l / 100.0;

        if s == 0.0 {
            let v = (l * 255.0).round() as u8;
            return (v, v, v);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

        (
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        )
    }

    /// Convert HSL to hex string like "#1e1e2e"
    pub fn to_hex(&self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    /// Convert from RGB (0-255 each) to HSL
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < f64::EPSILON {
            return Self::new(0.0, 0.0, l * 100.0);
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if (max - r).abs() < f64::EPSILON {
            let mut h = (g - b) / d;
            if g < b {
                h += 6.0;
            }
            h
        } else if (max - g).abs() < f64::EPSILON {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        Self::new(h * 60.0, s * 100.0, l * 100.0)
    }

    /// Parse a hex string like "#1e1e2e" into HSL
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self::from_rgb(r, g, b))
    }

    pub fn to_ratatui_color(&self) -> ratatui::style::Color {
        let (r, g, b) = self.to_rgb();
        ratatui::style::Color::Rgb(r, g, b)
    }
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Which color field is being edited
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorField {
    Background,
    Foreground,
    CursorColor,
    CursorText,
    SelectionBg,
    SelectionFg,
    Palette(usize), // 0-15
}

impl ColorField {
    /// All fields in display order
    pub fn all() -> Vec<ColorField> {
        let mut fields = vec![
            ColorField::Background,
            ColorField::Foreground,
            ColorField::CursorColor,
            ColorField::CursorText,
            ColorField::SelectionBg,
            ColorField::SelectionFg,
        ];
        for i in 0..16 {
            fields.push(ColorField::Palette(i));
        }
        fields
    }

    pub fn label(&self) -> String {
        match self {
            ColorField::Background => "background".to_string(),
            ColorField::Foreground => "foreground".to_string(),
            ColorField::CursorColor => "cursor-color".to_string(),
            ColorField::CursorText => "cursor-text".to_string(),
            ColorField::SelectionBg => "select-bg".to_string(),
            ColorField::SelectionFg => "select-fg".to_string(),
            ColorField::Palette(i) => format!("palette {}", i),
        }
    }
}

/// Which HSL slider is focused
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SliderFocus {
    Hue,
    Saturation,
    Lightness,
}

/// Whether the picker is in slider or hex mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PickerMode {
    Slider,
    HexInput,
}

/// Palette generation algorithm
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GenAlgorithm {
    HueRotation,
    Base16,
}

impl GenAlgorithm {
    pub fn toggle(&self) -> Self {
        match self {
            GenAlgorithm::HueRotation => GenAlgorithm::Base16,
            GenAlgorithm::Base16 => GenAlgorithm::HueRotation,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            GenAlgorithm::HueRotation => "hue rotation",
            GenAlgorithm::Base16 => "base16",
        }
    }
}

pub struct CreatorState {
    /// Title for the theme (entered at start)
    pub title: String,
    /// All 22 color values in HSL
    pub colors: Vec<HslColor>,
    /// Currently selected field index (into ColorField::all())
    pub field_index: usize,
    /// Whether we're in field editing mode
    pub editing: bool,
    /// Slider vs hex input
    pub picker_mode: PickerMode,
    /// Which HSL slider is active
    pub slider_focus: SliderFocus,
    /// Hex input buffer
    pub hex_input: String,
    /// Palette generation algorithm
    pub gen_algorithm: GenAlgorithm,
    /// OSC live preview on/off (default off)
    pub osc_preview: bool,
    /// Whether manual palette edits have been made
    pub palette_dirty: bool,
    /// Whether any changes have been made at all
    pub unsaved: bool,
    /// Source theme slug if forked
    pub forked_from: Option<String>,
    /// Field list scroll offset (for rendering)
    pub field_scroll: usize,
}

impl CreatorState {
    /// Create a new blank creator state (from scratch)
    pub fn new(title: String) -> Self {
        let default_bg = HslColor::new(240.0, 21.0, 15.0);   // dark blue-gray
        let default_fg = HslColor::new(226.0, 64.0, 88.0);   // light blue-white
        let mut colors = Vec::with_capacity(22);
        colors.push(default_bg); // background
        colors.push(default_fg); // foreground
        // cursor-color: derive from fg
        colors.push(default_fg);
        // cursor-text: derive from bg
        colors.push(default_bg);
        // selection-bg: derive from fg, lower lightness
        colors.push(HslColor::new(default_fg.h, default_fg.s * 0.5, 30.0));
        // selection-fg: same as fg
        colors.push(default_fg);
        // palette 0-15: empty, will be auto-generated
        for _ in 0..16 {
            colors.push(HslColor::new(0.0, 0.0, 50.0));
        }
        let mut state = Self {
            title,
            colors,
            field_index: 0,
            editing: false,
            picker_mode: PickerMode::Slider,
            slider_focus: SliderFocus::Hue,
            hex_input: String::new(),
            gen_algorithm: GenAlgorithm::HueRotation,
            osc_preview: false,
            palette_dirty: false,
            unsaved: false,
            forked_from: None,
            field_scroll: 0,
        };
        // Auto-generate initial palette
        state.generate_palette();
        state
    }

    /// Create from an existing GhosttyConfig (fork)
    pub fn from_theme(theme: &crate::theme::GhosttyConfig) -> Self {
        let bg = HslColor::from_hex(&theme.background).unwrap_or(HslColor::new(0.0, 0.0, 15.0));
        let fg = HslColor::from_hex(&theme.foreground).unwrap_or(HslColor::new(0.0, 0.0, 85.0));
        let cursor = theme.cursor_color.as_deref()
            .and_then(HslColor::from_hex)
            .unwrap_or(fg);
        let cursor_text = theme.cursor_text.as_deref()
            .and_then(HslColor::from_hex)
            .unwrap_or(bg);
        let sel_bg = theme.selection_bg.as_deref()
            .and_then(HslColor::from_hex)
            .unwrap_or(HslColor::new(fg.h, fg.s * 0.5, 30.0));
        let sel_fg = theme.selection_fg.as_deref()
            .and_then(HslColor::from_hex)
            .unwrap_or(fg);

        let mut colors = vec![bg, fg, cursor, cursor_text, sel_bg, sel_fg];
        for i in 0..16 {
            let c = theme.palette.get(i)
                .and_then(|h| HslColor::from_hex(h))
                .unwrap_or(HslColor::new(0.0, 0.0, 50.0));
            colors.push(c);
        }

        Self {
            title: format!("{} (fork)", theme.title),
            colors,
            field_index: 0,
            editing: false,
            picker_mode: PickerMode::Slider,
            slider_focus: SliderFocus::Hue,
            hex_input: String::new(),
            gen_algorithm: GenAlgorithm::HueRotation,
            osc_preview: false,
            palette_dirty: true, // forked palette counts as manual
            unsaved: false,
            forked_from: Some(theme.slug.clone()),
            field_scroll: 0,
        }
    }

    /// Get the current field enum
    pub fn current_field(&self) -> ColorField {
        ColorField::all()[self.field_index]
    }

    /// Get mutable reference to the color for the current field
    pub fn current_color(&self) -> &HslColor {
        &self.colors[self.field_index]
    }

    /// Set the color for the current field
    pub fn set_current_color(&mut self, color: HslColor) {
        self.colors[self.field_index] = color;
        self.unsaved = true;
        // Track if a palette field was manually edited
        if self.field_index >= 6 {
            self.palette_dirty = true;
        }
    }

    /// Adjust the active slider value
    pub fn adjust_slider(&mut self, delta: f64) {
        let mut color = self.colors[self.field_index];
        match self.slider_focus {
            SliderFocus::Hue => color.h = (color.h + delta).rem_euclid(360.0),
            SliderFocus::Saturation => color.s = (color.s + delta).clamp(0.0, 100.0),
            SliderFocus::Lightness => color.l = (color.l + delta).clamp(0.0, 100.0),
        }
        self.colors[self.field_index] = color;
        self.unsaved = true;
        if self.field_index >= 6 {
            self.palette_dirty = true;
        }
    }

    /// Commit hex input to the current color
    pub fn commit_hex_input(&mut self) {
        let input = if self.hex_input.starts_with('#') {
            self.hex_input.clone()
        } else {
            format!("#{}", self.hex_input)
        };
        if let Some(color) = HslColor::from_hex(&input) {
            self.set_current_color(color);
        }
    }

    /// Sync hex input buffer from current color
    pub fn sync_hex_from_color(&mut self) {
        self.hex_input = self.colors[self.field_index].to_hex();
    }

    /// Auto-derive cursor/selection from bg/fg
    pub fn auto_derive(&mut self) {
        let bg = self.colors[0];
        let fg = self.colors[1];
        // cursor-color = fg
        self.colors[2] = fg;
        // cursor-text = bg
        self.colors[3] = bg;
        // selection-bg
        self.colors[4] = HslColor::new(fg.h, fg.s * 0.5, if bg.l < 50.0 { 30.0 } else { 70.0 });
        // selection-fg = fg
        self.colors[5] = fg;
    }

    /// Generate palette from bg/fg using current algorithm
    pub fn generate_palette(&mut self) {
        let bg = self.colors[0];
        let fg = self.colors[1];
        let is_dark = bg.l < 50.0;

        match self.gen_algorithm {
            GenAlgorithm::HueRotation => self.gen_hue_rotation(bg, fg, is_dark),
            GenAlgorithm::Base16 => self.gen_base16(bg, fg, is_dark),
        }
    }

    fn gen_hue_rotation(&mut self, bg: HslColor, fg: HslColor, is_dark: bool) {
        let anchor_hue = fg.h;
        // 6 accent hues at 60° intervals
        let accent_hues = [
            (anchor_hue + 0.0).rem_euclid(360.0),   // red-ish
            (anchor_hue + 60.0).rem_euclid(360.0),   // yellow-ish
            (anchor_hue + 120.0).rem_euclid(360.0),  // green-ish
            (anchor_hue + 180.0).rem_euclid(360.0),  // cyan-ish
            (anchor_hue + 240.0).rem_euclid(360.0),  // blue-ish
            (anchor_hue + 300.0).rem_euclid(360.0),  // magenta-ish
        ];
        let (normal_l, bright_l) = if is_dark { (55.0, 70.0) } else { (40.0, 30.0) };
        let sat = 60.0;

        // Color 0: darkened bg (or lightened for light themes)
        self.colors[6] = HslColor::new(bg.h, bg.s, if is_dark { bg.l + 8.0 } else { bg.l - 8.0 });
        // Colors 1-6: accents at normal lightness
        for (i, &hue) in accent_hues.iter().enumerate() {
            self.colors[7 + i] = HslColor::new(hue, sat, normal_l);
        }
        // Color 7: dimmed fg
        self.colors[13] = HslColor::new(fg.h, fg.s * 0.5, if is_dark { fg.l - 15.0 } else { fg.l + 15.0 });
        // Color 8: lighter than color 0
        self.colors[14] = HslColor::new(bg.h, bg.s, if is_dark { bg.l + 16.0 } else { bg.l - 16.0 });
        // Colors 9-14: accents at bright lightness
        for (i, &hue) in accent_hues.iter().enumerate() {
            self.colors[15 + i] = HslColor::new(hue, sat, bright_l);
        }
        // Color 15: fg
        self.colors[21] = fg;
    }

    fn gen_base16(&mut self, bg: HslColor, fg: HslColor, is_dark: bool) {
        // Grayscale ramp
        let (l0, l7, l8, l15) = if is_dark {
            (bg.l + 5.0, fg.l - 20.0, bg.l + 15.0, fg.l)
        } else {
            (bg.l - 5.0, fg.l + 20.0, bg.l - 15.0, fg.l)
        };
        self.colors[6] = HslColor::new(bg.h, bg.s, l0.clamp(0.0, 100.0));
        self.colors[13] = HslColor::new(fg.h, fg.s * 0.3, l7.clamp(0.0, 100.0));
        self.colors[14] = HslColor::new(bg.h, bg.s, l8.clamp(0.0, 100.0));
        self.colors[21] = HslColor::new(fg.h, fg.s, l15.clamp(0.0, 100.0));

        // Canonical terminal hues
        let hues = [0.0, 120.0, 60.0, 240.0, 300.0, 180.0]; // red, green, yellow, blue, magenta, cyan
        let sat = 65.0;
        let normal_l = if is_dark { 55.0 } else { 45.0 };
        let bright_l = if is_dark { 70.0 } else { 35.0 };

        for (i, &hue) in hues.iter().enumerate() {
            self.colors[7 + i] = HslColor::new(hue, sat, normal_l);
            self.colors[15 + i] = HslColor::new(hue, sat, bright_l);
        }
    }

    /// Detect if theme is dark based on background lightness
    pub fn is_dark(&self) -> bool {
        self.colors[0].l < 50.0
    }

    /// Build a GhosttyConfig for preview rendering
    pub fn build_preview_config(&self) -> crate::theme::GhosttyConfig {
        let palette: Vec<String> = (0..16).map(|i| self.colors[6 + i].to_hex()).collect();
        crate::theme::GhosttyConfig {
            id: String::new(),
            slug: slug_from_title(&self.title),
            title: self.title.clone(),
            description: None,
            raw_config: self.build_raw_config(),
            background: self.colors[0].to_hex(),
            foreground: self.colors[1].to_hex(),
            cursor_color: Some(self.colors[2].to_hex()),
            cursor_text: Some(self.colors[3].to_hex()),
            selection_bg: Some(self.colors[4].to_hex()),
            selection_fg: Some(self.colors[5].to_hex()),
            palette,
            font_family: None,
            font_size: None,
            cursor_style: None,
            bg_opacity: None,
            is_dark: self.is_dark(),
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

    /// Build raw_config string in Ghostty config format
    pub fn build_raw_config(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("background = {}", self.colors[0].to_hex()));
        lines.push(format!("foreground = {}", self.colors[1].to_hex()));
        lines.push(format!("cursor-color = {}", self.colors[2].to_hex()));
        lines.push(format!("cursor-text = {}", self.colors[3].to_hex()));
        lines.push(format!("selection-background = {}", self.colors[4].to_hex()));
        lines.push(format!("selection-foreground = {}", self.colors[5].to_hex()));
        for i in 0..16 {
            lines.push(format!("palette = {}={}", i, self.colors[6 + i].to_hex()));
        }
        lines.join("\n")
    }
}

/// Convert title to URL-friendly slug
fn slug_from_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}
```

**Step 2: Add `mod creator;` to `src/main.rs`**

Add `mod creator;` after `mod config;` in the module declarations at the top of `src/main.rs`.

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles with no errors (warnings about unused code are OK)

**Step 4: Commit**

```bash
git add src/creator.rs src/main.rs
git commit -m "feat: add CreatorState data model with HSL color math"
```

---

### Task 2: Export module (save, apply, open browser)

**Files:**
- Create: `src/export.rs`

**Context:** This module handles three actions after theme creation: applying to Ghostty config (reuses `config::apply_theme`), exporting to a `.conf` file, and opening the upload page in the browser. The themes directory is `~/.config/ghostty-styles/themes/`.

**Step 1: Create `src/export.rs`**

```rust
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::collection;
use crate::config;
use crate::creator::CreatorState;
use crate::theme::GhosttyConfig;

/// Directory for exported theme files
fn themes_dir() -> PathBuf {
    collection::base_dir().join("themes")
}

/// Export a theme to a .conf file in the themes directory
pub fn export_theme(state: &CreatorState) -> Result<String, String> {
    let dir = themes_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create themes dir: {}", e))?;

    let slug = slug_from_title(&state.title);
    let filename = format!("{}.conf", slug);
    let path = dir.join(&filename);

    let raw_config = state.build_raw_config();
    fs::write(&path, &raw_config).map_err(|e| format!("Failed to write theme file: {}", e))?;

    Ok(path.display().to_string())
}

/// Apply the created theme to the Ghostty config
pub fn apply_created_theme(state: &CreatorState) -> Result<String, String> {
    let preview = state.build_preview_config();
    config::apply_theme(&preview)
}

/// Export theme file and open the upload page in the default browser
pub fn upload_theme(state: &CreatorState) -> Result<String, String> {
    let path = export_theme(state)?;

    let url = "https://ghostty-style.vercel.app/upload";
    open_url(url).map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(format!(
        "Config saved to {}. Upload page opened — drag the file to submit.",
        path
    ))
}

/// Open a URL in the default browser
fn open_url(url: &str) -> Result<(), String> {
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else {
        Command::new("xdg-open").arg(url).status()
    };

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("Browser exited with status: {}", status)),
        Err(e) => Err(format!("{}", e)),
    }
}

fn slug_from_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}
```

**Step 2: Add `mod export;` to `src/main.rs`**

Add `mod export;` after `mod daemon;`.

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/export.rs src/main.rs
git commit -m "feat: add export module for save, apply, and upload"
```

---

### Task 3: App state changes (Screen::Create, Screen::CreateMeta, CreatorState field)

**Files:**
- Modify: `src/app.rs`

**Context:** Add two new screen variants and a `CreatorState` field to `App`. Add methods `enter_creator()` and `enter_creator_from_theme()`. The `Screen` enum is at line 23-28, `App` struct at line 51-84, `impl App` at line 86.

**Step 1: Add new Screen variants**

In `src/app.rs`, add `Create` and `CreateMeta` to the `Screen` enum (after `Collections`):

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Browse,
    Detail,
    Confirm,
    Collections,
    Create,
    CreateMeta,
}
```

**Step 2: Add `creator_state` field to `App`**

Add to the `App` struct (after `collections_input`):

```rust
    pub creator_state: Option<crate::creator::CreatorState>,
```

And initialize it as `None` in `App::new()`:

```rust
            creator_state: None,
```

**Step 3: Add entry methods**

Add these methods to `impl App` (after `load_selected_collection`):

```rust
    pub fn enter_creator(&mut self, title: String) {
        self.creator_state = Some(crate::creator::CreatorState::new(title));
        self.screen = Screen::Create;
    }

    pub fn enter_creator_from_theme(&mut self) {
        if let Some(theme) = self.selected_theme() {
            self.creator_state = Some(crate::creator::CreatorState::from_theme(theme));
            self.screen = Screen::Create;
        }
    }

    pub fn enter_create_meta(&mut self) {
        self.screen = Screen::CreateMeta;
    }
```

**Step 4: Verify it compiles**

Run: `cargo build`

**Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: add Create/CreateMeta screens and creator state to App"
```

---

### Task 4: CLI `create` command

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

**Context:** Add a `Create` variant to the `Commands` enum with optional `--from` flag. CLI `create` without `--from` launches the TUI in creator mode. With `--from <slug>`, it fetches the theme from the API first. The `--apply`, `--export`, `--upload` flags are post-creation actions (used only after the TUI creator saves).

**Step 1: Add `Create` command to `cli.rs`**

In the `Commands` enum, add after `Cycle`:

```rust
    /// Create a new theme
    Create {
        /// Fork from an existing theme by slug
        #[arg(long)]
        from: Option<String>,
    },
```

**Step 2: Handle `Create` in `dispatch_command` in `main.rs`**

In `dispatch_command`, add the match arm before the closing brace:

```rust
        Commands::Create { from } => {
            run_tui_create(from);
        }
```

Add the `run_tui_create` function after `run_tui`:

```rust
fn run_tui_create(from_slug: Option<String>) {
    // Ghostty detection
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    if term_program.to_lowercase() != "ghostty" {
        eprintln!("ghostty-styles requires the Ghostty terminal.");
        std::process::exit(1);
    }

    // If forking, fetch the source theme first
    let source_theme = if let Some(ref slug) = from_slug {
        match api::fetch_config_by_id(slug) {
            Ok(theme) => Some(theme),
            Err(e) => {
                eprintln!("Error fetching theme '{}': {}", slug, e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // Setup terminal
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)
        .expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut app = App::new();
    match source_theme {
        Some(theme) => {
            app.creator_state = Some(creator::CreatorState::from_theme(&theme));
            app.screen = Screen::Create;
        }
        None => {
            app.creator_state = Some(creator::CreatorState::new("Untitled".to_string()));
            app.screen = Screen::Create;
        }
    }

    let result = run_app(&mut terminal, &mut app);

    // Cleanup
    app.cleanup();
    disable_raw_mode().expect("Failed to disable raw mode");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )
    .expect("Failed to leave alternate screen");

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

**Step 3: Update imports in `main.rs`**

Update the `use cli::` import to include the new variant. No change needed since `Commands::Create` is matched via the `Commands` import.

**Step 4: Verify it compiles**

Run: `cargo build`

**Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add create CLI command with --from flag"
```

---

### Task 5: Mouse capture and event routing in main.rs

**Files:**
- Modify: `src/main.rs`

**Context:** Enable mouse capture in the terminal setup for `run_tui`. Route `Event::Mouse` events only when `app.screen == Screen::Create`. Add placeholder `handle_create_input` and `handle_create_meta_input` functions. Also add `n` keybinding on Browse (currently `n` is next page — we need to remap) and `f` on Detail.

**Important:** `n` is currently bound to `next_page` on Browse. We need to remap: use `]` for next page and `[` for prev page (or keep `N`/`n` and use something else for create). Let's use `Ctrl+n` for new theme to avoid conflicts, or better: check the design — it says `n` on Browse. Let's remap pagination to `]`/`[` and use `n` for new theme.

**Step 1: Enable mouse capture in `run_tui`**

In `run_tui`, change the `execute!` call to include `EnableMouseCapture`:

```rust
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)
        .expect("Failed to enter alternate screen");
```

And the cleanup:

```rust
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )
    .expect("Failed to leave alternate screen");
```

**Step 2: Add mouse event handling in `run_app`**

Change the event read block in `run_app` to handle both Key and Mouse events:

```rust
        if event::poll(Duration::from_millis(50))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    app.status_message = None;
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        app.should_quit = true;
                    }
                    match app.screen {
                        Screen::Browse => handle_browse_input(app, key.code),
                        Screen::Detail => handle_detail_input(app, key.code),
                        Screen::Confirm => handle_confirm_input(app, key.code),
                        Screen::Collections => handle_collections_input(app, key.code),
                        Screen::Create => handle_create_input(app, key.code, key.modifiers),
                        Screen::CreateMeta => handle_create_meta_input(app, key.code),
                    }
                }
                Event::Mouse(mouse) => {
                    if app.screen == Screen::Create {
                        handle_create_mouse(app, mouse);
                    }
                }
                _ => {}
            }
        }
```

**Step 3: Add draw routing for new screens**

In the `terminal.draw` closure, add the new screens:

```rust
        terminal.draw(|f| match app.screen {
            Screen::Browse => ui::render_browser(f, app),
            Screen::Detail | Screen::Confirm => ui::render_detail(f, app),
            Screen::Collections => ui::render_collections(f, app),
            Screen::Create => ui::render_creator(f, app),
            Screen::CreateMeta => ui::render_create_meta(f, app),
        })?;
```

**Step 4: Remap browse pagination and add `n` for create**

In `handle_browse_input`, `InputMode::Normal` arm:
- Change `KeyCode::Char('n')` from `app.next_page()` to entering creator (prompt for title).
- Add `KeyCode::Char(']')` for next page and `KeyCode::Char('[')` for prev page.
- Remove the existing `KeyCode::Char('N')` prev_page mapping.

```rust
            KeyCode::Char('n') => {
                app.enter_creator("Untitled".to_string());
            }
            KeyCode::Char(']') => app.next_page(),
            KeyCode::Char('[') => app.prev_page(),
```

**Step 5: Add `f` keybinding on Detail screen**

In `handle_detail_input`, add:

```rust
        KeyCode::Char('f') => {
            app.enter_creator_from_theme();
        }
```

**Step 6: Add placeholder handler functions**

Add these stub functions at the end of `main.rs`:

```rust
fn handle_create_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    let _ = (app, key, modifiers);
    // TODO: Task 7 implements this
}

fn handle_create_meta_input(app: &mut App, key: KeyCode) {
    let _ = (app, key);
    // TODO: Task 9 implements this
}

fn handle_create_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    let _ = (app, mouse);
    // TODO: Task 8 implements this
}
```

**Step 7: Add necessary imports**

Add `crossterm::event::EnableMouseCapture` and `crossterm::event::DisableMouseCapture` and `crossterm::event::MouseEvent` to the crossterm imports. Update the `use app::` import to include the new Screen variants.

**Step 8: Verify it compiles**

Run: `cargo build`

**Step 9: Commit**

```bash
git add src/main.rs
git commit -m "feat: add mouse capture, create screen routing, and n/f keybindings"
```

---

### Task 6: Creator screen rendering (ui/creator.rs)

**Files:**
- Create: `src/ui/creator.rs`
- Modify: `src/ui/mod.rs`

**Context:** This is the main creator screen UI. Three-column layout: color field list (25%), HSL picker (35%), theme preview (40%). Uses the existing `ThemePreview` widget for the right panel. The HSL sliders are rendered as colored `█` blocks. A color swatch is shown below the sliders. The hex input field is shown below the sliders when in hex mode.

**Step 1: Create `src/ui/creator.rs`**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::creator::{ColorField, HslColor, PickerMode, SliderFocus};
use crate::ui::preview::ThemePreview;

const ACCENT: Color = Color::Rgb(187, 154, 247);
const DIM: Color = Color::Rgb(100, 100, 120);

/// Render the full creator screen
pub fn render_creator(f: &mut Frame, app: &App) {
    let state = match &app.creator_state {
        Some(s) => s,
        None => return,
    };

    let area = f.area();

    // Outer layout: top bar, main content, bottom bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    // Top bar
    render_top_bar(f, outer[0], state);

    // Main: three columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(outer[1]);

    // Left: color field list
    render_field_list(f, cols[0], state);

    // Center: HSL picker
    render_picker(f, cols[1], state);

    // Right: theme preview
    let preview_config = state.build_preview_config();
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" Preview ", Style::default().fg(ACCENT)));
    let inner = preview_block.inner(cols[2]);
    f.render_widget(preview_block, cols[2]);
    f.render_widget(ThemePreview { theme: &preview_config }, inner);

    // Bottom hints bar
    render_bottom_bar(f, outer[2], state);
}

fn render_top_bar(f: &mut Frame, area: Rect, state: &crate::creator::CreatorState) {
    let osc_indicator = if state.osc_preview {
        Span::styled(" [OSC ON] ", Style::default().fg(Color::Green))
    } else {
        Span::styled("", Style::default())
    };
    let title_line = Line::from(vec![
        Span::styled(" Create Theme: ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(&state.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" ", Style::default()),
        osc_indicator,
    ]);
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM));
    let p = Paragraph::new(title_line).block(block);
    f.render_widget(p, area);
}

fn render_field_list(f: &mut Frame, area: Rect, state: &crate::creator::CreatorState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" Colors ", Style::default().fg(ACCENT)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let fields = ColorField::all();
    let visible_height = inner.height as usize;

    // Ensure selected field is visible
    let scroll = state.field_scroll;

    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in fields.iter().enumerate().skip(scroll).take(visible_height) {
        let selected = i == state.field_index;
        let color = state.colors[i];
        let hex = color.to_hex();
        let marker = if selected { "> " } else { "  " };
        let label = field.label();

        let style = if selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };

        let swatch_color = color.to_ratatui_color();

        lines.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled("██", Style::default().fg(swatch_color)),
            Span::styled(" ", style),
            Span::styled(format!("{:<12}", label), style),
            Span::styled(&hex, Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Generation algorithm indicator at bottom
    if lines.len() < visible_height {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  gen: ", Style::default().fg(DIM)),
            Span::styled(state.gen_algorithm.label(), Style::default().fg(ACCENT)),
        ]));
    }

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn render_picker(f: &mut Frame, area: Rect, state: &crate::creator::CreatorState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(
            if state.picker_mode == PickerMode::Slider { " HSL Slider " } else { " Hex Input " },
            Style::default().fg(ACCENT),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let color = *state.current_color();
    let mut y = inner.y;
    let w = inner.width.saturating_sub(10) as usize; // bar width, leaving room for label+value

    if state.editing {
        match state.picker_mode {
            PickerMode::Slider => {
                // Hue slider
                render_slider_bar(f, inner.x, y, w, inner.width,
                    "H", color.h, 0.0, 360.0,
                    state.slider_focus == SliderFocus::Hue,
                    |frac| {
                        let h = frac * 360.0;
                        HslColor::new(h, 100.0, 50.0).to_ratatui_color()
                    });
                y += 1;

                // Saturation slider
                render_slider_bar(f, inner.x, y, w, inner.width,
                    "S", color.s, 0.0, 100.0,
                    state.slider_focus == SliderFocus::Saturation,
                    |frac| {
                        HslColor::new(color.h, frac * 100.0, color.l).to_ratatui_color()
                    });
                y += 1;

                // Lightness slider
                render_slider_bar(f, inner.x, y, w, inner.width,
                    "L", color.l, 0.0, 100.0,
                    state.slider_focus == SliderFocus::Lightness,
                    |frac| {
                        HslColor::new(color.h, color.s, frac * 100.0).to_ratatui_color()
                    });
                y += 2;
            }
            PickerMode::HexInput => {
                let input_line = Line::from(vec![
                    Span::styled("  Hex: ", Style::default().fg(DIM)),
                    Span::styled(&state.hex_input, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled("_", Style::default().fg(ACCENT)),
                ]);
                let p = Paragraph::new(input_line);
                f.render_widget(p, Rect::new(inner.x, y, inner.width, 1));
                y += 2;
            }
        }
    } else {
        // Not editing — show current color info
        let info = Line::from(vec![
            Span::styled("  Press Enter to edit", Style::default().fg(DIM)),
        ]);
        let p = Paragraph::new(info);
        f.render_widget(p, Rect::new(inner.x, y, inner.width, 1));
        y += 2;
    }

    // Color swatch (large)
    let swatch_color = color.to_ratatui_color();
    let swatch_height = 4.min(inner.y + inner.height - y);
    for sy in 0..swatch_height {
        for sx in 2..(inner.width.saturating_sub(2)) {
            if y + sy < inner.y + inner.height && inner.x + sx < inner.x + inner.width {
                f.buffer_mut()[(inner.x + sx, y + sy)]
                    .set_style(Style::default().bg(swatch_color));
                f.buffer_mut()[(inner.x + sx, y + sy)].set_char(' ');
            }
        }
    }
    y += swatch_height + 1;

    // Hex value display
    if y < inner.y + inner.height {
        let hex_line = Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(color.to_hex(), Style::default().fg(Color::White)),
            Span::styled(
                format!("  H:{:.0} S:{:.0} L:{:.0}", color.h, color.s, color.l),
                Style::default().fg(DIM),
            ),
        ]);
        let p = Paragraph::new(hex_line);
        f.render_widget(p, Rect::new(inner.x, y, inner.width, 1));
    }
}

fn render_slider_bar(
    f: &mut Frame,
    x: u16,
    y: u16,
    bar_width: usize,
    total_width: u16,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    focused: bool,
    color_fn: impl Fn(f64) -> Color,
) {
    let label_style = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    };

    // Label
    let mut spans = vec![
        Span::styled(format!("  {}: ", label), label_style),
    ];

    // Bar
    let frac = (value - min) / (max - min);
    let marker_pos = (frac * bar_width as f64).round() as usize;

    for i in 0..bar_width {
        let segment_frac = i as f64 / bar_width as f64;
        let c = color_fn(segment_frac);
        if i == marker_pos {
            spans.push(Span::styled("░", Style::default().fg(Color::White).bg(c)));
        } else {
            spans.push(Span::styled("█", Style::default().fg(c)));
        }
    }

    // Value
    let value_str = if max > 200.0 {
        format!(" {:.0}°", value)
    } else {
        format!(" {:.0}%", value)
    };
    spans.push(Span::styled(value_str, label_style));

    let line = Line::from(spans);
    let p = Paragraph::new(line);
    f.render_widget(p, Rect::new(x, y, total_width, 1));
}

fn render_bottom_bar(f: &mut Frame, area: Rect, state: &crate::creator::CreatorState) {
    let hints = if state.editing {
        vec![
            ("←/→", "adjust"),
            ("Shift+←/→", "×10"),
            ("↑/↓", "slider"),
            ("Tab", "hex/slider"),
            ("Esc", "done"),
        ]
    } else {
        vec![
            ("j/k", "nav"),
            ("Enter", "edit"),
            ("g", "generate"),
            ("p", "osc preview"),
            ("s", "save"),
            ("Esc", "quit"),
        ]
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(*key, Style::default().fg(ACCENT)));
        spans.push(Span::styled(format!(":{}", action), Style::default().fg(DIM)));
    }
    let line = Line::from(spans);
    let p = Paragraph::new(line);
    f.render_widget(p, area);
}

/// Returns the layout rects for mouse hit testing.
/// Call this with the same area as render_creator to get clickable regions.
pub fn get_layout_rects(area: Rect) -> CreatorLayout {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(outer[1]);

    let fields_block = Block::default().borders(Borders::ALL);
    let picker_block = Block::default().borders(Borders::ALL);

    CreatorLayout {
        fields_inner: fields_block.inner(cols[0]),
        picker_inner: picker_block.inner(cols[1]),
    }
}

pub struct CreatorLayout {
    pub fields_inner: Rect,
    pub picker_inner: Rect,
}
```

**Step 2: Update `src/ui/mod.rs`**

Add the new modules and exports:

```rust
mod browser;
mod collections;
mod create_meta;
mod creator;
mod details;
mod preview;

pub use browser::render_browser;
pub use collections::render_collections;
pub use create_meta::render_create_meta;
pub use creator::render_creator;
pub use details::render_detail;
```

**Step 3: Create a stub `src/ui/create_meta.rs`**

```rust
use ratatui::Frame;
use crate::app::App;

/// Render the metadata entry screen (stub — Task 9 fills this in)
pub fn render_create_meta(f: &mut Frame, app: &App) {
    let _ = (f, app);
}
```

**Step 4: Verify it compiles**

Run: `cargo build`

**Step 5: Commit**

```bash
git add src/ui/creator.rs src/ui/create_meta.rs src/ui/mod.rs
git commit -m "feat: add creator screen rendering with HSL sliders and preview"
```

---

### Task 7: Creator keyboard input handling

**Files:**
- Modify: `src/main.rs`

**Context:** Replace the placeholder `handle_create_input` with the full implementation. Handles: j/k field navigation, Enter to toggle edit mode, arrow keys for slider adjustment, Shift+arrow for ×10, Tab to toggle slider/hex, g to toggle/regenerate palette, p for OSC preview toggle, s to go to metadata screen, Esc/q to exit (with confirmation if unsaved), up/down to change focused slider in edit mode.

**Step 1: Implement `handle_create_input`**

Replace the placeholder with:

```rust
fn handle_create_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    use crate::creator::{PickerMode, SliderFocus};

    let state = match app.creator_state.as_mut() {
        Some(s) => s,
        None => return,
    };

    if state.editing {
        match state.picker_mode {
            PickerMode::Slider => match key {
                KeyCode::Left => {
                    let delta = if modifiers.contains(KeyModifiers::SHIFT) { -10.0 } else { -1.0 };
                    state.adjust_slider(delta);
                    if state.osc_preview {
                        let config = state.build_preview_config();
                        preview::apply_osc_preview(&config);
                    }
                }
                KeyCode::Right => {
                    let delta = if modifiers.contains(KeyModifiers::SHIFT) { 10.0 } else { 1.0 };
                    state.adjust_slider(delta);
                    if state.osc_preview {
                        let config = state.build_preview_config();
                        preview::apply_osc_preview(&config);
                    }
                }
                KeyCode::Up => {
                    state.slider_focus = match state.slider_focus {
                        SliderFocus::Hue => SliderFocus::Lightness,
                        SliderFocus::Saturation => SliderFocus::Hue,
                        SliderFocus::Lightness => SliderFocus::Saturation,
                    };
                }
                KeyCode::Down => {
                    state.slider_focus = match state.slider_focus {
                        SliderFocus::Hue => SliderFocus::Saturation,
                        SliderFocus::Saturation => SliderFocus::Lightness,
                        SliderFocus::Lightness => SliderFocus::Hue,
                    };
                }
                KeyCode::Tab => {
                    state.picker_mode = PickerMode::HexInput;
                    state.sync_hex_from_color();
                }
                KeyCode::Esc | KeyCode::Enter => {
                    state.editing = false;
                }
                _ => {}
            },
            PickerMode::HexInput => match key {
                KeyCode::Char(c) if c.is_ascii_hexdigit() || c == '#' => {
                    if state.hex_input.len() < 7 {
                        state.hex_input.push(c);
                    }
                    // Auto-commit when we have a full hex value
                    let hex_digits = state.hex_input.trim_start_matches('#').len();
                    if hex_digits == 6 {
                        state.commit_hex_input();
                        if state.osc_preview {
                            let config = state.build_preview_config();
                            preview::apply_osc_preview(&config);
                        }
                    }
                }
                KeyCode::Backspace => {
                    state.hex_input.pop();
                }
                KeyCode::Enter => {
                    state.commit_hex_input();
                    state.editing = false;
                    if state.osc_preview {
                        let config = state.build_preview_config();
                        preview::apply_osc_preview(&config);
                    }
                }
                KeyCode::Tab => {
                    state.picker_mode = PickerMode::Slider;
                }
                KeyCode::Esc => {
                    state.editing = false;
                }
                _ => {}
            },
        }
    } else {
        // Navigation mode
        let field_count = crate::creator::ColorField::all().len();
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                state.field_index = (state.field_index + 1).min(field_count - 1);
                // Scroll field list
                // Assume ~18 visible rows; adjust scroll if needed
                if state.field_index >= state.field_scroll + 18 {
                    state.field_scroll = state.field_index.saturating_sub(17);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.field_index = state.field_index.saturating_sub(1);
                if state.field_index < state.field_scroll {
                    state.field_scroll = state.field_index;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                state.editing = true;
                state.slider_focus = SliderFocus::Hue;
                state.sync_hex_from_color();
            }
            KeyCode::Char('g') => {
                if state.palette_dirty {
                    // For now, just regenerate (in a more polished version, we'd confirm)
                    // Toggle algorithm and regenerate
                    state.gen_algorithm = state.gen_algorithm.toggle();
                    state.generate_palette();
                    state.palette_dirty = false;
                    app.status_message = Some(format!("Generated palette ({})", state.gen_algorithm.label()));
                } else {
                    state.gen_algorithm = state.gen_algorithm.toggle();
                    state.generate_palette();
                    app.status_message = Some(format!("Generated palette ({})", state.gen_algorithm.label()));
                }
                if state.osc_preview {
                    let config = state.build_preview_config();
                    preview::apply_osc_preview(&config);
                }
            }
            KeyCode::Char('p') => {
                state.osc_preview = !state.osc_preview;
                if state.osc_preview {
                    app.saved_colors = Some(preview::save_current_colors());
                    let config = state.build_preview_config();
                    preview::apply_osc_preview(&config);
                    app.status_message = Some("OSC preview on".into());
                } else {
                    if let Some(ref saved) = app.saved_colors {
                        preview::restore_colors(saved);
                    }
                    app.saved_colors = None;
                    app.status_message = Some("OSC preview off".into());
                }
            }
            KeyCode::Char('s') => {
                app.enter_create_meta();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // Restore OSC if active
                if state.osc_preview {
                    if let Some(ref saved) = app.saved_colors {
                        preview::restore_colors(saved);
                    }
                    app.saved_colors = None;
                }
                app.creator_state = None;
                app.screen = Screen::Browse;
            }
            _ => {}
        }
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement creator keyboard input handling"
```

---

### Task 8: Creator mouse input handling

**Files:**
- Modify: `src/main.rs`

**Context:** Replace the placeholder `handle_create_mouse`. Mouse clicks on the field list select that field. Mouse clicks/drags on HSL slider bars set the value proportionally. We use `get_layout_rects()` from `ui/creator.rs` to determine click regions.

**Step 1: Implement `handle_create_mouse`**

Replace the placeholder with:

```rust
fn handle_create_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    use crossterm::event::MouseEventKind;

    let state = match app.creator_state.as_mut() {
        Some(s) => s,
        None => return,
    };

    let layout = ui::creator::get_layout_rects(app_area(app));

    match mouse.kind {
        MouseEventKind::Down(_) | MouseEventKind::Drag(_) => {
            let col = mouse.column;
            let row = mouse.row;

            // Check if click is in field list area
            if col >= layout.fields_inner.x
                && col < layout.fields_inner.x + layout.fields_inner.width
                && row >= layout.fields_inner.y
                && row < layout.fields_inner.y + layout.fields_inner.height
            {
                let relative_row = (row - layout.fields_inner.y) as usize;
                let field_idx = relative_row + state.field_scroll;
                let field_count = crate::creator::ColorField::all().len();
                if field_idx < field_count {
                    state.field_index = field_idx;
                    state.editing = true;
                    state.sync_hex_from_color();
                }
                return;
            }

            // Check if click/drag is in picker area (for slider manipulation)
            if state.editing
                && col >= layout.picker_inner.x
                && col < layout.picker_inner.x + layout.picker_inner.width
                && row >= layout.picker_inner.y
                && row < layout.picker_inner.y + layout.picker_inner.height
            {
                let relative_row = row - layout.picker_inner.y;
                let bar_start_x = layout.picker_inner.x + 5; // "  H: " = 5 chars
                let bar_width = layout.picker_inner.width.saturating_sub(15) as f64; // room for label + value

                if col >= bar_start_x && bar_width > 0.0 {
                    let frac = ((col - bar_start_x) as f64 / bar_width).clamp(0.0, 1.0);

                    // Which slider row was clicked
                    match relative_row {
                        0 => {
                            state.slider_focus = crate::creator::SliderFocus::Hue;
                            let mut color = state.colors[state.field_index];
                            color.h = frac * 360.0;
                            state.colors[state.field_index] = color;
                            state.unsaved = true;
                        }
                        1 => {
                            state.slider_focus = crate::creator::SliderFocus::Saturation;
                            let mut color = state.colors[state.field_index];
                            color.s = frac * 100.0;
                            state.colors[state.field_index] = color;
                            state.unsaved = true;
                        }
                        2 => {
                            state.slider_focus = crate::creator::SliderFocus::Lightness;
                            let mut color = state.colors[state.field_index];
                            color.l = frac * 100.0;
                            state.colors[state.field_index] = color;
                            state.unsaved = true;
                        }
                        _ => {}
                    }

                    if state.osc_preview {
                        let config = state.build_preview_config();
                        preview::apply_osc_preview(&config);
                    }
                }
            }
        }
        _ => {}
    }
}
```

**Step 2: Add `app_area` helper**

We need a way to get the terminal size for layout calculations. Add this helper:

```rust
fn app_area(app: &App) -> Rect {
    // Get terminal size from crossterm
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    Rect::new(0, 0, w, h)
}
```

Note: Need to add `use ratatui::layout::Rect;` to the imports at the top of main.rs.

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement creator mouse input for field selection and slider dragging"
```

---

### Task 9: Metadata screen (ui/create_meta.rs) and input handling

**Files:**
- Modify: `src/ui/create_meta.rs` (replace stub)
- Modify: `src/app.rs`
- Modify: `src/main.rs`

**Context:** The metadata screen collects title, description, tags, and author name before save/export/upload. Navigation with j/k, Enter to edit text fields, Space to toggle tags. After metadata, `a` to apply, `e` to export, `u` to upload.

**Step 1: Add metadata state to App**

In `src/app.rs`, add a `CreateMetaState` struct and field:

```rust
pub struct CreateMetaState {
    pub description: String,
    pub tags: Vec<String>,
    pub author_name: String,
    pub field_index: usize, // 0=title, 1=description, 2=tags, 3=author, 4=actions
    pub editing: bool,
    pub tag_cursor: usize,
}
```

Add to `App` struct:

```rust
    pub create_meta_state: Option<CreateMetaState>,
```

Initialize as `None` in `App::new()`.

Update `enter_create_meta` to initialize it:

```rust
    pub fn enter_create_meta(&mut self) {
        let title = self.creator_state.as_ref().map(|s| s.title.clone()).unwrap_or_default();
        self.create_meta_state = Some(CreateMetaState {
            description: String::new(),
            tags: Vec::new(),
            author_name: String::new(),
            field_index: 0,
            editing: false,
            tag_cursor: 0,
        });
        self.screen = Screen::CreateMeta;
    }
```

**Step 2: Replace `src/ui/create_meta.rs` with full implementation**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, AVAILABLE_TAGS};

const ACCENT: Color = Color::Rgb(187, 154, 247);
const DIM: Color = Color::Rgb(100, 100, 120);

pub fn render_create_meta(f: &mut Frame, app: &App) {
    let (creator, meta) = match (&app.creator_state, &app.create_meta_state) {
        (Some(c), Some(m)) => (c, m),
        _ => return,
    };

    let area = f.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    // Top bar
    let title_line = Line::from(vec![
        Span::styled(" Save Theme: ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(&creator.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]);
    let top_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM));
    f.render_widget(Paragraph::new(title_line).block(top_block), outer[0]);

    // Main content: form fields
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer[1]);

    let form_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" Metadata ", Style::default().fg(ACCENT)));
    let form_inner = form_block.inner(content[0]);
    f.render_widget(form_block, content[0]);

    let mut lines: Vec<Line> = Vec::new();

    // Field 0: Title
    let sel = meta.field_index == 0;
    let marker = if sel { "> " } else { "  " };
    let editing_indicator = if sel && meta.editing { " (editing)" } else { "" };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Title: ", Style::default().fg(DIM)),
        Span::styled(&creator.title, Style::default().fg(Color::White)),
        Span::styled(editing_indicator, Style::default().fg(ACCENT)),
    ]));
    lines.push(Line::from(""));

    // Field 1: Description
    let sel = meta.field_index == 1;
    let marker = if sel { "> " } else { "  " };
    let desc_display = if meta.description.is_empty() { "(optional)" } else { &meta.description };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Description: ", Style::default().fg(DIM)),
        Span::styled(desc_display, Style::default().fg(if meta.description.is_empty() { DIM } else { Color::White })),
        if sel && meta.editing {
            Span::styled("_", Style::default().fg(ACCENT))
        } else {
            Span::styled("", Style::default())
        },
    ]));
    lines.push(Line::from(""));

    // Field 2: Tags
    let sel = meta.field_index == 2;
    let marker = if sel { "> " } else { "  " };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Tags: ", Style::default().fg(DIM)),
        Span::styled(
            if meta.tags.is_empty() { "(select up to 5)".to_string() } else { meta.tags.join(", ") },
            Style::default().fg(if meta.tags.is_empty() { DIM } else { Color::White }),
        ),
    ]));

    // Show tag selector when this field is active and editing
    if sel && meta.editing {
        let upload_tags = [
            "dark", "light", "minimal", "colorful", "retro",
            "pastel", "high-contrast", "monochrome", "warm", "cool", "neon",
        ];
        for (i, tag) in upload_tags.iter().enumerate() {
            let is_selected = meta.tags.contains(&tag.to_string());
            let is_cursor = i == meta.tag_cursor;
            let check = if is_selected { "[x]" } else { "[ ]" };
            let cursor_marker = if is_cursor { " > " } else { "   " };
            lines.push(Line::from(vec![
                Span::styled(format!("    {}{} ", cursor_marker, check),
                    Style::default().fg(if is_cursor { ACCENT } else { DIM })),
                Span::styled(*tag, Style::default().fg(if is_cursor { Color::White } else { DIM })),
            ]));
        }
    }
    lines.push(Line::from(""));

    // Field 3: Author name
    let sel = meta.field_index == 3;
    let marker = if sel { "> " } else { "  " };
    let author_display = if meta.author_name.is_empty() { "(optional)" } else { &meta.author_name };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Author: ", Style::default().fg(DIM)),
        Span::styled(author_display, Style::default().fg(if meta.author_name.is_empty() { DIM } else { Color::White })),
        if sel && meta.editing {
            Span::styled("_", Style::default().fg(ACCENT))
        } else {
            Span::styled("", Style::default())
        },
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Action buttons
    let sel = meta.field_index == 4;
    lines.push(Line::from(vec![
        Span::styled(if sel { "> " } else { "  " }, field_style(sel)),
        Span::styled(" a ", Style::default().fg(Color::Black).bg(ACCENT)),
        Span::styled(" Apply  ", Style::default().fg(DIM)),
        Span::styled(" e ", Style::default().fg(Color::Black).bg(Color::Green)),
        Span::styled(" Export  ", Style::default().fg(DIM)),
        Span::styled(" u ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::styled(" Upload ", Style::default().fg(DIM)),
    ]));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), form_inner);

    // Right panel: preview
    let preview_config = creator.build_preview_config();
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" Preview ", Style::default().fg(ACCENT)));
    let preview_inner = preview_block.inner(content[1]);
    f.render_widget(preview_block, content[1]);
    f.render_widget(crate::ui::preview::ThemePreview { theme: &preview_config }, preview_inner);

    // Bottom bar
    let hints = if meta.editing && meta.field_index == 2 {
        vec![("j/k", "nav tags"), ("Space", "toggle"), ("Esc", "done")]
    } else if meta.editing {
        vec![("type", "edit"), ("Esc", "done")]
    } else {
        vec![("j/k", "nav"), ("Enter", "edit"), ("a", "apply"), ("e", "export"), ("u", "upload"), ("Esc", "back")]
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(*key, Style::default().fg(ACCENT)));
        spans.push(Span::styled(format!(":{}", action), Style::default().fg(DIM)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), outer[2]);
}

fn field_style(selected: bool) -> Style {
    if selected {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    }
}
```

**Step 3: Implement `handle_create_meta_input` in `src/main.rs`**

Replace the placeholder:

```rust
fn handle_create_meta_input(app: &mut App, key: KeyCode) {
    let meta = match app.create_meta_state.as_mut() {
        Some(m) => m,
        None => return,
    };

    if meta.editing {
        match meta.field_index {
            0 => {
                // Editing title — edit on creator_state
                if let Some(ref mut creator) = app.creator_state {
                    match key {
                        KeyCode::Char(c) => creator.title.push(c),
                        KeyCode::Backspace => { creator.title.pop(); }
                        KeyCode::Enter | KeyCode::Esc => { meta.editing = false; }
                        _ => {}
                    }
                }
            }
            1 => {
                // Editing description
                match key {
                    KeyCode::Char(c) => meta.description.push(c),
                    KeyCode::Backspace => { meta.description.pop(); }
                    KeyCode::Enter | KeyCode::Esc => { meta.editing = false; }
                    _ => {}
                }
            }
            2 => {
                // Tag selection mode
                let tag_list = [
                    "dark", "light", "minimal", "colorful", "retro",
                    "pastel", "high-contrast", "monochrome", "warm", "cool", "neon",
                ];
                match key {
                    KeyCode::Char('j') | KeyCode::Down => {
                        meta.tag_cursor = (meta.tag_cursor + 1).min(tag_list.len() - 1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        meta.tag_cursor = meta.tag_cursor.saturating_sub(1);
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        let tag = tag_list[meta.tag_cursor].to_string();
                        if meta.tags.contains(&tag) {
                            meta.tags.retain(|t| t != &tag);
                        } else if meta.tags.len() < 5 {
                            meta.tags.push(tag);
                        }
                    }
                    KeyCode::Esc => { meta.editing = false; }
                    _ => {}
                }
            }
            3 => {
                // Editing author name
                match key {
                    KeyCode::Char(c) => meta.author_name.push(c),
                    KeyCode::Backspace => { meta.author_name.pop(); }
                    KeyCode::Enter | KeyCode::Esc => { meta.editing = false; }
                    _ => {}
                }
            }
            _ => {}
        }
    } else {
        // Navigation mode
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                meta.field_index = (meta.field_index + 1).min(4);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                meta.field_index = meta.field_index.saturating_sub(1);
            }
            KeyCode::Enter => {
                if meta.field_index <= 3 {
                    meta.editing = true;
                }
            }
            KeyCode::Char('a') => {
                // Apply to Ghostty config
                if let Some(ref state) = app.creator_state {
                    match export::apply_created_theme(state) {
                        Ok(path) => {
                            app.status_message = Some(format!("Applied to {}", path));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
            KeyCode::Char('e') => {
                // Export to file
                if let Some(ref state) = app.creator_state {
                    match export::export_theme(state) {
                        Ok(path) => {
                            app.status_message = Some(format!("Exported to {}", path));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
            KeyCode::Char('u') => {
                // Upload
                if let Some(ref state) = app.creator_state {
                    match export::upload_theme(state) {
                        Ok(msg) => {
                            app.status_message = Some(msg);
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
            KeyCode::Esc => {
                // Back to creator
                app.create_meta_state = None;
                app.screen = Screen::Create;
            }
            _ => {}
        }
    }
}
```

**Step 4: Verify it compiles**

Run: `cargo build`

**Step 5: Commit**

```bash
git add src/main.rs src/app.rs src/ui/create_meta.rs
git commit -m "feat: implement metadata screen with tag selection and export actions"
```

---

### Task 10: Browse/Detail keybinding hints update

**Files:**
- Modify: `src/ui/browser.rs`
- Modify: `src/ui/details.rs`

**Context:** Add `n:new` hint to the Browse screen bottom bar and `f:fork` hint to the Detail screen bottom bar. Also update pagination hints from `n/N` to `]/[` since we remapped those keys in Task 5.

**Step 1: Update Browse screen bottom bar in `src/ui/browser.rs`**

Find the keybinding hints section and:
- Replace `n/N:page` with `]/[:page`
- Add `n:new` hint

**Step 2: Update Detail screen bottom bar in `src/ui/details.rs`**

Add `f:fork` hint alongside existing hints.

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/ui/browser.rs src/ui/details.rs
git commit -m "feat: update keybinding hints for create/fork and pagination"
```

---

### Task 11: Integration polish and edge cases

**Files:**
- Modify: `src/main.rs`
- Modify: `src/creator.rs`
- Modify: `src/app.rs`

**Context:** Handle edge cases: empty title validation, OSC cleanup on exit from creator, field scroll bounds, ensure unsaved confirmation works, auto-derive cursor/selection when bg/fg change.

**Step 1: Auto-derive when bg/fg edited**

In `creator.rs`, add to `set_current_color`:

```rust
    pub fn set_current_color(&mut self, color: HslColor) {
        self.colors[self.field_index] = color;
        self.unsaved = true;
        if self.field_index >= 6 {
            self.palette_dirty = true;
        }
        // Auto-derive cursor/selection when bg or fg changes
        if self.field_index <= 1 {
            self.auto_derive();
        }
    }
```

Also update `adjust_slider` to call `auto_derive` similarly when `field_index <= 1`.

**Step 2: Title validation on save**

In `handle_create_meta_input`, when `a`, `e`, or `u` is pressed, check that title is not empty:

```rust
            KeyCode::Char('a') | KeyCode::Char('e') | KeyCode::Char('u') => {
                if let Some(ref state) = app.creator_state {
                    if state.title.trim().is_empty() {
                        app.status_message = Some("Title cannot be empty".into());
                        return;
                    }
                    // ... proceed with action
                }
            }
```

**Step 3: Cleanup on App::cleanup**

In `app.rs`, update `cleanup()` to also restore OSC if creator was using it:

```rust
    pub fn cleanup(&mut self) {
        if self.osc_preview_active {
            if let Some(ref saved) = self.saved_colors {
                preview::restore_colors(saved);
            }
        }
        // Also check creator OSC state
        if let Some(ref state) = self.creator_state {
            if state.osc_preview {
                if let Some(ref saved) = self.saved_colors {
                    preview::restore_colors(saved);
                }
            }
        }
    }
```

**Step 4: Verify it compiles and test manually**

Run: `cargo build`
Run: `cargo run` — verify `n` opens creator, can navigate fields, edit colors, see preview update, `s` goes to metadata, `a`/`e`/`u` work.
Run: `cargo run -- create` — verify CLI create works.
Run: `cargo run -- create --from catppuccin-mocha` — verify fork works (requires network).

**Step 5: Commit**

```bash
git add src/creator.rs src/main.rs src/app.rs
git commit -m "feat: polish creator with auto-derive, title validation, and cleanup"
```

---

### Task 12: Update CLAUDE.md and README.md

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

**Context:** Document the new modules (`creator.rs`, `export.rs`, `ui/creator.rs`, `ui/create_meta.rs`), new screens (`Create`, `CreateMeta`), CLI `create` command, and keybindings in both CLAUDE.md and README.md.

**Step 1: Update CLAUDE.md**

Add to the Core Modules section:
- `creator.rs` — CreatorState, HSL color math, palette auto-generation, raw config building
- `export.rs` — Theme export to .conf file, apply to Ghostty config, open browser for upload

Add to the UI Modules section:
- `ui/creator.rs` — Creator screen: three-column layout with color field list, HSL sliders, theme preview
- `ui/create_meta.rs` — Metadata entry screen: title, description, tags, author, export actions

Update Screen Flow:
```
Browse → Detail → Confirm (apply)
  ↓        ↓
  n      f (fork)
  ↓        ↓
Create → CreateMeta → Apply/Export/Upload
```

**Step 2: Update README.md**

Add a "Theme Creation" section with:
- Creating from scratch (`n` on Browse or `ghostty-styles create`)
- Forking existing themes (`f` on Detail or `ghostty-styles create --from <slug>`)
- Color picker keybindings
- HSL slider and hex input
- Palette auto-generation algorithms
- Export, apply, and upload workflow

Update the keybindings table to include new keys and the pagination remap.

**Step 3: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: update CLAUDE.md and README for theme creation feature"
```
