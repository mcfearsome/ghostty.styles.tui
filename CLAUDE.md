# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A TUI application for browsing and previewing Ghostty terminal themes. It fetches themes from the ghostty-style.vercel.app API and lets users search, filter, preview (via OSC escape sequences), and apply themes directly to their Ghostty config file.

Requires the Ghostty terminal to run (checks `TERM_PROGRAM` env var at startup).

## Build & Run

```bash
cargo build          # debug build
cargo run            # run the TUI (must be in Ghostty terminal)
cargo build --release  # release build
```

No tests or lints are currently configured.

## Architecture

**Rust + ratatui (0.29) + crossterm (0.28)** — standard immediate-mode TUI pattern with a main event loop.

### Core Modules

- **`main.rs`** — Terminal setup/teardown, event loop, and input handling dispatched by `Screen` and `InputMode`. All keybinding logic lives here in `handle_browse_input`, `handle_detail_input`, `handle_confirm_input`, `handle_create_input`, `handle_create_meta_input`, `handle_create_mouse`.
- **`app.rs`** — Central `App` state struct. Owns all UI state (selection, pagination, filters, search). Uses `mpsc` channels for background API fetches on a spawned thread. `BgMessage` enum for thread communication.
- **`api.rs`** — HTTP client using `reqwest::blocking`. Fetches from `https://ghostty-style.vercel.app/api/configs` with query/tag/sort/page/dark params. `SortOrder` enum cycles through Popular → Newest → Trending.
- **`theme.rs`** — `GhosttyConfig` and `ConfigResponse` serde models (camelCase deserialized). Helper methods for parsing hex colors to ratatui `Color`.
- **`config.rs`** — Reads/writes the Ghostty config file (`~/Library/Application Support/com.mitchellh.ghostty/config` on macOS, `~/.config/ghostty/config` on Linux). Strips existing color keys before appending theme's `raw_config`. Creates `.config.bak` backup.
- **`preview.rs`** — OSC escape sequences (OSC 10/11/12/4) for live terminal color preview. Restores via OSC 110/111/112/104.
- **`cli.rs`** — Clap CLI definitions. Subcommands: collection (create/list/show/add/use/delete), next, cycle (start/stop/status), mode (dark/light/auto-os/auto-time/off/status).
- **`collection.rs`** — Collection data model and persistence. CRUD for named theme collections stored in `~/.config/ghostty-styles/collections/`. `AppConfig` for active collection tracking, mode preference (`ModePreference` enum: Dark/Light/AutoOs/AutoTime), and time-of-day boundaries (`dark_after`/`light_after`).
- **`cycling.rs`** — Theme cycling logic. `apply_next()` advances to next theme (sequential or shuffle), filters by mode preference (dark/light), and writes to Ghostty config.
- **`daemon.rs`** — Cycling daemon. start/stop/status for timed theme rotation. Uses PID file for process management. Multi-source wake: OS dark mode watcher, interval timer, time-of-day boundary.
- **`darkmode.rs`** — OS dark mode detection (`defaults read` on macOS, gsettings/dconf on Linux), event-driven watcher (DistributedNotificationCenter on macOS, gsettings monitor on Linux, 30s polling fallback), time-of-day resolution via libc.
- **`shell_hook.rs`** — Shell hook installer. Detects shell (zsh/bash), installs snippet to rc file for new-tab cycling.
- **`creator.rs`** — `CreatorState` data model, `HslColor` with HSL↔RGB↔Hex conversion, `ColorField` enum (22 fields: bg, fg, cursor, selection, palette 0-15), palette auto-generation (hue rotation and base16 algorithms), raw config building.
- **`export.rs`** — Theme export to `~/.config/ghostty-styles/themes/<slug>.conf`, apply to Ghostty config via `config::apply_theme`, open browser for upload to ghostty-style.vercel.app.

### UI Modules (`src/ui/`)

- **`browser.rs`** — Browse screen: top bar (title, search, filters), theme list (45%) + preview panel (55%), bottom keybind hints, tag popup overlay.
- **`details.rs`** — Detail screen: theme info, raw config display, and confirmation prompt for applying.
- **`preview.rs`** — `ThemePreview` widget (implements ratatui `Widget`). Renders palette swatches and sample terminal output using theme colors.
- **`collections.rs`** — Collections management screen. Two-panel layout with collection list and theme detail.
- **`creator.rs`** — Creator screen: three-column layout with color field list (25%), HSL picker with gradient sliders (35%), and theme preview (40%). Supports mouse click/drag on sliders.
- **`create_meta.rs`** — Metadata entry screen: title, description, tags (multi-select up to 5), author name, and action buttons (apply/export/upload).

### Screen Flow

`Browse` → `Detail` → `Confirm` (apply theme). Navigation is vim-style (j/k/h/l) plus arrow keys.

```
Browse → Detail → Confirm (apply)
  ↓        ↓
  n      f (fork)
  ↓        ↓
Create → CreateMeta → Apply/Export/Upload
  ↓
Collections → Collection themes
```

### Key Patterns

- Background API fetches: `App::trigger_fetch()` spawns a thread, result received via `App::poll_background()` called each frame.
- Color constants `ACCENT` (purple) and `DIM` are defined locally in both `browser.rs` and `details.rs`.
- The `COLOR_KEYS` array in `config.rs` determines which config lines get replaced when applying a theme.
- `c` on Browse/Detail: add theme to collection.
- `C` on Browse: open Collections screen.
- `n` on Browse: open theme creator. `f` on Detail: fork theme into creator.
- Creator uses mouse capture for field selection and slider dragging.
- `]/[` for pagination (remapped from n/N).
- `m` on Browse: cycle mode preference (Dark → Light → AutoOs → AutoTime → Off). Persists to config and re-fetches with appropriate dark_filter.
- `d` on Browse: manual dark/light API filter toggle (independent of mode preference).
