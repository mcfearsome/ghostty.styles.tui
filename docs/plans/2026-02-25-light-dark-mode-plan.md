# Light/Dark Mode Awareness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add global mode preference (dark/light/auto-os/auto-time) that filters theme cycling and browsing, with event-driven OS dark mode detection for instant switching.

**Architecture:** New `darkmode.rs` module handles OS detection and event listening. `AppConfig` gets mode preference fields. `cycling::apply_next()` filters by resolved mode. Daemon select-loops over watcher channel, interval timer, and time boundary timer. TUI gets `m` keybinding for mode cycling.

**Tech Stack:** Rust, `objc2` + `objc2-foundation` (macOS only), `gsettings monitor` subprocess (Linux), `std::sync::mpsc` for watcher channel

---

### Task 1: Add mode preference to AppConfig and persistence

**Files:**
- Modify: `src/collection.rs`
- Modify: `src/cli.rs`

**Context:** `AppConfig` currently only has `active_collection: Option<String>`. We need to add `mode_preference`, `dark_after`, and `light_after` fields. Also need a `ModePreference` enum and a `Mode` CLI subcommand.

**Step 1: Add ModePreference enum and update AppConfig in `src/collection.rs`**

Add after the `CycleOrder` enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ModePreference {
    Dark,
    Light,
    AutoOs,
    AutoTime,
}

impl ModePreference {
    pub fn label(&self) -> &'static str {
        match self {
            ModePreference::Dark => "dark",
            ModePreference::Light => "light",
            ModePreference::AutoOs => "auto-os",
            ModePreference::AutoTime => "auto-time",
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            ModePreference::Dark => Some(ModePreference::Light),
            ModePreference::Light => Some(ModePreference::AutoOs),
            ModePreference::AutoOs => Some(ModePreference::AutoTime),
            ModePreference::AutoTime => None, // cycles back to None (off)
        }
    }
}
```

Update `AppConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_collection: Option<String>,
    #[serde(default)]
    pub mode_preference: Option<ModePreference>,
    #[serde(default = "default_dark_after")]
    pub dark_after: String,
    #[serde(default = "default_light_after")]
    pub light_after: String,
}

fn default_dark_after() -> String { "19:00".to_string() }
fn default_light_after() -> String { "07:00".to_string() }
```

Update the `Default` impl:

```rust
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_collection: None,
            mode_preference: None,
            dark_after: default_dark_after(),
            light_after: default_light_after(),
        }
    }
}
```

**Step 2: Add Mode subcommand to `src/cli.rs`**

Add to the `Commands` enum:

```rust
    /// Set dark/light mode preference
    Mode {
        #[command(subcommand)]
        action: ModeAction,
    },
```

Add the `ModeAction` enum:

```rust
#[derive(Subcommand)]
pub enum ModeAction {
    /// Set mode to dark (only dark themes)
    Dark,
    /// Set mode to light (only light themes)
    Light,
    /// Auto-detect from OS dark mode setting
    AutoOs,
    /// Auto-switch based on time of day
    AutoTime {
        /// Time to switch to dark themes (HH:MM, default 19:00)
        #[arg(long, default_value = "19:00")]
        dark_after: String,
        /// Time to switch to light themes (HH:MM, default 07:00)
        #[arg(long, default_value = "07:00")]
        light_after: String,
    },
    /// Disable mode filtering
    Off,
    /// Show current mode status
    Status,
}
```

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/collection.rs src/cli.rs
git commit -m "feat: add ModePreference to AppConfig and Mode CLI subcommand"
```

---

### Task 2: OS dark mode detection (`darkmode.rs`)

**Files:**
- Create: `src/darkmode.rs`
- Modify: `src/main.rs` (add `mod darkmode;`)

**Context:** This module provides `detect_current()` to query the OS for dark/light mode, and `spawn_watcher()` that returns an `mpsc::Receiver<bool>` that emits when the OS mode changes. On macOS, detection uses `defaults read`; the watcher uses `objc2` for `DistributedNotificationCenter`. On Linux, detection uses `gsettings`; the watcher uses `gsettings monitor`. Falls back to polling if event listening fails.

**Step 1: Create `src/darkmode.rs` with detection logic**

