use std::process::Command;

pub fn reload_shortcut_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd+Shift+,"
    } else {
        "Super+Shift+,"
    }
}

/// Best-effort reload of Ghostty config for the currently focused app.
///
/// On macOS, this sends the default reload keybind to the frontmost app.
/// On other platforms we currently return an error and rely on manual reload.
pub fn try_reload_config() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/osascript")
            .args([
                "-e",
                r#"tell application "System Events" to keystroke "," using {command down, shift down}"#,
            ])
            .output()
            .map_err(|e| format!("failed to run osascript: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                Err("reload command failed".to_string())
            } else {
                Err(stderr)
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("automatic reload is not supported on this platform".to_string())
    }
}
