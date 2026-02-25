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

- **`main.rs`** — Terminal setup/teardown, event loop, and input handling dispatched by `Screen` and `InputMode`. All keybinding logic lives here in `handle_browse_input`, `handle_detail_input`, `handle_confirm_input`.
- **`app.rs`** — Central `App` state struct. Owns all UI state (selection, pagination, filters, search). Uses `mpsc` channels for background API fetches on a spawned thread. `BgMessage` enum for thread communication.
- **`api.rs`** — HTTP client using `reqwest::blocking`. Fetches from `https://ghostty-style.vercel.app/api/configs` with query/tag/sort/page/dark params. `SortOrder` enum cycles through Popular → Newest → Trending.
- **`theme.rs`** — `GhosttyConfig` and `ConfigResponse` serde models (camelCase deserialized). Helper methods for parsing hex colors to ratatui `Color`.
- **`config.rs`** — Reads/writes the Ghostty config file (`~/Library/Application Support/com.mitchellh.ghostty/config` on macOS, `~/.config/ghostty/config` on Linux). Strips existing color keys before appending theme's `raw_config`. Creates `.config.bak` backup.
- **`preview.rs`** — OSC escape sequences (OSC 10/11/12/4) for live terminal color preview. Restores via OSC 110/111/112/104.

### UI Modules (`src/ui/`)

- **`browser.rs`** — Browse screen: top bar (title, search, filters), theme list (45%) + preview panel (55%), bottom keybind hints, tag popup overlay.
- **`details.rs`** — Detail screen: theme info, raw config display, and confirmation prompt for applying.
- **`preview.rs`** — `ThemePreview` widget (implements ratatui `Widget`). Renders palette swatches and sample terminal output using theme colors.

### Screen Flow

`Browse` → `Detail` → `Confirm` (apply theme). Navigation is vim-style (j/k/h/l) plus arrow keys.

### Key Patterns

- Background API fetches: `App::trigger_fetch()` spawns a thread, result received via `App::poll_background()` called each frame.
- Color constants `ACCENT` (purple) and `DIM` are defined locally in both `browser.rs` and `details.rs`.
- The `COLOR_KEYS` array in `config.rs` determines which config lines get replaced when applying a theme.
