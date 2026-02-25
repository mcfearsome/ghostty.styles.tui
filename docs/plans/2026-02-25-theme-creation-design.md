# Theme Creation — Design

## Overview

Add the ability to create new Ghostty themes from scratch or by forking an existing theme. Features a full-screen color editor with HSL sliders, hex input, mouse support, live preview, and palette auto-generation. Themes can be applied locally, exported as config files, or submitted to the ghostty-style website.

## Entry Points

- **From scratch:** `n` keybinding on Browse screen, or `ghostty-styles create` CLI command
- **Fork existing:** `f` keybinding on Detail screen, or `ghostty-styles create --from <slug>` CLI command
- Forking pre-fills all colors from the source theme

## Creator Screen Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  Create Theme: "My Theme"                            p:preview  │
├──────────────┬──────────────────────┬───────────────────────────┤
│ Color Fields │   HSL Slider         │  Theme Preview            │
│              │                      │                           │
│ > background │  H: ████████░░ 240°  │  ┌─────────────────────┐  │
│   foreground │  S: ██████░░░░  60%  │  │ My Theme            │  │
│   cursor     │  L: ████░░░░░░  12%  │  │                     │  │
│   cursor-txt │                      │  │ ██ ██ ██ ██ ██ ██ ██│  │
│   select-bg  │  Hex: #1e1e2e       │  │ ██ ██ ██ ██ ██ ██ ██│  │
│   select-fg  │                      │  │                     │  │
│   palette 0  │  ┌────────────────┐  │  │ $ cargo build       │  │
│   palette 1  │  │  Color Swatch  │  │  │   Compiling...      │  │
│   ...        │  │                │  │  │ $ git status        │  │
│   palette 15 │  └────────────────┘  │  │  M src/main.rs      │  │
│              │                      │  │                     │  │
│  [gen: hue]  │  Tab: hex/slider     │  │  BG ██  FG ██       │  │
├──────────────┴──────────────────────┴───────────────────────────┤
│ j/k:nav  Enter:edit  g:generate  Tab:hex/slider  p:osc  s:save │
└─────────────────────────────────────────────────────────────────┘
```

Three-column layout: Fields (25%) | Picker (35%) | Preview (40%)

### Keybindings

| Key | Action |
|-----|--------|
| j/k | Navigate color fields |
| Enter/l | Enter edit mode for selected field |
| Tab | Toggle between HSL slider and hex input |
| Arrow keys | Adjust HSL values (left/right on active slider) |
| Shift+Arrow | Adjust HSL values by 10 |
| g | Cycle palette generation algorithm (hue rotation / base16) |
| p | Toggle OSC live terminal preview (off by default) |
| s | Save — proceed to metadata/export screen |
| Esc/q | Exit creator (confirm if unsaved changes) |

### Mouse Support

- Click a color field row to select it
- Click/drag on HSL slider bars to adjust values
- Click the hex input field to focus it
- Mouse events only processed on `Screen::Create` — other screens unchanged

## Color Picker Mechanics

### HSL Slider

Three horizontal bars rendered with colored block characters:

- **Hue (H):** 0°–360° — rainbow gradient with marker at current value
- **Saturation (S):** 0%–100% — gradient from gray to full color
- **Lightness (L):** 0%–100% — gradient from black through color to white

Arrow left/right adjusts by 1 unit, Shift+arrow adjusts by 10. A color swatch box below shows the composed color.

### Hex Input

Text field showing `#rrggbb`. Type hex chars directly (auto-prefixed with `#`), backspace to delete. Changes reflect bidirectionally with HSL sliders.

### Bidirectional HSL ↔ Hex

All color state stored as HSL internally:
- HSL → Hex: standard conversion, updates hex display on every slider change
- Hex → HSL: on hex input commit (Enter or 6 valid chars), converts and updates sliders

## Color Fields

Required: `background`, `foreground`, 16-color `palette` (ANSI 0-15)

Auto-derived (overridable): `cursor-color` (from fg), `cursor-text` (from bg), `selection-background` (from fg with reduced opacity), `selection-foreground` (from fg)

