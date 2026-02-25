use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Screen};

const ACCENT: Color = Color::Rgb(187, 154, 247);
const DIM: Color = Color::Rgb(120, 120, 140);

pub fn render_help(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = area.width.saturating_sub(6).clamp(40, 96);
    let height = area.height.saturating_sub(4).clamp(12, 30);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                "Commands",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  (current: {})", screen_label(&app.screen)),
                Style::default().fg(DIM),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Global", Style::default().fg(ACCENT))),
        Line::from("  ?: toggle help  |  Ctrl+C: quit"),
        Line::from(""),
        Line::from(Span::styled("Browse", Style::default().fg(ACCENT))),
        Line::from("  j/k or arrows: navigate  |  Enter/l: details"),
        Line::from("  /: search  |  t: tags  |  s: sort  |  d: dark/light"),
        Line::from("  m: mode  |  p: live preview  |  a: apply"),
        Line::from("  c: add to collection  |  C: collections"),
        Line::from("  n: new theme  |  [ ]: page  |  r: refresh  |  q/Esc: quit"),
        Line::from(""),
        Line::from(Span::styled("Detail", Style::default().fg(ACCENT))),
        Line::from("  h/Left/Esc: back  |  p: preview  |  a: apply  |  c: collect  |  f: fork"),
        Line::from(""),
        Line::from(Span::styled("Collections", Style::default().fg(ACCENT))),
        Line::from("  list: j/k nav, Enter view, n new, d delete, u activate, s order, i interval"),
        Line::from("  themes: j/k nav, x remove, Esc back"),
        Line::from(""),
        Line::from(Span::styled("Creator", Style::default().fg(ACCENT))),
        Line::from("  j/k nav fields, Enter edit, g generate, p preview, s save, Esc back"),
        Line::from("  editing: Left/Right adjust, Shift+Left/Right x10, Up/Down focus, Tab mode"),
        Line::from(""),
        Line::from(Span::styled("Save Metadata", Style::default().fg(ACCENT))),
        Line::from("  j/k nav, Enter edit, a apply, e export, u upload, Esc back"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(DIM),
        )),
    ];

    let max_body_lines = height.saturating_sub(2) as usize;
    if lines.len() > max_body_lines {
        lines.truncate(max_body_lines);
    }

    let block = Block::default()
        .title(Span::styled(" Help ", Style::default().fg(ACCENT)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(block);
    f.render_widget(paragraph, popup);
}

fn screen_label(screen: &Screen) -> &'static str {
    match screen {
        Screen::Browse => "browse",
        Screen::Detail => "detail",
        Screen::Confirm => "confirm",
        Screen::Collections => "collections",
        Screen::Create => "creator",
        Screen::CreateMeta => "save-meta",
    }
}
