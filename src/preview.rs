use std::io::Write;

use crate::theme::GhosttyConfig;

/// Send OSC sequences to temporarily change terminal colors to match a theme.
pub fn apply_osc_preview(theme: &GhosttyConfig) {
    let mut stdout = std::io::stdout();

    // Set foreground (OSC 10)
    let _ = write!(stdout, "\x1b]10;{}\x07", theme.foreground);

    // Set background (OSC 11)
    let _ = write!(stdout, "\x1b]11;{}\x07", theme.background);

    // Set cursor color (OSC 12)
    if let Some(ref cursor) = theme.cursor_color {
        let _ = write!(stdout, "\x1b]12;{}\x07", cursor);
    }

    // Set palette colors (OSC 4;N;color)
    for (i, color) in theme.palette.iter().enumerate() {
        let _ = write!(stdout, "\x1b]4;{};{}\x07", i, color);
    }

    let _ = stdout.flush();
}

/// Query current terminal colors and save them for later restoration.
/// Returns a snapshot of saved colors as OSC restore sequences.
pub fn save_current_colors() -> SavedColors {
    // We can't reliably query all terminals, so we'll store a "reset" command instead.
    // Most terminals support OSC 104 (reset palette), OSC 110 (reset fg), OSC 111 (reset bg), OSC 112 (reset cursor).
    SavedColors
}

/// Restore terminal colors to their original state.
pub fn restore_colors(_saved: &SavedColors) {
    let mut stdout = std::io::stdout();

    // Reset foreground (OSC 110)
    let _ = write!(stdout, "\x1b]110\x07");
    // Reset background (OSC 111)
    let _ = write!(stdout, "\x1b]111\x07");
    // Reset cursor color (OSC 112)
    let _ = write!(stdout, "\x1b]112\x07");
    // Reset all palette colors (OSC 104)
    let _ = write!(stdout, "\x1b]104\x07");

    let _ = stdout.flush();
}

pub struct SavedColors;