Total: 22 color fields

## Palette Auto-Generation

Triggered when entering first palette field with no values set, or anytime via `g` key. Two algorithms, toggled with `g`:

### Hue Rotation

1. Foreground hue as anchor
2. 6 accent hues at 60° intervals
3. Normal colors (0-7): color 0 = darkened bg, color 7 = dimmed fg, colors 1-6 = accents at moderate saturation/lightness
4. Bright colors (8-15): color 8 = lighter color 0, color 15 = fg, colors 9-14 = same hues boosted lightness
5. Scaling relative to bg lightness (dark vs light theme treatment)

### Base16-style

1. Colors 0, 7, 8, 15 form a grayscale ramp between bg and fg
2. Colors 1-6: canonical terminal hues (red=0°, green=120°, yellow=60°, blue=240°, magenta=300°, cyan=180°)
3. Colors 9-14: same hues, boosted lightness
4. Saturation/lightness derived from bg/fg contrast ratio

### is_dark Detection

Automatic from background lightness: L < 50% → dark. Influences grayscale ramp direction and saturation/lightness scaling.

Re-generating with `g` overwrites manual tweaks (confirmation prompt if any edits were made).

## Save, Export & Submit

### Metadata Screen (Screen::CreateMeta)

Pressing `s` transitions to metadata form:

1. **Title** — pre-filled from creation start (required)
2. **Description** — text input (optional)
3. **Tags** — multi-select from 11 predefined: dark, light, minimal, colorful, retro, pastel, high-contrast, monochrome, warm, cool, neon. Max 5. Toggle with Space.
4. **Author name** — text input (optional)

Navigate with j/k, Enter to edit, Space to toggle tags.

### Actions (not mutually exclusive)

| Key | Action |
|-----|--------|
| a | **Apply** — write to Ghostty config |
| e | **Export** — save to `~/.config/ghostty-styles/themes/<slug>.conf` |
| u | **Upload** — export + open `https://ghostty-style.vercel.app/upload` in browser |

### Export Format

Standard Ghostty config snippet (same as `raw_config`):
```
background = #1e1e2e
foreground = #cdd6f4
cursor-color = #f5e0dc
palette = 0=#45475a
palette = 1=#f38ba8
...
```

### Upload Flow

1. Export `.conf` file to themes directory
2. Open upload page via `open` (macOS) / `xdg-open` (Linux)
3. Status message: "Config saved to <path>. Upload page opened — drag the file to submit."

### CLI Flags

`ghostty-styles create [--from <slug>] [--apply] [--export] [--upload]`

## OSC Live Preview

Off by default. Toggle with `p`. When enabled, fires OSC 10/11/12/4 escape sequences to update the actual terminal colors as you edit — the entire terminal becomes your preview. Restores original colors on toggle-off or exit.

## New Modules

- **`src/creator.rs`** — `CreatorState` struct, HSL↔Hex conversion, palette auto-generation, `build_raw_config()`, `build_ghostty_config()`
- **`src/ui/creator.rs`** — Creator screen rendering: three-column layout, HSL sliders, hex input, mouse hit-testing
- **`src/ui/create_meta.rs`** — Metadata form screen rendering
- **`src/export.rs`** — Export to `.conf` file, open browser, apply theme

## Modified Modules

- **`src/main.rs`** — Enable `MouseCapture`, add `Event::Mouse` handling (Create screen only), `handle_create_input()`, `handle_create_meta_input()`, `n` on Browse, `f` on Detail, CLI dispatch for `create`
- **`src/app.rs`** — Add `Screen::Create`, `Screen::CreateMeta`, `CreatorState` field, entry methods
- **`src/cli.rs`** — Add `Create` command with `--from`, `--apply`, `--export`, `--upload` flags
- **`src/ui/browser.rs`** — Add `n` keybinding hint
- **`src/ui/details.rs`** — Add `f` keybinding hint

## New Dependencies

None — HSL conversion is pure math, browser opening uses `std::process::Command`.