```rust
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Detect the current OS dark mode setting.
/// Returns Some(true) if dark, Some(false) if light, None if undetectable.
pub fn detect_current() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        detect_macos()
    }
    #[cfg(target_os = "linux")]
    {
        detect_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn detect_macos() -> Option<bool> {
    let output = Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
        .ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Some(stdout.trim().eq_ignore_ascii_case("dark"))
    } else {
        // Command fails when in light mode (no AppleInterfaceStyle key)
        Some(false)
    }
}

#[cfg(target_os = "linux")]
fn detect_linux() -> Option<bool> {
    // Try GTK_THEME env var
    if let Ok(theme) = std::env::var("GTK_THEME") {
        let lower = theme.to_lowercase();
        if lower.contains("dark") {
            return Some(true);
        }
    }

    // Try gsettings
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Some(stdout.contains("prefer-dark"));
        }
    }

    // Try dconf
    if let Ok(output) = Command::new("dconf")
        .args(["read", "/org/gnome/desktop/interface/color-scheme"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Some(stdout.contains("prefer-dark"));
        }
    }

    None
}

/// Spawn a background thread that watches for OS dark mode changes.
/// Returns a Receiver that emits `true` for dark, `false` for light.
/// Falls back to polling every 30 seconds if event listening is unavailable.
pub fn spawn_watcher() -> mpsc::Receiver<bool> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        #[cfg(target_os = "macos")]
        {
            if watch_macos(&tx) {
                return; // watcher took over
            }
        }
        #[cfg(target_os = "linux")]
        {
            if watch_linux(&tx) {
                return; // watcher took over
            }
        }

        // Fallback: poll every 30 seconds
        let mut last = detect_current();
        loop {
            thread::sleep(Duration::from_secs(30));
            let current = detect_current();
            if current != last {
                if let Some(is_dark) = current {
                    let _ = tx.send(is_dark);
                }
                last = current;
            }
        }
    });

    rx
}

/// macOS: use DistributedNotificationCenter to watch for theme changes.
/// Returns true if the watcher was successfully set up (blocks forever).
#[cfg(target_os = "macos")]
fn watch_macos(tx: &mpsc::Sender<bool>) -> bool {
    use std::io::BufRead;

    // Use a helper approach: spawn a small Swift script via `swift` that
    // listens for the notification and prints "changed" on each toggle.
    // This avoids needing objc2 crate dependency.
    let script = r#"
import Foundation
let center = DistributedNotificationCenter.default()
let name = NSNotification.Name("AppleInterfaceThemeChangedNotification")
center.addObserver(forName: name, object: nil, queue: nil) { _ in
    print("changed", terminator: "\n")
    fflush(stdout)
}
RunLoop.current.run()
"#;

    let mut child = match Command::new("swift")
        .args(["-e", script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return false,
    };

    let reader = std::io::BufReader::new(stdout);
    for line in reader.lines() {
        if line.is_ok() {
            // Theme changed — detect new state
            if let Some(is_dark) = detect_current() {
                let _ = tx.send(is_dark);
            }
        }
    }

    true
}

/// Linux: use `gsettings monitor` to watch for theme changes.
/// Returns true if the watcher was successfully set up (blocks forever).
#[cfg(target_os = "linux")]
fn watch_linux(tx: &mpsc::Sender<bool>) -> bool {
    use std::io::BufRead;

    let mut child = match Command::new("gsettings")
        .args([
            "monitor",
            "org.gnome.desktop.interface",
            "color-scheme",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return false,
    };

    let reader = std::io::BufReader::new(stdout);
    for line in reader.lines() {
        if let Ok(line) = line {
            let is_dark = line.contains("prefer-dark");
            let _ = tx.send(is_dark);
        }
    }

    true
}

/// Resolve the desired mode (dark=true, light=false) from the given preference.
pub fn resolve_mode(
    pref: &crate::collection::ModePreference,
    dark_after: &str,
    light_after: &str,
) -> Option<bool> {
    use crate::collection::ModePreference;
    match pref {
        ModePreference::Dark => Some(true),
        ModePreference::Light => Some(false),
        ModePreference::AutoOs => detect_current(),
        ModePreference::AutoTime => resolve_time(dark_after, light_after),
    }
}

/// Determine whether it's "dark time" based on current local time and the
/// configured dark_after / light_after thresholds.
fn resolve_time(dark_after: &str, light_after: &str) -> Option<bool> {
    let now = chrono_free_now();
    let dark_mins = parse_hhmm(dark_after)?;
    let light_mins = parse_hhmm(light_after)?;

    // If light_after < dark_after (normal: light=07:00, dark=19:00)
    //   light period: light_after..dark_after
    //   dark period: everything else
    if light_mins < dark_mins {
        Some(now < light_mins || now >= dark_mins)
    } else {
        // Inverted: dark_after < light_after (e.g., dark=01:00, light=09:00)
        Some(now >= dark_mins && now < light_mins)
    }
}

/// Get current time as minutes since midnight, no chrono dependency.
fn chrono_free_now() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Get local time offset. Use libc localtime.
    let local_mins = unsafe {
        let t = secs as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        (tm.tm_hour as u32) * 60 + (tm.tm_min as u32)
    };
    local_mins
}

/// Parse "HH:MM" into minutes since midnight.
fn parse_hhmm(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    if h >= 24 || m >= 60 {
        return None;
    }
    Some(h * 60 + m)
}

/// Calculate seconds until the next dark/light time boundary.
pub fn seconds_until_boundary(dark_after: &str, light_after: &str) -> Option<u64> {
    let now = chrono_free_now();
    let dark_mins = parse_hhmm(dark_after)?;
    let light_mins = parse_hhmm(light_after)?;

    let boundaries = [dark_mins, light_mins];
    let min_future = boundaries
        .iter()
        .map(|&b| if b > now { b - now } else { b + 1440 - now })
        .min()?;

    Some((min_future as u64) * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hhmm_valid() {
        assert_eq!(parse_hhmm("07:00"), Some(420));
        assert_eq!(parse_hhmm("19:00"), Some(1140));
        assert_eq!(parse_hhmm("00:00"), Some(0));
        assert_eq!(parse_hhmm("23:59"), Some(1439));
    }

    #[test]
    fn parse_hhmm_invalid() {
        assert_eq!(parse_hhmm("25:00"), None);
        assert_eq!(parse_hhmm("12:60"), None);
        assert_eq!(parse_hhmm("abc"), None);
        assert_eq!(parse_hhmm(""), None);
    }

    #[test]
    fn resolve_time_normal_schedule() {
        // light=07:00 (420), dark=19:00 (1140)
        // At 12:00 (720) → light period → Some(false)
        // At 22:00 (1320) → dark period → Some(true)
        // At 03:00 (180) → dark period → Some(true)

        // We can't easily test with fixed time, but we test the parse logic
        assert!(parse_hhmm("19:00").unwrap() > parse_hhmm("07:00").unwrap());
    }

    #[test]
    fn detect_current_returns_option() {
        // Just verify it doesn't panic
        let _ = detect_current();
    }
}
```

