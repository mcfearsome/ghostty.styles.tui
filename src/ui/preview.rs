use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::GhosttyConfig;

/// A widget that renders a color preview of a Ghostty theme.
pub struct ThemePreview<'a> {
    pub theme: &'a GhosttyConfig,
}

impl<'a> Widget for ThemePreview<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let bg = theme.bg_color();
        let fg = theme.fg_color();

        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_style(Style::default().bg(bg));
            }
        }

        let mut y = area.y;

        // Title
        if y < area.y + area.height {
            let title = format!(" {} ", theme.title);
            let line = Line::from(vec![Span::styled(
                &title,
                Style::default()
                    .fg(fg)
                    .bg(bg)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )]);
            buf.set_line(area.x + 1, y, &line, area.width.saturating_sub(2));
            y += 1;
        }

        // Author
        if y < area.y + area.height {
            if let Some(ref author) = theme.author_name {
                let line = Line::from(vec![Span::styled(
                    format!(" by {} ", author),
                    Style::default().fg(fg).bg(bg),
                )]);
                buf.set_line(area.x + 1, y, &line, area.width.saturating_sub(2));
            }
            y += 1;
        }

        // Separator
        if y < area.y + area.height {
            y += 1;
        }

        // Color palette - normal colors (0-7)
        if y < area.y + area.height {
            let mut spans = vec![Span::styled(" ", Style::default().bg(bg))];
            for i in 0..8 {
                let c = theme.palette_color(i);
                spans.push(Span::styled("  ", Style::default().bg(c)));
                spans.push(Span::styled(" ", Style::default().bg(bg)));
            }
            let line = Line::from(spans);
            buf.set_line(area.x, y, &line, area.width);
            y += 1;
        }

        // Color palette - bright colors (8-15)
        if y < area.y + area.height {
            let mut spans = vec![Span::styled(" ", Style::default().bg(bg))];
            for i in 8..16 {
                let c = theme.palette_color(i);
                spans.push(Span::styled("  ", Style::default().bg(c)));
                spans.push(Span::styled(" ", Style::default().bg(bg)));
            }
            let line = Line::from(spans);
            buf.set_line(area.x, y, &line, area.width);
            y += 1;
        }

        // Separator
        if y < area.y + area.height {
            y += 1;
        }

        // Sample terminal output
        let samples: Vec<(&str, usize)> = vec![
            ("$ ls -la", 2),          // green
            ("README.md", 4),         // blue
            ("Cargo.toml", 3),        // yellow
            ("src/", 6),              // cyan
            ("$ git status", 2),      // green
            ("modified: main.rs", 1), // red
            ("$ cargo build", 5),     // magenta
            ("Compiling...", 3),      // yellow
            ("Finished OK", 2),       // green
        ];

        for (text, color_idx) in &samples {
            if y >= area.y + area.height {
                break;
            }
            let prompt_color = theme.palette_color(*color_idx);
            let line = Line::from(vec![
                Span::styled(" ", Style::default().bg(bg)),
                Span::styled(*text, Style::default().fg(prompt_color).bg(bg)),
            ]);
            buf.set_line(area.x, y, &line, area.width);
            y += 1;
        }

        // Separator
        if y < area.y + area.height {
            y += 1;
        }

        // Color info
        let color_infos: Vec<(&str, Color)> = vec![("BG", bg), ("FG", fg)];
        if y < area.y + area.height {
            let mut spans = vec![Span::styled(" ", Style::default().bg(bg))];
            for (label, color) in &color_infos {
                spans.push(Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(fg).bg(bg),
                ));
                spans.push(Span::styled("  ", Style::default().bg(*color)));
                spans.push(Span::styled(" ", Style::default().bg(bg)));
            }
            let line = Line::from(spans);
            buf.set_line(area.x, y, &line, area.width);
        }
    }
}
