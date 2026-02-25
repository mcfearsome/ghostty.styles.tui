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

    print!(
        "Install shell hook in {} ({})? [y/N] ",
        rc_path.display(),
        shell_name
    );
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_ok() && input.trim().eq_ignore_ascii_case("y") {
        match install(&rc_path) {
            Ok(_) => {
                println!(
                    "Hook installed. Restart your shell or run: source {}",
                    rc_path.display()
                );
                return true;
            }
            Err(e) => {
                eprintln!("Failed to install hook: {}", e);
            }
        }
    }
    false
}