**Step 2: Add `mod darkmode;` and `libc` dependency**

In `src/main.rs`, add `mod darkmode;` with the other module declarations.

In `Cargo.toml`, add:
```toml
libc = "0.2"
```

**Step 3: Verify it compiles**

Run: `cargo build`

**Step 4: Commit**

```bash
git add src/darkmode.rs src/main.rs Cargo.toml
git commit -m "feat: add darkmode detection with OS watcher and time-of-day support"
```

---

### Task 3: Mode-aware cycling

**Files:**
- Modify: `src/cycling.rs`

**Context:** `apply_next()` currently picks the next theme from the entire collection. We need to filter by the resolved mode preference first. If no themes match the desired mode, skip filtering and warn.

**Step 1: Update `apply_next()` to accept and apply mode filter**

Replace the entire function body to add mode filtering. The key change is between loading the collection and selecting the next index:

```rust
use crate::collection::{self, CycleOrder, ModePreference};
use crate::config;
use crate::darkmode;
use crate::theme::GhosttyConfig;
use rand::Rng;

/// Advance to the next theme in the active collection and apply it.
/// Respects the global mode preference to filter themes.
pub fn apply_next() -> Result<String, String> {
    let app_config = collection::load_config();
    let coll_name = app_config
        .active_collection
        .ok_or("No active collection. Run: ghostty-styles collection use <name>")?;

    let mut coll = collection::load_collection(&coll_name)?;

    if coll.themes.is_empty() {
        return Err(format!("Collection '{}' is empty", coll_name));
    }

    // Clamp current_index
    if coll.current_index >= coll.themes.len() {
        coll.current_index = 0;
    }

    // Resolve mode filter
    let want_dark: Option<bool> = app_config.mode_preference.as_ref().and_then(|pref| {
        darkmode::resolve_mode(pref, &app_config.dark_after, &app_config.light_after)
    });

    // Build list of eligible indices
    let eligible: Vec<usize> = if let Some(dark) = want_dark {
        let filtered: Vec<usize> = coll
            .themes
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_dark == dark)
            .map(|(i, _)| i)
            .collect();
        if filtered.is_empty() {
            eprintln!(
                "[warning] No {} themes in '{}', ignoring mode filter",
                if dark { "dark" } else { "light" },
                coll_name
            );
            (0..coll.themes.len()).collect()
        } else {
            filtered
        }
    } else {
        (0..coll.themes.len()).collect()
    };

    // Find current position within eligible list
    let current_eligible_pos = eligible
        .iter()
        .position(|&i| i == coll.current_index)
        .unwrap_or(0);

    let next_eligible_pos = match coll.order {
        CycleOrder::Sequential => (current_eligible_pos + 1) % eligible.len(),
        CycleOrder::Shuffle => {
            let mut rng = rand::thread_rng();
            if eligible.len() == 1 {
                0
            } else {
                let mut next = current_eligible_pos;
                while next == current_eligible_pos {
                    next = rng.gen_range(0..eligible.len());
                }
                next
            }
        }
    };

    let next_index = eligible[next_eligible_pos];
    let theme_entry = &coll.themes[next_index];

    let ghost_config = GhosttyConfig {
        id: String::new(),
        slug: theme_entry.slug.clone(),
        title: theme_entry.title.clone(),
        description: None,
        raw_config: theme_entry.raw_config.clone(),
        background: String::new(),
        foreground: String::new(),
        cursor_color: None,
        cursor_text: None,
        selection_bg: None,
        selection_fg: None,
        palette: Vec::new(),
        font_family: None,
        font_size: None,
        cursor_style: None,
        bg_opacity: None,
        is_dark: theme_entry.is_dark,
        tags: Vec::new(),
        source_url: None,
        author_name: None,
        author_url: None,
        is_featured: false,
        vote_count: 0,
        view_count: 0,
        download_count: 0,
    };

    config::apply_theme(&ghost_config)?;

    coll.current_index = next_index;
    collection::save_collection(&coll)?;

    let mode_label = want_dark.map(|d| if d { " [dark]" } else { " [light]" }).unwrap_or("");
    Ok(format!("Applied '{}' from '{}'{}", theme_entry.title, coll_name, mode_label))
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/cycling.rs
git commit -m "feat: add mode-aware theme filtering to cycling"
```

