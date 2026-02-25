# Theme Collection Cycling — Design

## Overview

Add the ability to define named collections of themes that cycle automatically, either on a timed interval or when a new terminal tab/window/split is opened. The binary gains subcommands while preserving the current TUI as the default (no-args) behavior.

## CLI Structure

```
ghostty-styles                          # Launch TUI (current behavior)
ghostty-styles collection create <name> # Create a named collection
ghostty-styles collection list          # List all collections
ghostty-styles collection show <name>   # Show themes in a collection
ghostty-styles collection add <coll> <slug>  # Add theme by slug
ghostty-styles collection use <name>    # Set active collection
ghostty-styles collection delete <name> # Delete a collection
ghostty-styles next                     # Apply next theme from active collection
ghostty-styles cycle start              # Start daemon for timed cycling
ghostty-styles cycle stop               # Stop running daemon
ghostty-styles cycle status             # Show daemon status
```

On `collection create`, prompt the user to:
1. Optionally start the daemon with an interval
2. Optionally install a shell hook in .zshrc/.bashrc (detected from $SHELL)

## Data Model

All state in `~/.config/ghostty-styles/`:

```
~/.config/ghostty-styles/
├── config.json
├── collections/
│   ├── work.json
│   └── chill.json
└── daemon.pid
```

**config.json:**
```json
{
  "active_collection": "work"
}
```

**Collection file:**
```json
{
  "name": "work",
  "themes": [
    {
      "slug": "catppuccin-mocha",
      "title": "Catppuccin Mocha",
      "is_dark": true,
      "raw_config": "background = #1e1e2e\n..."
    }
  ],
  "current_index": 0,
  "order": "sequential",
  "interval": null
}
```

- `order`: `"sequential"` or `"shuffle"`
- `interval`: `null` (manual/hook only) or duration string like `"30m"`, `"1h"`
- `current_index`: position for sequential mode; for shuffle, random pick avoiding repeats until exhausted
- Themes stored fully (raw_config included) so cycling requires no network

## Daemon & Cycling Logic

**`next` command:**
1. Read config.json for active collection
2. Load collection file
3. Pick next theme (sequential: increment + wrap; shuffle: random without repeat)
4. Call `config::apply_theme()` to write Ghostty config
5. Update current_index in collection file
6. Ghostty auto-reloads config on change

**`cycle start`:**
1. Check daemon.pid — error if already running
2. Daemonize (fork to background)
3. Write PID to daemon.pid
4. Loop: sleep for interval, run next logic
5. SIGTERM: remove PID file, exit

**`cycle stop`:** Read daemon.pid, send SIGTERM, remove PID file.

**`cycle status`:** Check PID file + process alive. Show active collection, interval, current theme.

## Shell Hook

For new-tab cycling, detect $SHELL and offer to add to the appropriate rc file:

```sh
# ghostty-styles theme cycling
if command -v ghostty-styles &>/dev/null && [ "$TERM_PROGRAM" = "ghostty" ]; then
  ghostty-styles next 2>/dev/null
fi
```

## TUI Integration

### Adding themes to collections

`c` keybinding on Browse and Detail screens:
- No collections: prompt to create one (inline text input)
- One collection: add directly, show status message
- Multiple: popup selector (like existing tag popup)

### Collections screen

`C` (shift-c) from Browse opens new `Screen::Collections`:
- Left panel: list of collections (active highlighted)
- Right panel: selected collection's themes

Keybindings:

| Key | Action |
|-----|--------|
| j/k | Navigate collections |
| Enter/l | View collection's themes |
| n | New collection |
| d | Delete collection (confirm) |
| u | Set as active |
| s | Toggle order (sequential/shuffle) |
| i | Set interval |
| x | Remove theme (in theme list view) |
| h/Esc | Back |

### Screen flow

```
Browse → Detail → Confirm (apply)
  ↓
Collections → Collection themes
```

## New Dependencies

- `clap` — CLI subcommand parsing
- `nix` or `libc` — daemon fork/signal handling

## New Modules

- `cli.rs` — clap definitions and dispatch
- `collection.rs` — CRUD, data model, next-theme logic
- `daemon.rs` — fork, PID file, signal handling, cycle loop
- `shell_hook.rs` — detect shell, install/remove rc snippet
- `ui/collections.rs` — Collections screen widget

## Modified Modules

- `main.rs` — check subcommands first, fall through to TUI
- `app.rs` — add Screen::Collections, collection state, new input modes
- `ui/browser.rs` — `c` and `C` keybindings
- `ui/details.rs` — `c` keybinding
- `config.rs` — make apply_theme callable from both TUI and CLI
