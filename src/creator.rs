use crate::theme::GhosttyConfig;

// ---------------------------------------------------------------------------
// HslColor
// ---------------------------------------------------------------------------

/// A color represented in the HSL (Hue, Saturation, Lightness) color space.
///
/// - `h`: hue in degrees, 0.0..360.0
/// - `s`: saturation as a percentage, 0.0..100.0
/// - `l`: lightness as a percentage, 0.0..100.0
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HslColor {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

impl HslColor {
    /// Create a new `HslColor`. Values are clamped to their valid ranges.
    pub fn new(h: f64, s: f64, l: f64) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            s: s.clamp(0.0, 100.0),
            l: l.clamp(0.0, 100.0),
        }
    }

    /// Convert this HSL color to an (r, g, b) tuple with each channel in 0..255.
    pub fn to_rgb(self) -> (u8, u8, u8) {
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

    /// Convert this HSL color to a hex string like `#rrggbb`.
    pub fn to_hex(self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    /// Create an `HslColor` from RGB channel values (each 0..255).
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let r_f = r as f64 / 255.0;
        let g_f = g as f64 / 255.0;
        let b_f = b as f64 / 255.0;

        let max = r_f.max(g_f).max(b_f);
        let min = r_f.min(g_f).min(b_f);
        let delta = max - min;

        let l = (max + min) / 2.0;

        if delta == 0.0 {
            return Self::new(0.0, 0.0, l * 100.0);
        }

        let s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        let h = if (max - r_f).abs() < f64::EPSILON {
            let mut h = (g_f - b_f) / delta;
            if h < 0.0 {
                h += 6.0;
            }
            h
        } else if (max - g_f).abs() < f64::EPSILON {
            (b_f - r_f) / delta + 2.0
        } else {
            (r_f - g_f) / delta + 4.0
        };

        Self::new(h * 60.0, s * 100.0, l * 100.0)
    }

    /// Parse a hex color string (e.g. `#ff00aa` or `ff00aa`) into an `HslColor`.
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

    /// Convert this HSL color to a `ratatui::style::Color`.
    pub fn to_ratatui_color(self) -> ratatui::style::Color {
        let (r, g, b) = self.to_rgb();
        ratatui::style::Color::Rgb(r, g, b)
    }
}

/// Helper for HSL-to-RGB conversion.
fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
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

// ---------------------------------------------------------------------------
// ColorField
// ---------------------------------------------------------------------------

/// Identifies which color slot is being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorField {
    Background,
    Foreground,
    CursorColor,
    CursorText,
    SelectionBg,
    SelectionFg,
    Palette(usize),
}

impl ColorField {
    /// Returns all 22 color fields in canonical order:
    /// bg, fg, cursor-color, cursor-text, selection-bg, selection-fg, palette 0-15.
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

    /// Human-readable label for this color field.
    pub fn label(&self) -> String {
        match self {
            ColorField::Background => "Background".to_string(),
            ColorField::Foreground => "Foreground".to_string(),
            ColorField::CursorColor => "Cursor Color".to_string(),
            ColorField::CursorText => "Cursor Text".to_string(),
            ColorField::SelectionBg => "Selection BG".to_string(),
            ColorField::SelectionFg => "Selection FG".to_string(),
            ColorField::Palette(i) => {
                let name = match i {
                    0 => "Black",
                    1 => "Red",
                    2 => "Green",
                    3 => "Yellow",
                    4 => "Blue",
                    5 => "Magenta",
                    6 => "Cyan",
                    7 => "White",
                    8 => "Bright Black",
                    9 => "Bright Red",
                    10 => "Bright Green",
                    11 => "Bright Yellow",
                    12 => "Bright Blue",
                    13 => "Bright Magenta",
                    14 => "Bright Cyan",
                    15 => "Bright White",
                    _ => "Unknown",
                };
                format!("Palette {:>2} ({})", i, name)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SliderFocus / PickerMode / GenAlgorithm
// ---------------------------------------------------------------------------

/// Which HSL slider component is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderFocus {
    Hue,
    Saturation,
    Lightness,
}

/// The current editing mode for the color picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    Slider,
    HexInput,
}

/// Algorithm used to auto-generate the 16-color palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenAlgorithm {
    HueRotation,
    Base16,
}

impl GenAlgorithm {
    /// Toggle to the other algorithm.
    pub fn toggle(self) -> Self {
        match self {
            GenAlgorithm::HueRotation => GenAlgorithm::Base16,
            GenAlgorithm::Base16 => GenAlgorithm::HueRotation,
        }
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            GenAlgorithm::HueRotation => "Hue Rotation",
            GenAlgorithm::Base16 => "Base16",
        }
    }
}