---

### Task 4: Mode-aware daemon with watcher channel

**Files:**
- Modify: `src/daemon.rs`

**Context:** The daemon currently does `loop { sleep(interval); apply_next(); }`. We need to replace this with a select over: watcher channel (OS mode change → immediate apply), interval timer, time boundary timer, and clean shutdown.

**Step 1: Update daemon `start()` with watcher integration**

Replace the main loop section (from `println!("Daemon started...")` onward):

```rust
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::collection;
use crate::cycling;
use crate::darkmode;

// ... parse_interval and is_process_alive remain the same ...

pub fn start() -> Result<(), String> {
    // ... existing PID check, collection validation, interval parsing ...
    // (everything before the main loop stays the same)

    let pid_file = collection::pid_path();

    if pid_file.exists() {
        let contents = fs::read_to_string(&pid_file)
            .map_err(|e| format!("Failed to read PID file: {}", e))?;
        let existing_pid: i32 = contents
            .trim()
            .parse()
            .map_err(|_| "Corrupt PID file".to_string())?;

        if is_process_alive(existing_pid) {
            return Err(format!(
                "Daemon is already running (PID {}). Stop it first with: ghostty-styles cycle stop",
                existing_pid
            ));
        }

        let _ = fs::remove_file(&pid_file);
    }

    let app_config = collection::load_config();
    let coll_name = app_config
        .active_collection
        .ok_or("No active collection. Run: ghostty-styles collection use <name>")?;

    let coll = collection::load_collection(&coll_name)?;

    let interval_str = coll
        .interval
        .as_deref()
        .ok_or(format!(
            "Collection '{}' has no interval set. Set one before starting the daemon.",
            coll_name
        ))?;

    let interval = parse_interval(interval_str)?;

    if coll.themes.is_empty() {
        return Err(format!("Collection '{}' has no themes", coll_name));
    }

    collection::ensure_dirs()?;
    let my_pid = std::process::id();
    fs::write(&pid_file, my_pid.to_string())
        .map_err(|e| format!("Failed to write PID file: {}", e))?;

    let mode_label = app_config
        .mode_preference
        .as_ref()
        .map(|p| format!(", mode: {}", p.label()))
        .unwrap_or_default();
    println!(
        "Daemon started (PID {}) — collection '{}', interval {}{}",
        my_pid, coll_name, interval_str, mode_label
    );

    // Spawn OS mode watcher (if auto-os mode is active)
    let watcher_rx: Option<mpsc::Receiver<bool>> =
        if app_config.mode_preference == Some(collection::ModePreference::AutoOs) {
            Some(darkmode::spawn_watcher())
        } else {
            None
        };

    // Main loop with multi-source waiting
    let mut next_cycle = Instant::now() + interval;

    // For auto-time mode, calculate next boundary
    let mut next_boundary: Option<Instant> =
        if app_config.mode_preference == Some(collection::ModePreference::AutoTime) {
            darkmode::seconds_until_boundary(&app_config.dark_after, &app_config.light_after)
                .map(|s| Instant::now() + Duration::from_secs(s))
        } else {
            None
        };

    loop {
        // Calculate sleep duration: minimum of interval timer and boundary timer
        let now = Instant::now();
        let mut sleep_dur = next_cycle.saturating_duration_since(now);

        if let Some(boundary) = next_boundary {
            let boundary_dur = boundary.saturating_duration_since(now);
            sleep_dur = sleep_dur.min(boundary_dur);
        }

        // Sleep, but wake up for watcher events
        let triggered_by_watcher = if let Some(ref rx) = watcher_rx {
            match rx.recv_timeout(sleep_dur) {
                Ok(_is_dark) => true,
                Err(mpsc::RecvTimeoutError::Timeout) => false,
                Err(mpsc::RecvTimeoutError::Disconnected) => false,
            }
        } else {
            thread::sleep(sleep_dur);
            false
        };

        // Determine what triggered us and act
        let now = Instant::now();

        if triggered_by_watcher {
            // OS mode changed — immediate apply
            eprintln!("[daemon] OS dark mode changed, switching theme");
            match cycling::apply_next() {
                Ok(msg) => eprintln!("[daemon] {}", msg),
                Err(e) => eprintln!("[daemon] Error: {}", e),
            }
        }

        if now >= next_cycle {
            // Normal interval cycle
            match cycling::apply_next() {
                Ok(msg) => eprintln!("[daemon] {}", msg),
                Err(e) => eprintln!("[daemon] Error: {}", e),
            }
            next_cycle = now + interval;
        }

        if let Some(boundary) = next_boundary {
            if now >= boundary {
                // Time boundary crossed — apply and recalculate
                eprintln!("[daemon] Time boundary crossed, switching theme");
                match cycling::apply_next() {
                    Ok(msg) => eprintln!("[daemon] {}", msg),
                    Err(e) => eprintln!("[daemon] Error: {}", e),
                }
                // Recalculate next boundary
                next_boundary = darkmode::seconds_until_boundary(
                    &app_config.dark_after,
                    &app_config.light_after,
                )
                .map(|s| Instant::now() + Duration::from_secs(s));
            }
        }
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/daemon.rs
git commit -m "feat: daemon watches for OS mode changes and time boundaries"
```

