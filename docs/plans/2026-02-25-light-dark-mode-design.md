# Light/Dark Mode Awareness Design

**Goal:** Automatically filter theme cycling and browsing based on dark/light mode preference, with OS detection, time-of-day scheduling, and manual override.

## Mode Preference

A global `mode_preference` stored in `AppConfig` (`~/.config/ghostty-styles/config.json`):

- `null` — no filtering (default, current behavior)
- `"dark"` / `"light"` — manual pin
- `"auto-os"` — detect from OS appearance, instant switch on change
- `"auto-time"` — schedule-based with `dark_after` and `light_after` times (defaults: 19:00, 07:00)

CLI: `ghostty-styles mode dark|light|auto-os|auto-time|off|status`
TUI: `m` on Browse screen cycles through modes.

## OS Dark Mode Detection (`darkmode.rs`)

### Detection

- **macOS:** `defaults read -g AppleInterfaceStyle` — returns "Dark" or errors (= light). ~5ms.
- **Linux:** Try `$GTK_THEME` env var, then `gsettings get org.gnome.desktop.interface color-scheme`, then `dconf`. Returns `None` if undetectable.

### Event-Driven Listener

- **macOS:** CoreFoundation run loop subscribed to `AppleInterfaceThemeChangedNotification` via `DistributedNotificationCenter`. Uses `objc2` + `objc2-foundation` crates. Sends mode changes over `mpsc` channel.
- **Linux:** `gsettings monitor org.gnome.desktop.interface color-scheme` subprocess. Parse output lines, send over channel.
- **Fallback:** If event listening fails, poll every 30 seconds.

Public API:
```rust
pub fn detect_current() -> Option<bool>  // Some(true)=dark, None=unknown
pub fn spawn_watcher() -> mpsc::Receiver<bool>  // emits on change
```

## Cycling Integration

`cycling::apply_next()` gains a mode filter:

1. Resolve desired mode from preference (manual, OS detection, or time check)
2. Filter collection themes by `is_dark` match
3. If no themes match, skip filtering + log warning to stderr
4. Pick next from filtered subset (sequential/shuffle within subset)
5. `current_index` still tracks position in the full collection

### Daemon Changes

Replace `sleep(interval)` loop with select over:
- Watcher channel (OS mode changed → immediate `apply_next()`)
- Interval timer (normal cycling)
- Time boundary timer (for auto-time, fires at dark_after/light_after)
- SIGTERM signal

## TUI Integration

- `m` keybinding on Browse: cycle mode (off → dark → light → auto-os → auto-time)
- Top bar shows mode indicator: `mode:auto-os (dark)`
- When mode is active, `d` key shows message "mode preference active, press m to change"
- On startup with auto mode, set `dark_filter` to match resolved mode

## New Dependencies

- `objc2` + `objc2-foundation` (macOS only, `#[cfg(target_os = "macos")]`)
- No new deps for Linux

## Edge Cases

- No matching themes in collection for current mode → skip filtering, log warning
- OS detection unavailable → treat as no filter, show status message
- Mode preference cleared (`off`) → revert to current behavior