// ---------------------------------------------------------------------------
// CreatorState
// ---------------------------------------------------------------------------

/// Full state for the theme creator screen.
pub struct CreatorState {
    /// The user-chosen title for the theme.
    pub title: String,
    /// 22 HSL colors: [bg, fg, cursor-color, cursor-text, sel-bg, sel-fg, palette 0..15].
    pub colors: Vec<HslColor>,
    /// Index into `ColorField::all()` indicating which field is selected.
    pub field_index: usize,
    /// Whether the color picker is actively editing the current field.
    pub editing: bool,
    /// Current picker interaction mode (slider vs hex input).
    pub picker_mode: PickerMode,
    /// Which HSL slider component has focus.
    pub slider_focus: SliderFocus,
    /// Buffer for hex color text input.
    pub hex_input: String,
    /// The algorithm used for auto-generating the palette.
    pub gen_algorithm: GenAlgorithm,
    /// Whether OSC live preview is active.
    pub osc_preview: bool,
    /// Whether the palette needs regeneration.
    pub palette_dirty: bool,
    /// Whether there are unsaved changes.
    pub unsaved: bool,
    /// If forked from an existing theme, its slug or title.
    pub forked_from: Option<String>,
    /// Scroll offset for the field list (for when list exceeds visible area).
    pub field_scroll: usize,
}