---

### Task 5: CLI mode command handler

**Files:**
- Modify: `src/main.rs`

**Context:** Wire up the `Mode` subcommand to read/write `AppConfig`. The `mode status` command should also show the current detected OS mode.

**Step 1: Add Mode command dispatch in `src/main.rs`**

In `dispatch_command`, add the `Commands::Mode` match arm:

```rust
        Commands::Mode { action } => {
            handle_mode(action);
        }
```

Add the import at the top of the function or file:

```rust
use cli::ModeAction;
```

Add the `handle_mode` function:

```rust
fn handle_mode(action: ModeAction) {
    use collection::ModePreference;

    let mut config = collection::load_config();

    match action {
        ModeAction::Dark => {
            config.mode_preference = Some(ModePreference::Dark);
            save_mode_config(&config);
            println!("Mode: dark (only dark themes will be used)");
        }
        ModeAction::Light => {
            config.mode_preference = Some(ModePreference::Light);
            save_mode_config(&config);
            println!("Mode: light (only light themes will be used)");
        }
        ModeAction::AutoOs => {
            config.mode_preference = Some(ModePreference::AutoOs);
            save_mode_config(&config);
            let detected = darkmode::detect_current();
            let state = match detected {
                Some(true) => "dark",
                Some(false) => "light",
                None => "undetectable",
            };
            println!("Mode: auto-os (currently {})", state);
        }
        ModeAction::AutoTime {
            dark_after,
            light_after,
        } => {
            // Validate time formats
            if !is_valid_hhmm(&dark_after) {
                eprintln!("Invalid time format for --dark-after: '{}' (use HH:MM)", dark_after);
                std::process::exit(1);
            }
            if !is_valid_hhmm(&light_after) {
                eprintln!("Invalid time format for --light-after: '{}' (use HH:MM)", light_after);
                std::process::exit(1);
            }
            config.mode_preference = Some(ModePreference::AutoTime);
            config.dark_after = dark_after.clone();
            config.light_after = light_after.clone();
            save_mode_config(&config);
            let resolved = darkmode::resolve_mode(
                &ModePreference::AutoTime,
                &dark_after,
                &light_after,
            );
            let state = match resolved {
                Some(true) => "dark",
                Some(false) => "light",
                None => "unknown",
            };
            println!(
                "Mode: auto-time (dark after {}, light after {}, currently {})",
                dark_after, light_after, state
            );
        }
        ModeAction::Off => {
            config.mode_preference = None;
            save_mode_config(&config);
            println!("Mode: off (no filtering)");
        }
        ModeAction::Status => {
            print_mode_status(&config);
        }
    }
}

fn save_mode_config(config: &collection::AppConfig) {
    if let Err(e) = collection::save_config(config) {
        eprintln!("Error saving config: {}", e);
        std::process::exit(1);
    }
}

fn is_valid_hhmm(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return false;
    }
    let h: u32 = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let m: u32 = match parts[1].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    h < 24 && m < 60
}

fn print_mode_status(config: &collection::AppConfig) {
    match &config.mode_preference {
        None => println!("Mode: off (no filtering)"),
        Some(pref) => {
            let resolved = darkmode::resolve_mode(
                pref,
                &config.dark_after,
                &config.light_after,
            );
            let state = match resolved {
                Some(true) => "dark",
                Some(false) => "light",
                None => "undetectable",
            };
            match pref {
                collection::ModePreference::Dark => println!("Mode: dark"),
                collection::ModePreference::Light => println!("Mode: light"),
                collection::ModePreference::AutoOs => {
                    println!("Mode: auto-os (currently {})", state);
                    let os = darkmode::detect_current();
                    println!(
                        "OS detection: {}",
                        match os {
                            Some(true) => "dark mode",
                            Some(false) => "light mode",
                            None => "unavailable",
                        }
                    );
                }
                collection::ModePreference::AutoTime => {
                    println!(
                        "Mode: auto-time (dark after {}, light after {}, currently {})",
                        config.dark_after, config.light_after, state
                    );
                }
            }
        }
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build`

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement mode CLI command for dark/light preference management"
```

---

### Task 6: TUI mode keybinding and browse screen integration

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs` (handle_browse_input)
- Modify: `src/ui/browser.rs`

