# ghostty-styles

A terminal UI for browsing, previewing, and applying [Ghostty](https://ghostty.org) themes.

Fetches themes from [ghostty-style.vercel.app](https://ghostty-style.vercel.app), displays live color previews using OSC escape sequences, and applies themes directly to your Ghostty config file.

Requires the Ghostty terminal to run.

## Features

- **Browse** hundreds of community themes with search, tag filtering, and sorting (Popular / Newest / Trending)
- **Live preview** — press `p` to apply theme colors to your terminal in real-time via OSC sequences (restored on exit)
- **Apply themes** directly to your Ghostty config with automatic backup
- **Filter** by dark/light mode, tags (retro, pastel, neon, minimal, etc.), and text search
- **Vim-style navigation** — `j`/`k`/`h`/`l`, arrow keys, or Enter to drill into details
- **Theme creation** — build themes from scratch or fork existing ones with an HSL color picker, mouse-draggable sliders, and palette auto-generation
- **Export & upload** — save themes locally, apply to your config, or export for upload to the community site

## Install

### Homebrew

```sh
brew tap mcfearsome/tap
brew install ghostty-styles
```

### From source

```sh
cargo install --path .
```

### Build manually

```sh
git clone https://github.com/mcfearsome/ghostty.styles.tui.git
cd ghostty.styles.tui
cargo build --release
# Binary is at target/release/ghostty-styles
```

## Usage

```sh
ghostty-styles
```

Must be run inside a Ghostty terminal session.

### Keybindings

#### Browse screen

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `Enter` / `l` | Open theme details |
| `/` | Search themes |
| `t` | Filter by tag |
| `s` | Cycle sort order |
| `d` | Toggle dark/light filter |
| `p` | Toggle live OSC preview |
| `a` | Apply theme to config |
| `n` | Create new theme |
| `]` / `[` | Next/previous page |
| `r` | Refresh |
| `c` | Add to collection |
| `C` | Manage collections |
| `q` / `Esc` | Quit |

#### Detail screen

| Key | Action |
|-----|--------|
| `h` / `Esc` | Back to browse |
| `p` | Toggle live preview |
| `a` | Apply theme |
| `c` | Add to collection |
| `f` | Fork into theme creator |

#### Creator screen

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate color fields |
| `Enter` / `l` | Edit selected color |
| Arrow keys | Adjust HSL sliders (Shift for x10) |
| `Tab` | Toggle hex input / slider mode |
| `g` | Toggle palette generation algorithm |
| `p` | Toggle live OSC preview |
| `s` | Save / metadata screen |
| Mouse click/drag | Select fields, drag sliders |
| `Esc` | Exit creator |

### Theme Creation

Create themes from scratch or fork existing ones:

```sh
# Open the theme creator
ghostty-styles create

# Fork an existing theme
ghostty-styles create --from catppuccin-mocha
```

Or press `n` on the Browse screen to create, or `f` on the Detail screen to fork.

The creator has three panels:
- **Color fields** — all 22 Ghostty color keys (bg, fg, cursor, selection, palette 0-15)
- **HSL picker** — hue, saturation, lightness sliders with gradient bars, or hex input
- **Preview** — live theme preview with palette swatches and sample output

Press `g` to toggle between hue-rotation and base16-style palette generation. Press `s` to enter metadata (title, description, tags, author) then apply, export, or upload.

### Applying themes

When you apply a theme, `ghostty-styles` will:

1. Create a backup of your config at `config.bak`
2. Remove existing color keys (background, foreground, palette, cursor-color, etc.)
3. Append the theme's configuration

Config file locations:
- **macOS:** `~/Library/Application Support/com.mitchellh.ghostty/config`
- **Linux:** `~/.config/ghostty/config`

### Collections

Create named collections of themes to cycle through:

```sh
# Create a collection
ghostty-styles collection create my-themes

# Add themes (by slug from the API)
ghostty-styles collection add my-themes catppuccin-mocha

# Or add themes from the TUI — press 'c' while browsing

# Set a collection as active
ghostty-styles collection use my-themes

# List collections
ghostty-styles collection list

# Show collection details
ghostty-styles collection show my-themes
```

Press `C` in the TUI to manage collections (reorder, set interval, toggle shuffle, remove themes).

### Theme Cycling

Cycle through themes in your active collection:

```sh
# Apply the next theme
ghostty-styles next

# Start automatic cycling (uses collection's interval)
ghostty-styles cycle start

# Check daemon status
ghostty-styles cycle status

# Stop the daemon
ghostty-styles cycle stop
```

#### Shell Hook

For automatic theme switching on new tabs/windows, add this to your shell rc file (or let `ghostty-styles collection create` install it for you):

```sh
# ghostty-styles theme cycling
if command -v ghostty-styles &>/dev/null && [ "$TERM_PROGRAM" = "ghostty" ]; then
  ghostty-styles next 2>/dev/null
fi
```

## License

MIT
