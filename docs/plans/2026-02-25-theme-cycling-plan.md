# Theme Collection Cycling — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add named theme collections that cycle automatically via timer or shell hook, with full TUI and CLI management.

**Architecture:** The binary gains clap subcommands while keeping the TUI as default. A new `collection.rs` module handles persistence to `~/.config/ghostty-styles/`. A lightweight daemon process handles timed cycling. The TUI gains a Collections screen and add-to-collection keybinding.

**Tech Stack:** Rust, clap (CLI), serde_json (persistence), nix crate (daemon/signals), ratatui (TUI)

---

### Task 1: Add clap dependency and CLI skeleton

**Files:**
- Modify: `Cargo.toml:18-25` (dependencies)
- Create: `src/cli.rs`
- Modify: `src/main.rs:1-56`

**Step 1: Add clap and rand dependencies to Cargo.toml**

Add to `[dependencies]`:
```toml
clap = { version = "4", features = ["derive"] }
nix = { version = "0.29", features = ["signal", "process"] }
rand = "0.8"
```

**Step 2: Create src/cli.rs with clap command definitions**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ghostty-styles", about = "Browse, preview, and cycle Ghostty themes")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage theme collections
    Collection {
        #[command(subcommand)]
        action: CollectionAction,
    },
    /// Apply the next theme from the active collection
    Next,
    /// Manage the cycling daemon
    Cycle {
        #[command(subcommand)]
        action: CycleAction,
    },
}

#[derive(Subcommand)]
pub enum CollectionAction {
    /// Create a new collection
    Create { name: String },
    /// List all collections
    List,
    /// Show themes in a collection
    Show { name: String },
    /// Add a theme by slug to a collection
    Add { collection: String, slug: String },
    /// Set a collection as active
    Use { name: String },
    /// Delete a collection
    Delete { name: String },
}

#[derive(Subcommand)]
pub enum CycleAction {
    /// Start the cycling daemon
    Start,
    /// Stop the cycling daemon
    Stop,
    /// Show daemon status
    Status,
}
```

**Step 3: Update main.rs to parse CLI args and dispatch**

Replace the current `fn main()` with:
```rust
mod cli;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => run_tui(),
        Some(Commands::Collection { action }) => handle_collection(action),
        Some(Commands::Next) => handle_next(),
        Some(Commands::Cycle { action }) => handle_cycle(action),
    }
}
```

Move existing TUI startup logic into `fn run_tui()`. Add stub functions `handle_collection`, `handle_next`, `handle_cycle` that print "not yet implemented".

**Step 4: Verify it compiles and TUI still works**

Run: `cargo build`
Expected: compiles with no errors

Run: `cargo run` (in Ghostty)
Expected: TUI launches as before

Run: `cargo run -- --help`
Expected: Shows clap help with subcommands

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/cli.rs src/main.rs
git commit -m "feat: add clap CLI skeleton with subcommands"
```

---

### Task 2: Collection data model and persistence

**Files:**
- Create: `src/collection.rs`
- Modify: `src/main.rs` (add `mod collection`)

**Step 1: Create src/collection.rs with data model**

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionTheme {
    pub slug: String,
    pub title: String,
    pub is_dark: bool,
    pub raw_config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    pub themes: Vec<CollectionTheme>,
    pub current_index: usize,
    pub order: CycleOrder,
    pub interval: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CycleOrder {
    Sequential,
    Shuffle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_collection: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_collection: None,
        }
    }
}

/// Base directory: ~/.config/ghostty-styles/
pub fn base_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ghostty-styles")
}

pub fn collections_dir() -> PathBuf {
    base_dir().join("collections")
}

pub fn config_path() -> PathBuf {
    base_dir().join("config.json")
}

pub fn pid_path() -> PathBuf {
    base_dir().join("daemon.pid")
}

pub fn ensure_dirs() -> Result<(), String> {
    fs::create_dir_all(collections_dir()).map_err(|e| format!("Failed to create dirs: {}", e))
}

pub fn load_config() -> AppConfig {
    config_path()
        .exists()
        .then(|| {
            fs::read_to_string(config_path())
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        })
        .flatten()
        .unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    ensure_dirs()?;
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| format!("Failed to write config: {}", e))
}