**Context:** Add `m` keybinding on Browse screen to cycle mode preference. Display mode indicator in top bar. When mode is active, override `d` key behavior. On startup with auto mode, set `dark_filter` to match resolved mode.

**Step 1: Add mode state to App**

In `src/app.rs`, add to the `App` struct:

```rust
    pub mode_preference: Option<crate::collection::ModePreference>,
```

Initialize in `App::new()`:

```rust
    let config = crate::collection::load_config();
    // ... existing fields ...
    mode_preference: config.mode_preference.clone(),
```

Also in `App::new()`, after setting `mode_preference`, resolve and apply to `dark_filter`:

```rust
    // Auto-set dark_filter based on mode preference
    let dark_filter = config.mode_preference.as_ref().and_then(|pref| {
        crate::darkmode::resolve_mode(pref, &config.dark_after, &config.light_after)
    });
```

And use this `dark_filter` value when initializing the `dark_filter` field instead of `None`.

Add a method to cycle mode:

```rust
    pub fn cycle_mode(&mut self) {
        use crate::collection::ModePreference;
        self.mode_preference = match &self.mode_preference {
            None => Some(ModePreference::Dark),
            Some(pref) => pref.next(),
        };
        // Persist
        let mut config = crate::collection::load_config();
        config.mode_preference = self.mode_preference.clone();
        let _ = crate::collection::save_config(&config);
        // Update dark_filter to match
        self.dark_filter = self.mode_preference.as_ref().and_then(|p| {
            crate::darkmode::resolve_mode(p, &config.dark_after, &config.light_after)
        });
        self.page = 1;
        self.trigger_fetch();
    }
```