impl CreatorState {
    /// Create a new blank creator state with sensible dark-theme defaults.
    pub fn new(title: impl Into<String>) -> Self {
        let bg = HslColor::new(220.0, 15.0, 13.0); // dark blue-gray
        let fg = HslColor::new(220.0, 10.0, 85.0); // light gray

        let cursor_color = fg;
        let cursor_text = bg;
        let selection_bg = HslColor::new(bg.h, bg.s.min(30.0), (bg.l + 15.0).min(100.0));
        let selection_fg = fg;

        let mut state = Self {
            title: title.into(),
            colors: vec![
                bg,
                fg,
                cursor_color,
                cursor_text,
                selection_bg,
                selection_fg,
                // Palette slots 0..15 — filled by generate_palette below
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
                HslColor::new(0.0, 0.0, 0.0),
            ],
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

        state.generate_palette();
        state.sync_hex_from_color();
        // A freshly created state has no unsaved changes yet.
        state.unsaved = false;
        state.palette_dirty = false;
        state
    }

    /// Fork a `CreatorState` from an existing `GhosttyConfig`, parsing all hex
    /// colors into HSL.
    pub fn from_theme(config: &GhosttyConfig) -> Self {
        let parse = |hex: &str| HslColor::from_hex(hex).unwrap_or(HslColor::new(0.0, 0.0, 0.0));

        let bg = parse(&config.background);
        let fg = parse(&config.foreground);
        let cursor_color = config.cursor_color.as_deref().map(parse).unwrap_or(fg);
        let cursor_text = config.cursor_text.as_deref().map(parse).unwrap_or(bg);
        let selection_bg = config
            .selection_bg
            .as_deref()
            .map(parse)
            .unwrap_or_else(|| HslColor::new(bg.h, bg.s.min(30.0), (bg.l + 15.0).min(100.0)));
        let selection_fg = config.selection_fg.as_deref().map(parse).unwrap_or(fg);

        let mut colors = vec![
            bg,
            fg,
            cursor_color,
            cursor_text,
            selection_bg,
            selection_fg,
        ];

        // Parse palette colors 0..15
        for i in 0..16 {
            let c = config
                .palette
                .get(i)
                .map(|s| parse(s))
                .unwrap_or(HslColor::new(0.0, 0.0, 0.0));
            colors.push(c);
        }

        let mut state = Self {
            title: config.title.clone(),
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
            forked_from: Some(config.slug.clone()),
            field_scroll: 0,
        };

        state.sync_hex_from_color();
        state.unsaved = false;
        state
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Return the `ColorField` for the current `field_index`.
    #[allow(dead_code)]
    pub fn current_field(&self) -> ColorField {
        let all = ColorField::all();
        all[self.field_index.min(all.len() - 1)]
    }

    /// Return a reference to the HSL color for the currently selected field.
    pub fn current_color(&self) -> &HslColor {
        &self.colors[self.field_index.min(self.colors.len() - 1)]
    }

    /// Set the color for the currently selected field and mark the state dirty.
    pub fn set_current_color(&mut self, color: HslColor) {
        let idx = self.field_index.min(self.colors.len() - 1);
        self.colors[idx] = color;
        self.unsaved = true;
        if idx >= 6 {
            self.palette_dirty = true;
        }
        // Auto-derive cursor/selection when bg or fg changes
        if idx <= 1 {
            self.auto_derive();
        }
    }

    // -----------------------------------------------------------------------
    // Slider / hex editing
    // -----------------------------------------------------------------------

    /// Adjust the currently focused HSL slider component by `delta`.
    pub fn adjust_slider(&mut self, delta: f64) {
        let mut color = *self.current_color();
        match self.slider_focus {
            SliderFocus::Hue => color.h = (color.h + delta).rem_euclid(360.0),
            SliderFocus::Saturation => color.s = (color.s + delta).clamp(0.0, 100.0),
            SliderFocus::Lightness => color.l = (color.l + delta).clamp(0.0, 100.0),
        }
        self.set_current_color(color);
        self.sync_hex_from_color();
    }

    /// Parse the current `hex_input` and, if valid, apply it to the current color.
    pub fn commit_hex_input(&mut self) {
        if let Some(c) = HslColor::from_hex(&self.hex_input) {
            self.set_current_color(c);
        }
    }

    /// Update `hex_input` to match the current color's hex representation.
    pub fn sync_hex_from_color(&mut self) {
        self.hex_input = self.current_color().to_hex();
    }

    // -----------------------------------------------------------------------
    // Auto-derive & palette generation
    // -----------------------------------------------------------------------

    /// Derive cursor and selection colors from the current background and
    /// foreground.
    pub fn auto_derive(&mut self) {
        let bg = self.colors[0];
        let fg = self.colors[1];

        // cursor-color = foreground
        self.colors[2] = fg;
        // cursor-text = background
        self.colors[3] = bg;
        // selection-bg: slightly lighter than bg, reduced saturation
        self.colors[4] = HslColor::new(bg.h, bg.s.min(30.0), (bg.l + 15.0).min(100.0));
        // selection-fg = foreground
        self.colors[5] = fg;

        self.unsaved = true;
    }

    /// Generate the 16-color ANSI palette using the current algorithm.
    pub fn generate_palette(&mut self) {
        match self.gen_algorithm {
            GenAlgorithm::HueRotation => self.gen_hue_rotation(),
            GenAlgorithm::Base16 => self.gen_base16(),
        }
        self.palette_dirty = false;
        self.unsaved = true;
    }

    /// Hue-rotation algorithm: produces 6 accent hues spaced 60 degrees apart
    /// starting from the foreground hue, with normal and bright variants.
    ///
    /// Palette mapping:
    /// - 0: black (dark), 7: white (light), 8: bright black, 15: bright white
    /// - 1-6: normal accent colors (red, green, yellow, blue, magenta, cyan)
    /// - 9-14: bright accent colors
    fn gen_hue_rotation(&mut self) {
        let bg = self.colors[0];
        let fg = self.colors[1];

        // Palette index 0: black — darkened bg
        self.colors[6] = HslColor::new(bg.h, bg.s, (bg.l * 0.5).max(3.0));
        // Palette index 7: white — slightly dimmed fg
        self.colors[13] = HslColor::new(fg.h, fg.s.min(15.0), fg.l.min(80.0));
        // Palette index 8: bright black — lighter than palette 0
        self.colors[14] = HslColor::new(bg.h, bg.s, (bg.l + 20.0).min(50.0));
        // Palette index 15: bright white — brighter fg
        self.colors[21] = HslColor::new(fg.h, fg.s.min(10.0), fg.l.clamp(90.0, 100.0));

        // 6 accent hues at 60-degree intervals from the foreground hue.
        // The canonical ANSI order is: red, green, yellow, blue, magenta, cyan.
        // We map hue offsets to approximate that ordering.
        let base_hue = fg.h;
        let accent_hues: [f64; 6] = [
            0.0,   // red-ish
            120.0, // green-ish
            60.0,  // yellow-ish
            240.0, // blue-ish
            300.0, // magenta-ish
            180.0, // cyan-ish
        ];

        let normal_sat = 60.0;
        let normal_light = if self.is_dark() { 60.0 } else { 40.0 };
        let bright_sat = 70.0;
        let bright_light = if self.is_dark() { 72.0 } else { 50.0 };

        for (i, &offset) in accent_hues.iter().enumerate() {
            let hue = (base_hue + offset).rem_euclid(360.0);

            // Normal variant: palette slots 1-6 -> colors[7..13]
            self.colors[7 + i] = HslColor::new(hue, normal_sat, normal_light);
            // Bright variant: palette slots 9-14 -> colors[15..21]
            self.colors[15 + i] = HslColor::new(hue, bright_sat, bright_light);
        }
    }

    /// Base16-inspired algorithm: uses a grayscale ramp for 0/7/8/15 and
    /// canonical ANSI hues for the accent colors 1-6 and 9-14.
    fn gen_base16(&mut self) {
        let bg = self.colors[0];
        let fg = self.colors[1];

        // Grayscale ramp
        self.colors[6] = HslColor::new(bg.h, bg.s.min(5.0), (bg.l + 5.0).min(100.0)); // 0: black
        self.colors[13] = HslColor::new(fg.h, fg.s.min(5.0), (fg.l - 10.0).max(0.0)); // 7: white
        self.colors[14] = HslColor::new(bg.h, bg.s.min(5.0), (bg.l + 25.0).min(100.0)); // 8: bright black
        self.colors[21] = HslColor::new(fg.h, fg.s.min(5.0), fg.l.min(100.0)); // 15: bright white

        // Canonical ANSI hues for 1-6.
        let canonical_hues: [f64; 6] = [
            0.0,   // red
            120.0, // green
            60.0,  // yellow
            210.0, // blue
            300.0, // magenta
            180.0, // cyan
        ];

        let normal_sat = 55.0;
        let normal_light = if self.is_dark() { 55.0 } else { 40.0 };
        let bright_sat = 65.0;
        let bright_light = if self.is_dark() { 68.0 } else { 50.0 };

        for (i, &hue) in canonical_hues.iter().enumerate() {
            self.colors[7 + i] = HslColor::new(hue, normal_sat, normal_light);
            self.colors[15 + i] = HslColor::new(hue, bright_sat, bright_light);
        }
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Returns `true` if the theme background is dark (lightness < 50).
    pub fn is_dark(&self) -> bool {
        self.colors[0].l < 50.0
    }

    // -----------------------------------------------------------------------
    // Config output
    // -----------------------------------------------------------------------

    /// Build a `GhosttyConfig` suitable for passing to the `ThemePreview` widget.
    pub fn build_preview_config(&self) -> GhosttyConfig {
        let palette: Vec<String> = (0..16).map(|i| self.colors[6 + i].to_hex()).collect();

        GhosttyConfig {
            id: String::new(),
            slug: self.slug_from_title(),
            title: self.title.clone(),
            description: self
                .forked_from
                .as_ref()
                .map(|s| format!("Forked from {}", s)),
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

    /// Render the current colors as a Ghostty-compatible config string.
    pub fn build_raw_config(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("background = {}", self.colors[0].to_hex()));
        lines.push(format!("foreground = {}", self.colors[1].to_hex()));
        lines.push(format!("cursor-color = {}", self.colors[2].to_hex()));
        lines.push(format!("cursor-text = {}", self.colors[3].to_hex()));
        lines.push(format!(
            "selection-background = {}",
            self.colors[4].to_hex()
        ));
        lines.push(format!(
            "selection-foreground = {}",
            self.colors[5].to_hex()
        ));

        for i in 0..16 {
            lines.push(format!("palette = {}={}", i, self.colors[6 + i].to_hex()));
        }

        lines.join("\n")
    }

    /// Derive a URL-friendly slug from the title.
    pub fn slug_from_title(&self) -> String {
        self.title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsl_round_trip_rgb() {
        // Pure red
        let c = HslColor::new(0.0, 100.0, 50.0);
        let (r, g, b) = c.to_rgb();
        assert_eq!((r, g, b), (255, 0, 0));
        let back = HslColor::from_rgb(r, g, b);
        assert!((back.h - 0.0).abs() < 1.0);
        assert!((back.s - 100.0).abs() < 1.0);
        assert!((back.l - 50.0).abs() < 1.0);
    }

    #[test]
    fn hsl_round_trip_hex() {
        let c = HslColor::new(120.0, 100.0, 50.0);
        assert_eq!(c.to_hex(), "#00ff00");

        let parsed = HslColor::from_hex("#00ff00").unwrap();
        assert!((parsed.h - 120.0).abs() < 1.0);
        assert!((parsed.s - 100.0).abs() < 1.0);
        assert!((parsed.l - 50.0).abs() < 1.0);
    }

    #[test]
    fn hsl_white_and_black() {
        let white = HslColor::new(0.0, 0.0, 100.0);
        assert_eq!(white.to_rgb(), (255, 255, 255));

        let black = HslColor::new(0.0, 0.0, 0.0);
        assert_eq!(black.to_rgb(), (0, 0, 0));
    }

    #[test]
    fn hsl_gray() {
        let gray = HslColor::new(0.0, 0.0, 50.0);
        let (r, g, b) = gray.to_rgb();
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert_eq!(r, 128);
    }

    #[test]
    fn from_hex_invalid() {
        assert!(HslColor::from_hex("xyz").is_none());
        assert!(HslColor::from_hex("#ff").is_none());
        assert!(HslColor::from_hex("").is_none());
    }

    #[test]
    fn color_field_all_count() {
        assert_eq!(ColorField::all().len(), 22);
    }

    #[test]
    fn color_field_labels() {
        assert_eq!(ColorField::Background.label(), "Background");
        assert_eq!(ColorField::Palette(1).label(), "Palette  1 (Red)");
        assert_eq!(ColorField::Palette(14).label(), "Palette 14 (Bright Cyan)");
    }

    #[test]
    fn gen_algorithm_toggle() {
        assert_eq!(GenAlgorithm::HueRotation.toggle(), GenAlgorithm::Base16);
        assert_eq!(GenAlgorithm::Base16.toggle(), GenAlgorithm::HueRotation);
    }

    #[test]
    fn gen_algorithm_labels() {
        assert_eq!(GenAlgorithm::HueRotation.label(), "Hue Rotation");
        assert_eq!(GenAlgorithm::Base16.label(), "Base16");
    }

    #[test]
    fn creator_state_new_has_22_colors() {
        let state = CreatorState::new("Test Theme");
        assert_eq!(state.colors.len(), 22);
        assert!(!state.unsaved);
        assert!(!state.palette_dirty);
        assert_eq!(state.title, "Test Theme");
    }

    #[test]
    fn creator_state_new_is_dark() {
        let state = CreatorState::new("Dark Theme");
        assert!(state.is_dark());
    }

    #[test]
    fn slug_from_title() {
        let state = CreatorState::new("My Cool Theme!");
        assert_eq!(state.slug_from_title(), "my-cool-theme");
    }

    #[test]
    fn build_raw_config_format() {
        let state = CreatorState::new("Test");
        let raw = state.build_raw_config();
        assert!(raw.contains("background = #"));
        assert!(raw.contains("foreground = #"));
        assert!(raw.contains("cursor-color = #"));
        assert!(raw.contains("cursor-text = #"));
        assert!(raw.contains("selection-background = #"));
        assert!(raw.contains("selection-foreground = #"));
        for i in 0..16 {
            assert!(raw.contains(&format!("palette = {}=#", i)));
        }
    }

    #[test]
    fn build_preview_config_valid() {
        let state = CreatorState::new("Preview Test");
        let config = state.build_preview_config();
        assert_eq!(config.title, "Preview Test");
        assert_eq!(config.palette.len(), 16);
        assert!(config.is_dark);
        assert!(config.cursor_color.is_some());
        assert!(config.cursor_text.is_some());
        assert!(config.selection_bg.is_some());
        assert!(config.selection_fg.is_some());
    }

    #[test]
    fn adjust_slider_hue_wraps() {
        let mut state = CreatorState::new("Test");
        // Set a known color on the current field
        state.set_current_color(HslColor::new(350.0, 50.0, 50.0));
        state.slider_focus = SliderFocus::Hue;
        state.adjust_slider(20.0);
        assert!((state.current_color().h - 10.0).abs() < 0.01);
    }

    #[test]
    fn adjust_slider_saturation_clamps() {
        let mut state = CreatorState::new("Test");
        state.set_current_color(HslColor::new(180.0, 95.0, 50.0));
        state.slider_focus = SliderFocus::Saturation;
        state.adjust_slider(10.0);
        assert!((state.current_color().s - 100.0).abs() < 0.01);
    }

    #[test]
    fn commit_hex_input_valid() {
        let mut state = CreatorState::new("Test");
        state.hex_input = "#ff0000".to_string();
        state.commit_hex_input();
        let (r, g, b) = state.current_color().to_rgb();
        assert_eq!((r, g, b), (255, 0, 0));
    }

    #[test]
    fn commit_hex_input_invalid_no_change() {
        let mut state = CreatorState::new("Test");
        let before = *state.current_color();
        state.hex_input = "nope".to_string();
        state.commit_hex_input();
        assert_eq!(*state.current_color(), before);
    }

    #[test]
    fn auto_derive_sets_cursor_and_selection() {
        let mut state = CreatorState::new("Test");
        let fg = state.colors[1];
        let bg = state.colors[0];
        state.auto_derive();
        assert_eq!(state.colors[2], fg); // cursor-color = fg
        assert_eq!(state.colors[3], bg); // cursor-text = bg
        assert_eq!(state.colors[5], fg); // selection-fg = fg
    }

    #[test]
    fn to_ratatui_color() {
        let c = HslColor::new(0.0, 100.0, 50.0);
        match c.to_ratatui_color() {
            ratatui::style::Color::Rgb(r, g, b) => {
                assert_eq!((r, g, b), (255, 0, 0));
            }
            _ => panic!("Expected Rgb color"),
        }
    }

    #[test]
    fn hsl_clamping() {
        let c = HslColor::new(400.0, 150.0, -10.0);
        assert!((c.h - 40.0).abs() < 0.01);
        assert!((c.s - 100.0).abs() < 0.01);
        assert!((c.l - 0.0).abs() < 0.01);
    }
}