pub fn load_collection(name: &str) -> Result<Collection, String> {
    let path = collections_dir().join(format!("{}.json", name));
    let data = fs::read_to_string(&path).map_err(|e| format!("Failed to read collection '{}': {}", name, e))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse collection '{}': {}", name, e))
}

pub fn save_collection(collection: &Collection) -> Result<(), String> {
    ensure_dirs()?;
    let path = collections_dir().join(format!("{}.json", collection.name));
    let json = serde_json::to_string_pretty(collection).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Failed to write collection: {}", e))
}

pub fn list_collections() -> Vec<String> {
    let dir = collections_dir();
    if !dir.exists() {
        return Vec::new();
    }
    fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.strip_suffix(".json").map(|n| n.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn delete_collection(name: &str) -> Result<(), String> {
    let path = collections_dir().join(format!("{}.json", name));
    fs::remove_file(path).map_err(|e| format!("Failed to delete collection '{}': {}", name, e))
}

pub fn create_collection(name: &str) -> Result<Collection, String> {
    let collection = Collection {
        name: name.to_string(),
        themes: Vec::new(),
        current_index: 0,
        order: CycleOrder::Sequential,
        interval: None,
    };
    save_collection(&collection)?;
    Ok(collection)
}
```

**Step 2: Add `mod collection;` to main.rs**

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles

**Step 4: Commit**

```bash
git add src/collection.rs src/main.rs
git commit -m "feat: add collection data model and persistence"
```

---

### Task 3: Implement collection CLI commands

**Files:**
- Modify: `src/main.rs` (flesh out `handle_collection`)
- Modify: `src/collection.rs` (add helper for adding theme by slug)
- Modify: `src/api.rs` (expose `fetch_config_by_id` for slug lookup — already exists but unused)

**Step 1: Implement handle_collection in main.rs**

```rust
fn handle_collection(action: CollectionAction) {
    use cli::CollectionAction;

    match action {
        CollectionAction::Create { name } => {
            match collection::create_collection(&name) {
                Ok(_) => {
                    println!("Created collection '{}'", name);
                    prompt_daemon_and_hook(&name);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        CollectionAction::List => {
            let collections = collection::list_collections();
            let config = collection::load_config();
            if collections.is_empty() {
                println!("No collections. Create one with: ghostty-styles collection create <name>");
                return;
            }
            for name in &collections {
                let active = config.active_collection.as_deref() == Some(name.as_str());
                let marker = if active { " *" } else { "" };
                let coll = collection::load_collection(name).ok();
                let count = coll.as_ref().map(|c| c.themes.len()).unwrap_or(0);
                println!("{}{} ({} themes)", name, marker, count);
            }
        }
        CollectionAction::Show { name } => {
            match collection::load_collection(&name) {
                Ok(coll) => {
                    println!("Collection: {} ({} themes, {})", coll.name, coll.themes.len(),
                        match coll.order { collection::CycleOrder::Sequential => "sequential", collection::CycleOrder::Shuffle => "shuffle" });
                    if let Some(ref interval) = coll.interval {
                        println!("Interval: {}", interval);
                    }
                    for (i, theme) in coll.themes.iter().enumerate() {
                        let marker = if i == coll.current_index { ">" } else { " " };
                        let mode = if theme.is_dark { "dark" } else { "light" };
                        println!("{} {} ({})", marker, theme.title, mode);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        CollectionAction::Add { collection: coll_name, slug } => {
            match add_theme_by_slug(&coll_name, &slug) {
                Ok(title) => println!("Added '{}' to '{}'", title, coll_name),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        CollectionAction::Use { name } => {
            if collection::load_collection(&name).is_err() {
                eprintln!("Collection '{}' not found", name);
                return;
            }
            let mut config = collection::load_config();
            config.active_collection = Some(name.clone());
            match collection::save_config(&config) {
                Ok(_) => println!("Active collection set to '{}'", name),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        CollectionAction::Delete { name } => {
            match collection::delete_collection(&name) {
                Ok(_) => {
                    println!("Deleted collection '{}'", name);
                    let mut config = collection::load_config();
                    if config.active_collection.as_deref() == Some(&name) {
                        config.active_collection = None;
                        let _ = collection::save_config(&config);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}

fn add_theme_by_slug(collection_name: &str, slug: &str) -> Result<String, String> {
    let mut coll = collection::load_collection(collection_name)?;
    let theme_data = api::fetch_config_by_id(slug)?;
    let theme = collection::CollectionTheme {
        slug: theme_data.slug.clone(),
        title: theme_data.title.clone(),
        is_dark: theme_data.is_dark,
        raw_config: theme_data.raw_config.clone(),
    };
    let title = theme.title.clone();
    coll.themes.push(theme);
    collection::save_collection(&coll)?;
    Ok(title)
}
```

**Step 2: Verify CLI commands work**

Run: `cargo run -- collection create test`
Expected: "Created collection 'test'"

Run: `cargo run -- collection list`
Expected: "test (0 themes)"

Run: `cargo run -- collection delete test`
Expected: "Deleted collection 'test'"

**Step 3: Commit**

```bash
git add src/main.rs src/collection.rs
git commit -m "feat: implement collection CLI commands"
```

---

### Task 4: Implement the `next` command (cycling logic)

**Files:**
- Create: `src/cycling.rs`
- Modify: `src/main.rs` (flesh out `handle_next`)
- Modify: `src/collection.rs` (add `Serialize` derive to `GhosttyConfig` if needed)

**Step 1: Create src/cycling.rs with next-theme logic**

```rust
use rand::Rng;

use crate::collection::{self, Collection, CycleOrder};
use crate::config;
use crate::theme::GhosttyConfig;

/// Advance to the next theme in the active collection and apply it.
pub fn apply_next() -> Result<String, String> {
    let app_config = collection::load_config();
    let coll_name = app_config
        .active_collection
        .ok_or("No active collection. Run: ghostty-styles collection use <name>")?;

    let mut coll = collection::load_collection(&coll_name)?;

    if coll.themes.is_empty() {
        return Err(format!("Collection '{}' is empty", coll_name));
    }

    let next_index = match coll.order {
        CycleOrder::Sequential => (coll.current_index + 1) % coll.themes.len(),
        CycleOrder::Shuffle => {
            let mut rng = rand::thread_rng();
            if coll.themes.len() == 1 {
                0
            } else {
                let mut next = coll.current_index;
                while next == coll.current_index {
                    next = rng.gen_range(0..coll.themes.len());
                }
                next
            }
        }
    };

    let theme_entry = &coll.themes[next_index];

    // Build a minimal GhosttyConfig for apply_theme
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

    Ok(format!("Applied '{}' from '{}'", theme_entry.title, coll_name))
}
```

**Step 2: Wire up handle_next in main.rs**

```rust
fn handle_next() {
    match cycling::apply_next() {
        Ok(msg) => println!("{}", msg),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

Add `mod cycling;` to main.rs.

**Step 3: Test manually**

Create a collection, add a theme, set active, run next:
```bash
cargo run -- collection create test
cargo run -- collection use test
# (add a theme via the API slug)
cargo run -- next
```

**Step 4: Commit**

```bash
git add src/cycling.rs src/main.rs
git commit -m "feat: implement next command for theme cycling"
```

---

### Task 5: Implement the daemon (cycle start/stop/status)

**Files:**
- Create: `src/daemon.rs`
- Modify: `src/main.rs` (flesh out `handle_cycle`)

**Step 1: Create src/daemon.rs**

```rust
use std::fs;
use std::io::Write;
use std::thread;
use std::time::Duration;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::collection;
use crate::cycling;

fn parse_interval(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if let Some(mins) = s.strip_suffix('m') {
        let n: u64 = mins.parse().map_err(|_| format!("Invalid interval: {}", s))?;
        Ok(Duration::from_secs(n * 60))
    } else if let Some(hours) = s.strip_suffix('h') {
        let n: u64 = hours.parse().map_err(|_| format!("Invalid interval: {}", s))?;
        Ok(Duration::from_secs(n * 3600))
    } else if let Some(secs) = s.strip_suffix('s') {
        let n: u64 = secs.parse().map_err(|_| format!("Invalid interval: {}", s))?;
        Ok(Duration::from_secs(n))
    } else {
        Err(format!("Invalid interval '{}'. Use format like 30m, 1h, 90s", s))
    }
}

pub fn start() -> Result<(), String> {
    let pid_path = collection::pid_path();

    // Check if already running
    if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                if signal::kill(Pid::from_raw(pid), None).is_ok() {
                    return Err("Daemon is already running. Use 'ghostty-styles cycle stop' first.".into());
                }
            }
        }
        // Stale PID file
        let _ = fs::remove_file(&pid_path);
    }

    // Get active collection and its interval
    let app_config = collection::load_config();
    let coll_name = app_config
        .active_collection
        .ok_or("No active collection. Run: ghostty-styles collection use <name>")?;
    let coll = collection::load_collection(&coll_name)?;
    let interval_str = coll.interval.as_deref()
        .ok_or(format!("Collection '{}' has no interval set. Use the TUI or set it first.", coll_name))?;
    let interval = parse_interval(interval_str)?;

    // Write PID file
    collection::ensure_dirs()?;
    let mut f = fs::File::create(&pid_path).map_err(|e| format!("Failed to write PID file: {}", e))?;
    write!(f, "{}", std::process::id()).map_err(|e| e.to_string())?;

    println!("Daemon started (PID {}). Cycling '{}' every {}.", std::process::id(), coll_name, interval_str);
    println!("Stop with: ghostty-styles cycle stop");

    // Cycle loop
    loop {
        thread::sleep(interval);
        match cycling::apply_next() {
            Ok(msg) => eprintln!("[daemon] {}", msg),
            Err(e) => eprintln!("[daemon] Error: {}", e),
        }
    }
}

pub fn stop() -> Result<(), String> {
    let pid_path = collection::pid_path();
    if !pid_path.exists() {
        return Err("No daemon running (no PID file found).".into());
    }

    let pid_str = fs::read_to_string(&pid_path).map_err(|e| format!("Failed to read PID file: {}", e))?;
    let pid: i32 = pid_str.trim().parse().map_err(|_| "Invalid PID file".to_string())?;

    signal::kill(Pid::from_raw(pid), Signal::SIGTERM)
        .map_err(|e| format!("Failed to stop daemon (PID {}): {}", pid, e))?;

    let _ = fs::remove_file(&pid_path);
    println!("Daemon stopped (PID {}).", pid);
    Ok(())
}

pub fn status() -> Result<(), String> {
    let pid_path = collection::pid_path();
    let app_config = collection::load_config();

    let running = if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                signal::kill(Pid::from_raw(pid), None).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if running {
        let pid_str = fs::read_to_string(&pid_path).unwrap_or_default();
        println!("Daemon: running (PID {})", pid_str.trim());
    } else {
        println!("Daemon: not running");
    }

    match app_config.active_collection {
        Some(name) => {
            if let Ok(coll) = collection::load_collection(&name) {
                println!("Active collection: {} ({} themes, {})", name, coll.themes.len(),
                    match coll.order { collection::CycleOrder::Sequential => "sequential", collection::CycleOrder::Shuffle => "shuffle" });
                if let Some(ref interval) = coll.interval {
                    println!("Interval: {}", interval);
                }
                if !coll.themes.is_empty() {
                    let current = &coll.themes[coll.current_index.min(coll.themes.len() - 1)];
                    println!("Current theme: {}", current.title);
                }
            } else {
                println!("Active collection: {} (not found)", name);
            }
        }
        None => println!("No active collection set."),
    }
    Ok(())
}
```

**Step 2: Wire up handle_cycle in main.rs**

```rust
fn handle_cycle(action: CycleAction) {
    use cli::CycleAction;
    let result = match action {
        CycleAction::Start => daemon::start(),
        CycleAction::Stop => daemon::stop(),
        CycleAction::Status => daemon::status(),
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

Add `mod daemon;` to main.rs.

**Step 3: Verify**

Run: `cargo run -- cycle status`
Expected: "Daemon: not running" / "No active collection set."

**Step 4: Commit**

```bash
git add src/daemon.rs src/main.rs
git commit -m "feat: implement cycling daemon (start/stop/status)"
```

---

### Task 6: Shell hook installer

**Files:**
- Create: `src/shell_hook.rs`
- Modify: `src/main.rs` (wire into `prompt_daemon_and_hook`)

**Step 1: Create src/shell_hook.rs**

```rust
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

const HOOK_MARKER: &str = "# ghostty-styles theme cycling";

const HOOK_SNIPPET: &str = r#"# ghostty-styles theme cycling
if command -v ghostty-styles &>/dev/null && [ "$TERM_PROGRAM" = "ghostty" ]; then
  ghostty-styles next 2>/dev/null
fi"#;

/// Detect the user's shell and return the path to the rc file.
pub fn detect_rc_file() -> Option<(String, PathBuf)> {
    let shell = env::var("SHELL").unwrap_or_default();
    let home = dirs::home_dir()?;

    if shell.contains("zsh") {
        Some(("zsh".to_string(), home.join(".zshrc")))
    } else if shell.contains("bash") {
        Some(("bash".to_string(), home.join(".bashrc")))
    } else {
        None
    }
}

/// Check if the hook is already installed in the given file.
pub fn is_installed(rc_path: &PathBuf) -> bool {
    fs::read_to_string(rc_path)
        .map(|content| content.contains(HOOK_MARKER))
        .unwrap_or(false)
}

/// Append the hook snippet to the rc file.
pub fn install(rc_path: &PathBuf) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(rc_path)
        .map_err(|e| format!("Failed to open {}: {}", rc_path.display(), e))?;

    writeln!(file).map_err(|e| e.to_string())?;
    writeln!(file, "{}", HOOK_SNIPPET).map_err(|e| e.to_string())?;

    Ok(())
}

/// Prompt the user to install the shell hook. Returns true if installed.
pub fn prompt_install() -> bool {
    let (shell_name, rc_path) = match detect_rc_file() {
        Some(v) => v,
        None => {
            println!("Could not detect shell. Add this to your shell rc file manually:");
            println!("{}", HOOK_SNIPPET);
            return false;
        }
    };

    if is_installed(&rc_path) {
        println!("Shell hook already installed in {}", rc_path.display());
        return true;
    }

    print!("Install shell hook in {} ({})? [y/N] ", rc_path.display(), shell_name);
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_ok() {
        if input.trim().eq_ignore_ascii_case("y") {
            match install(&rc_path) {
                Ok(_) => {
                    println!("Hook installed. Restart your shell or run: source {}", rc_path.display());
                    return true;
                }
                Err(e) => {
                    eprintln!("Failed to install hook: {}", e);
                }
            }
        }
    }
    false
}
```

**Step 2: Implement prompt_daemon_and_hook in main.rs**

```rust
fn prompt_daemon_and_hook(collection_name: &str) {
    use std::io::{self, BufRead, Write};

    // Ask about interval
    print!("Set a cycling interval? (e.g., 30m, 1h, or press Enter to skip): ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            if let Ok(mut coll) = collection::load_collection(collection_name) {
                coll.interval = Some(trimmed.to_string());
                let _ = collection::save_collection(&coll);
                println!("Interval set to '{}'", trimmed);
            }
        }
    }

    // Ask about shell hook
    shell_hook::prompt_install();
}
```

Add `mod shell_hook;` to main.rs.

**Step 3: Verify**

Run: `cargo run -- collection create test-hook`
Expected: prompts for interval and shell hook

**Step 4: Commit**

```bash
git add src/shell_hook.rs src/main.rs
git commit -m "feat: add shell hook installer for new-tab cycling"
```

---

### Task 7: TUI — add-to-collection keybinding (c)

**Files:**
- Modify: `src/app.rs` (add collection-related state and input modes)
- Modify: `src/main.rs` (add `c` key handling in browse and detail)
- Modify: `src/ui/browser.rs` (render collection popup and hint)
- Modify: `src/ui/details.rs` (add `c` hint)

**Step 1: Add collection state to App in app.rs**

Add to `App` struct:
```rust
pub collection_names: Vec<String>,
pub collection_popup_active: bool,
pub collection_popup_cursor: usize,
pub collection_name_input: String,
pub input_mode: InputMode, // already exists
```

Add `CollectionSelect` and `CollectionCreate` variants to `InputMode`:
```rust
pub enum InputMode {
    Normal,
    Search,
    TagSelect,
    CollectionSelect,
    CollectionCreate,
}
```

Add methods to `App`:
```rust
pub fn open_collection_popup(&mut self) {
    self.collection_names = crate::collection::list_collections();
    if self.collection_names.is_empty() {
        self.input_mode = InputMode::CollectionCreate;
        self.collection_name_input.clear();
    } else if self.collection_names.len() == 1 {
        self.add_to_collection(&self.collection_names[0].clone());
    } else {
        self.input_mode = InputMode::CollectionSelect;
        self.collection_popup_cursor = 0;
    }
}

pub fn add_to_collection(&mut self, name: &str) {
    if let Some(theme) = self.selected_theme() {
        let entry = crate::collection::CollectionTheme {
            slug: theme.slug.clone(),
            title: theme.title.clone(),
            is_dark: theme.is_dark,
            raw_config: theme.raw_config.clone(),
        };
        let title = entry.title.clone();
        match crate::collection::load_collection(name) {
            Ok(mut coll) => {
                coll.themes.push(entry);
                match crate::collection::save_collection(&coll) {
                    Ok(_) => self.status_message = Some(format!("Added '{}' to '{}'", title, name)),
                    Err(e) => self.status_message = Some(format!("Error: {}", e)),
                }
            }
            Err(e) => self.status_message = Some(format!("Error: {}", e)),
        }
    }
    self.input_mode = InputMode::Normal;
}

pub fn create_collection_and_add(&mut self) {
    let name = self.collection_name_input.trim().to_string();
    if name.is_empty() {
        self.input_mode = InputMode::Normal;
        return;
    }
    match crate::collection::create_collection(&name) {
        Ok(_) => self.add_to_collection(&name),
        Err(e) => {
            self.status_message = Some(format!("Error: {}", e));
            self.input_mode = InputMode::Normal;
        }
    }
}
```

Initialize the new fields in `App::new()`.

**Step 2: Add keybinding handling in main.rs**

In `handle_browse_input`, under `InputMode::Normal`, add:
```rust
KeyCode::Char('c') => {
    if !app.themes.is_empty() {
        app.open_collection_popup();
    }
}
```

Add new match arms for `InputMode::CollectionSelect` and `InputMode::CollectionCreate` in `handle_browse_input`:
```rust
InputMode::CollectionSelect => match key {
    KeyCode::Char('j') | KeyCode::Down => {
        app.collection_popup_cursor = (app.collection_popup_cursor + 1).min(app.collection_names.len() - 1);
    }
    KeyCode::Char('k') | KeyCode::Up => {
        app.collection_popup_cursor = app.collection_popup_cursor.saturating_sub(1);
    }
    KeyCode::Enter => {
        let name = app.collection_names[app.collection_popup_cursor].clone();
        app.add_to_collection(&name);
    }
    KeyCode::Char('n') => {
        app.input_mode = InputMode::CollectionCreate;
        app.collection_name_input.clear();
    }
    KeyCode::Esc => {
        app.input_mode = InputMode::Normal;
    }
    _ => {}
},
InputMode::CollectionCreate => match key {
    KeyCode::Enter => {
        app.create_collection_and_add();
    }
    KeyCode::Esc => {
        app.input_mode = InputMode::Normal;
    }
    KeyCode::Backspace => {
        app.collection_name_input.pop();
    }
    KeyCode::Char(c) => {
        app.collection_name_input.push(c);
    }
    _ => {}
},
```

Also add `c` handling in `handle_detail_input`.

**Step 3: Add collection popup rendering to browser.rs**

Add a `render_collection_popup` function similar to the existing `render_tag_popup`. Show it when `app.input_mode == InputMode::CollectionSelect` or `CollectionCreate`.

**Step 4: Add `c` hint to detail footer in details.rs**

Add to the non-confirm footer hints:
```rust
Span::styled("c", Style::default().fg(ACCENT)),
Span::styled(" collect  ", Style::default().fg(DIM)),
```

**Step 5: Verify**

Run: `cargo run` (TUI), press `c` on a theme
Expected: popup appears or prompts for collection name

**Step 6: Commit**

```bash
git add src/app.rs src/main.rs src/ui/browser.rs src/ui/details.rs
git commit -m "feat: add collect keybinding (c) for adding themes to collections"
```

---

### Task 8: TUI — Collections screen

**Files:**
- Create: `src/ui/collections.rs`
- Modify: `src/ui/mod.rs` (add module and export)
- Modify: `src/app.rs` (add Screen::Collections, CollectionDetail state)
- Modify: `src/main.rs` (add C keybinding, screen rendering, input handling)

**Step 1: Add Screen::Collections and state to app.rs**

Add to `Screen` enum:
```rust
pub enum Screen {
    Browse,
    Detail,
    Confirm,
    Collections,
}
```

Add state fields to `App`:
```rust
pub collections_list: Vec<String>,
pub collections_cursor: usize,
pub collections_detail: Option<Collection>,  // loaded when viewing a collection's themes
pub collections_theme_cursor: usize,
pub collections_viewing_themes: bool,
pub collections_input_mode: CollectionsInputMode,
pub collections_interval_input: String,
pub collections_name_input: String,
```

```rust
pub enum CollectionsInputMode {
    Normal,
    NewCollection,
    SetInterval,
    ConfirmDelete,
}
```

Add methods:
```rust
pub fn enter_collections(&mut self) {
    self.collections_list = crate::collection::list_collections();
    self.collections_cursor = 0;
    self.collections_viewing_themes = false;
    self.collections_detail = None;
    self.collections_input_mode = CollectionsInputMode::Normal;
    self.screen = Screen::Collections;
}

pub fn load_selected_collection(&mut self) {
    if let Some(name) = self.collections_list.get(self.collections_cursor) {
        if let Ok(coll) = crate::collection::load_collection(name) {
            self.collections_detail = Some(coll);
            self.collections_theme_cursor = 0;
            self.collections_viewing_themes = true;
        }
    }
}
```

**Step 2: Create src/ui/collections.rs**

Render a two-panel layout:
- Left: list of collection names, active marked with *, selected highlighted
- Right: if viewing themes, list themes in selected collection with current_index marked

Include keybind hints at the bottom. Handle all the input modes (normal, new collection, set interval, confirm delete).

**Step 3: Update src/ui/mod.rs**

```rust
mod collections;
pub use collections::render_collections;
```

**Step 4: Add C keybinding and screen routing in main.rs**

In `handle_browse_input`, `InputMode::Normal`:
```rust
KeyCode::Char('C') => {
    app.enter_collections();
}
```

In `run_app` draw match:
```rust
Screen::Collections => ui::render_collections(f, app),
```

Add `handle_collections_input` function with all the keybindings from the design (j/k, Enter/l, n, d, u, s, i, x, h/Esc).

**Step 5: Add `C` hint to browser bottom bar**

In `browser.rs` `render_bottom_bar`, add hint for `C` → "collections".

**Step 6: Verify**

Run: `cargo run` (TUI), press `C`
Expected: Collections screen renders, can navigate and manage collections

**Step 7: Commit**

```bash
git add src/ui/collections.rs src/ui/mod.rs src/app.rs src/main.rs src/ui/browser.rs
git commit -m "feat: add Collections screen to TUI"
```

---

### Task 9: Integration testing and polish

**Files:**
- Modify: various files for edge cases

**Step 1: Test the full CLI workflow end-to-end**

```bash
cargo run -- collection create my-themes
cargo run -- collection list
cargo run -- collection use my-themes
# Add themes via TUI (c keybinding) or CLI
cargo run -- next
cargo run -- cycle status
```

**Step 2: Test the full TUI workflow**

- Browse → press `c` → create collection → add theme
- Press `C` → collections screen → navigate, set interval, toggle order
- Press `u` to set active
- Verify `next` command works with the collection

**Step 3: Handle edge cases**

- Empty collection: `next` should error gracefully
- Missing active collection: clear error message
- Duplicate theme in collection: allow it (user might want repeats)
- Invalid interval format: clear error message
- Daemon already running: clear error message

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: polish and edge case handling for theme cycling"
```

---

### Task 10: Update CLAUDE.md and README

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

**Step 1: Update CLAUDE.md**

Add documentation for new modules (`cli.rs`, `collection.rs`, `cycling.rs`, `daemon.rs`, `shell_hook.rs`, `ui/collections.rs`) and new screen flow.

**Step 2: Update README.md**

Add sections for:
- Collections (creating, managing)
- Theme cycling (next command, daemon, shell hook)
- CLI reference for all new subcommands

**Step 3: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: update CLAUDE.md and README for theme cycling feature"
```