**Step 2: Add `m` keybinding in `handle_browse_input`**

In `src/main.rs`, in the `InputMode::Normal` match in `handle_browse_input`, add:

```rust
            KeyCode::Char('m') => app.cycle_mode(),
```

Update the `d` key handler to check if mode is active:

```rust
            KeyCode::Char('d') => {
                if app.mode_preference.is_some() {
                    app.status_message = Some("Mode preference active, press m to change".into());
                } else {
                    app.toggle_dark_filter();
                }
            }
```

**Step 3: Update browse screen top bar in `src/ui/browser.rs`**

After the existing `dark_filter` display (around line 89-93), add mode indicator:

```rust
    if let Some(ref pref) = app.mode_preference {
        filter_spans.push(Span::styled(
            format!("mode:{} ", pref.label()),
            Style::default().fg(ACCENT),
        ));
    }
```

**Step 4: Update browse screen bottom bar hints**

Change the `("d", "dark/light")` hint to be context-aware. In the hints array:

```rust
            ("m", "mode"),
```

(Replace or add alongside the `d` hint.)

**Step 5: Verify it compiles**

Run: `cargo build`

**Step 6: Commit**

```bash
git add src/app.rs src/main.rs src/ui/browser.rs
git commit -m "feat: add m keybinding for mode cycling with browse screen integration"
```

---

### Task 7: Update documentation

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

**Context:** Document the new `darkmode.rs` module, mode preference in AppConfig, CLI `mode` command, and `m` keybinding.

**Step 1: Update CLAUDE.md**

Add to Core Modules:
- `darkmode.rs` — OS dark mode detection (macOS `defaults read`, Linux `gsettings`), event-driven watcher thread (`spawn_watcher()`), time-of-day resolution. Used by cycling and daemon for mode-aware filtering.

Update Key Patterns:
- `m` on Browse: cycle mode preference (off → dark → light → auto-os → auto-time)
- Mode preference filters cycling to only match dark/light themes
- Daemon supports OS watcher channel for instant mode switching

**Step 2: Update README.md**

Add a "Dark/Light Mode" section after Theme Cycling:

```markdown
### Dark/Light Mode

Control which themes are used based on dark or light mode:

```sh
# Set manual mode
ghostty-styles mode dark
ghostty-styles mode light

# Auto-detect from OS appearance (instant switching)
ghostty-styles mode auto-os

# Schedule based on time of day
ghostty-styles mode auto-time --dark-after 19:00 --light-after 07:00

# Disable mode filtering
ghostty-styles mode off

# Check current mode
ghostty-styles mode status
```

Press `m` in the TUI to cycle through modes. When a mode is active, cycling and browsing only show matching themes.
```

Update the Browse screen keybindings table:
- Add `m` | Cycle dark/light mode preference
- Update `d` note to mention it's overridden when mode is active

**Step 3: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: add dark/light mode documentation to CLAUDE.md and README"
```
