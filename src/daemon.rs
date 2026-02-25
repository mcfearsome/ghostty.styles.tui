use std::fs;
use std::thread;
use std::time::Duration;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::collection;
use crate::cycling;

/// Parse an interval string like "30m", "1h", "90s" into a `Duration`.
fn parse_interval(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Interval string is empty".to_string());
    }

    let (num_str, suffix) = if s.ends_with('h') {
        (&s[..s.len() - 1], "h")
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], "m")
    } else if s.ends_with('s') {
        (&s[..s.len() - 1], "s")
    } else {
        return Err(format!(
            "Invalid interval '{}': must end with 's', 'm', or 'h'",
            s
        ));
    };

    let value: u64 = num_str
        .parse()
        .map_err(|_| format!("Invalid interval '{}': could not parse number", s))?;

    if value == 0 {
        return Err("Interval must be greater than zero".to_string());
    }

    let secs = match suffix {
        "s" => value,
        "m" => value * 60,
        "h" => value * 3600,
        _ => unreachable!(),
    };

    Ok(Duration::from_secs(secs))
}

/// Check whether a process with the given PID is alive.
fn is_process_alive(pid: i32) -> bool {
    signal::kill(Pid::from_raw(pid), None).is_ok()
}

/// Start the cycling daemon as a foreground process.
pub fn start() -> Result<(), String> {
    let pid_file = collection::pid_path();

    // Check for existing daemon
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

        // Stale PID file, remove it
        let _ = fs::remove_file(&pid_file);
    }

    // Load active collection and verify interval
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

    // Write PID file
    collection::ensure_dirs()?;
    let my_pid = std::process::id();
    fs::write(&pid_file, my_pid.to_string())
        .map_err(|e| format!("Failed to write PID file: {}", e))?;

    println!(
        "Daemon started (PID {}) — collection '{}', interval {}",
        my_pid, coll_name, interval_str
    );

    // Main loop: sleep then cycle
    loop {
        thread::sleep(interval);

        match cycling::apply_next() {
            Ok(msg) => {
                eprintln!("[daemon] {}", msg);
            }
            Err(e) => {
                eprintln!("[daemon] Error cycling theme: {}", e);
            }
        }
    }
}

/// Stop a running daemon by sending SIGTERM.
pub fn stop() -> Result<(), String> {
    let pid_file = collection::pid_path();

    if !pid_file.exists() {
        return Err("No daemon is running (PID file not found)".to_string());
    }

    let contents = fs::read_to_string(&pid_file)
        .map_err(|e| format!("Failed to read PID file: {}", e))?;
    let pid: i32 = contents
        .trim()
        .parse()
        .map_err(|_| "Corrupt PID file".to_string())?;

    if !is_process_alive(pid) {
        let _ = fs::remove_file(&pid_file);
        return Err(format!(
            "Daemon (PID {}) is not running. Removed stale PID file.",
            pid
        ));
    }

    signal::kill(Pid::from_raw(pid), Signal::SIGTERM)
        .map_err(|e| format!("Failed to send SIGTERM to PID {}: {}", pid, e))?;

    let _ = fs::remove_file(&pid_file);
    println!("Stopped daemon (PID {})", pid);

    Ok(())
}

/// Print the current status of the daemon and active collection.
pub fn status() -> Result<(), String> {
    let pid_file = collection::pid_path();

    if pid_file.exists() {
        let contents = fs::read_to_string(&pid_file)
            .map_err(|e| format!("Failed to read PID file: {}", e))?;
        let pid: i32 = contents
            .trim()
            .parse()
            .map_err(|_| "Corrupt PID file".to_string())?;

        if is_process_alive(pid) {
            println!("Daemon: running (PID {})", pid);
        } else {
            println!("Daemon: not running (stale PID file for {})", pid);
        }
    } else {
        println!("Daemon: not running");
    }

    // Print active collection info
    let app_config = collection::load_config();
    match app_config.active_collection {
        Some(name) => {
            match collection::load_collection(&name) {
                Ok(coll) => {
                    let order_str = match coll.order {
                        collection::CycleOrder::Sequential => "sequential",
                        collection::CycleOrder::Shuffle => "shuffle",
                    };
                    let interval_str = coll.interval.as_deref().unwrap_or("not set");
                    let current_theme = if coll.themes.is_empty() {
                        "(none)".to_string()
                    } else {
                        let idx = coll.current_index.min(coll.themes.len() - 1);
                        coll.themes[idx].title.clone()
                    };

                    println!("Collection: {}", name);
                    println!("Themes:     {}", coll.themes.len());
                    println!("Order:      {}", order_str);
                    println!("Interval:   {}", interval_str);
                    println!("Current:    {}", current_theme);
                }
                Err(e) => {
                    println!("Collection: {} (error: {})", name, e);
                }
            }
        }
        None => {
            println!("Collection: (none active)");
        }
    }

    Ok(())
}
