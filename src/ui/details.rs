use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::ui::preview::ThemePreview;

const ACCENT: Color = Color::Rgb(187, 154, 247);
const DIM: Color = Color::Rgb(100, 100, 120);

pub fn render_detail(f: &mut Frame, app: &App) {
    let theme = match app.selected_theme() {
        Some(t) => t,
        None => return,
    };

    let area = f.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(10),   // main content
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" < ", Style::default().fg(ACCENT)),
        Span::styled(
            &theme.title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            theme
                .author_name
                .as_deref()
                .map(|a| format!("  by {}", a))
                .unwrap_or_default(),
            Style::default().fg(DIM),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, outer[0]);

    // Main content: left info + right preview
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[1]);

    // Left: theme info + raw config
    render_info_panel(f, app, main[0]);

    // Right: color preview
    let preview_block = Block::default()
        .title(Span::styled(" Preview ", Style::default().fg(ACCENT)))
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)));
    let preview_inner = preview_block.inner(main[1]);
    f.render_widget(preview_block, main[1]);
    f.render_widget(ThemePreview { theme }, preview_inner);

    // Footer
    let footer_spans = if app.screen == Screen::Confirm {
        vec![
            Span::styled(
                " Apply this theme? ",
                Style::default()
                    .fg(Color::Rgb(255, 200, 50))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("y", Style::default().fg(ACCENT)),
            Span::styled("/", Style::default().fg(DIM)),
            Span::styled("n", Style::default().fg(ACCENT)),
        ]
    } else {
        vec![
            Span::styled(" Esc", Style::default().fg(ACCENT)),
            Span::styled(" back  ", Style::default().fg(DIM)),
            Span::styled("p", Style::default().fg(ACCENT)),
            Span::styled(" preview  ", Style::default().fg(DIM)),
            Span::styled("a", Style::default().fg(ACCENT)),
            Span::styled(" apply  ", Style::default().fg(DIM)),
            Span::styled("c", Style::default().fg(ACCENT)),
            Span::styled(" collect  ", Style::default().fg(DIM)),
            Span::styled("f", Style::default().fg(ACCENT)),
            Span::styled(" fork  ", Style::default().fg(DIM)),
        ]
    };
    let footer = Paragraph::new(Line::from(footer_spans));
    f.render_widget(footer, outer[2]);
}

fn render_info_panel(f: &mut Frame, app: &App, area: Rect) {
    let theme = match app.selected_theme() {
        Some(t) => t,
        None => return,
    };

    let mut lines = Vec::new();

    // Description
    if let Some(ref desc) = theme.description {
        lines.push(Line::from(Span::styled(
            format!(" {}", desc),
            Style::default().fg(Color::Gray),
        )));
        lines.push(Line::from(""));
    }

    // Tags
    if !theme.tags.is_empty() {
        let mut spans = vec![Span::styled(" Tags: ", Style::default().fg(DIM))];
        for tag in &theme.tags {
            spans.push(Span::styled(
                format!(" {} ", tag),
                Style::default()
                    .fg(Color::Rgb(140, 140, 160))
                    .bg(Color::Rgb(50, 50, 70)),
            ));
            spans.push(Span::raw(" "));
        }
        lines.push(Line::from(spans));
        lines.push(Line::from(""));
    }

    // Stats
    lines.push(Line::from(vec![
        Span::styled(" Votes: ", Style::default().fg(DIM)),
        Span::styled(
            format!("{}", theme.vote_count),
            Style::default().fg(Color::White),
        ),
        Span::styled("  Views: ", Style::default().fg(DIM)),
        Span::styled(
            format!("{}", theme.view_count),
            Style::default().fg(Color::White),
        ),
        Span::styled("  Downloads: ", Style::default().fg(DIM)),
        Span::styled(
            format!("{}", theme.download_count),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(""));

    // Dark/light
    lines.push(Line::from(vec![
        Span::styled(" Mode: ", Style::default().fg(DIM)),
        Span::styled(
            if theme.is_dark { "Dark" } else { "Light" },
            Style::default().fg(Color::White),
        ),
    ]));

    // Font
    if let Some(ref font) = theme.font_family {
        lines.push(Line::from(vec![
            Span::styled(" Font: ", Style::default().fg(DIM)),
            Span::styled(font.as_str(), Style::default().fg(Color::White)),
        ]));
    }
    lines.push(Line::from(""));

    // Raw config header
    lines.push(Line::from(Span::styled(
        " Raw Config:",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─────────────────────────────",
        Style::default().fg(Color::Rgb(60, 60, 80)),
    )));

    // Raw config lines
    for line in theme.raw_config.lines() {
        let styled = if line.starts_with('#') {
            Span::styled(format!(" {}", line), Style::default().fg(DIM))
        } else if line.contains('=') {
            // Won't render as separate spans in a single Span, so just color the whole line
            Span::styled(
                format!(" {}", line),
                Style::default().fg(Color::Rgb(180, 200, 220)),
            )
        } else {
            Span::styled(format!(" {}", line), Style::default().fg(Color::Gray))
        };
        lines.push(Line::from(styled));
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, area);
}
