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
        if theme.to_lowercase().contains("dark") {
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
                return;
            }
        }
        #[cfg(target_os = "linux")]
        {
            if watch_linux(&tx) {
                return;
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

/// macOS: watch for theme changes via a Swift helper that listens to
/// DistributedNotificationCenter. Returns true if watcher set up (blocks forever).
#[cfg(target_os = "macos")]
fn watch_macos(tx: &mpsc::Sender<bool>) -> bool {
    use std::io::BufRead;

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
            if let Some(is_dark) = detect_current() {
                let _ = tx.send(is_dark);
            }
        }
    }

    true
}

/// Linux: watch via `gsettings monitor`. Returns true if watcher set up (blocks forever).
#[cfg(target_os = "linux")]
fn watch_linux(tx: &mpsc::Sender<bool>) -> bool {
    use std::io::BufRead;

    let mut child = match Command::new("gsettings")
        .args(["monitor", "org.gnome.desktop.interface", "color-scheme"])
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

/// Resolve the desired mode from the given preference.
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

/// Determine whether it's "dark time" based on current local time.
fn resolve_time(dark_after: &str, light_after: &str) -> Option<bool> {
    let now = local_minutes_now();
    let dark_mins = parse_hhmm(dark_after)?;
    let light_mins = parse_hhmm(light_after)?;

    if light_mins < dark_mins {
        // Normal: light=07:00, dark=19:00
        // Light period: light_after..dark_after
        Some(now < light_mins || now >= dark_mins)
    } else {
        // Inverted: dark=01:00, light=09:00
        Some(now >= dark_mins && now < light_mins)
    }
}

/// Get current local time as minutes since midnight.
fn local_minutes_now() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    unsafe {
        let t = secs as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        (tm.tm_hour as u32) * 60 + (tm.tm_min as u32)
    }
}

/// Parse "HH:MM" into minutes since midnight.
pub fn parse_hhmm(s: &str) -> Option<u32> {
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
    let now = local_minutes_now();
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
    fn detect_current_returns_option() {
        let _ = detect_current();
    }

    #[test]
    fn seconds_until_boundary_returns_some() {
        let result = seconds_until_boundary("19:00", "07:00");
        assert!(result.is_some());
        assert!(result.unwrap() > 0);
    }
}
