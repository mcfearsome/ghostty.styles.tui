use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

const ACCENT: Color = Color::Rgb(187, 154, 247);
const DIM: Color = Color::Rgb(100, 100, 120);

pub fn render_create_meta(f: &mut Frame, app: &App) {
    let (creator, meta) = match (&app.creator_state, &app.create_meta_state) {
        (Some(c), Some(m)) => (c, m),
        _ => return,
    };

    let area = f.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    // Top bar
    let title_line = Line::from(vec![
        Span::styled(
            " Save Theme: ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &creator.title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let top_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM));
    f.render_widget(Paragraph::new(title_line).block(top_block), outer[0]);

    // Main content: form fields
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer[1]);

    let form_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" Metadata ", Style::default().fg(ACCENT)));
    let form_inner = form_block.inner(content[0]);
    f.render_widget(form_block, content[0]);

    let mut lines: Vec<Line> = Vec::new();

    // Field 0: Title
    let sel = meta.field_index == 0;
    let marker = if sel { "> " } else { "  " };
    let editing_indicator = if sel && meta.editing {
        " (editing)"
    } else {
        ""
    };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Title: ", Style::default().fg(DIM)),
        Span::styled(&creator.title, Style::default().fg(Color::White)),
        Span::styled(editing_indicator, Style::default().fg(ACCENT)),
    ]));
    lines.push(Line::from(""));

    // Field 1: Description
    let sel = meta.field_index == 1;
    let marker = if sel { "> " } else { "  " };
    let desc_display = if meta.description.is_empty() {
        "(optional)"
    } else {
        &meta.description
    };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Description: ", Style::default().fg(DIM)),
        Span::styled(
            desc_display,
            Style::default().fg(if meta.description.is_empty() {
                DIM
            } else {
                Color::White
            }),
        ),
        if sel && meta.editing {
            Span::styled("_", Style::default().fg(ACCENT))
        } else {
            Span::styled("", Style::default())
        },
    ]));
    lines.push(Line::from(""));

    // Field 2: Tags
    let sel = meta.field_index == 2;
    let marker = if sel { "> " } else { "  " };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Tags: ", Style::default().fg(DIM)),
        Span::styled(
            if meta.tags.is_empty() {
                "(select up to 5)".to_string()
            } else {
                meta.tags.join(", ")
            },
            Style::default().fg(if meta.tags.is_empty() {
                DIM
            } else {
                Color::White
            }),
        ),
    ]));

    // Show tag selector when this field is active and editing
    if sel && meta.editing {
        let upload_tags = [
            "dark",
            "light",
            "minimal",
            "colorful",
            "retro",
            "pastel",
            "high-contrast",
            "monochrome",
            "warm",
            "cool",
            "neon",
        ];
        for (i, tag) in upload_tags.iter().enumerate() {
            let is_selected = meta.tags.contains(&tag.to_string());
            let is_cursor = i == meta.tag_cursor;
            let check = if is_selected { "[x]" } else { "[ ]" };
            let cursor_marker = if is_cursor { " > " } else { "   " };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {}{} ", cursor_marker, check),
                    Style::default().fg(if is_cursor { ACCENT } else { DIM }),
                ),
                Span::styled(
                    *tag,
                    Style::default().fg(if is_cursor { Color::White } else { DIM }),
                ),
            ]));
        }
    }
    lines.push(Line::from(""));

    // Field 3: Author name
    let sel = meta.field_index == 3;
    let marker = if sel { "> " } else { "  " };
    let author_display = if meta.author_name.is_empty() {
        "(optional)"
    } else {
        &meta.author_name
    };
    lines.push(Line::from(vec![
        Span::styled(marker, field_style(sel)),
        Span::styled("Author: ", Style::default().fg(DIM)),
        Span::styled(
            author_display,
            Style::default().fg(if meta.author_name.is_empty() {
                DIM
            } else {
                Color::White
            }),
        ),
        if sel && meta.editing {
            Span::styled("_", Style::default().fg(ACCENT))
        } else {
            Span::styled("", Style::default())
        },
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Action buttons
    let sel = meta.field_index == 4;
    lines.push(Line::from(vec![
        Span::styled(if sel { "> " } else { "  " }, field_style(sel)),
        Span::styled(" a ", Style::default().fg(Color::Black).bg(ACCENT)),
        Span::styled(" Apply  ", Style::default().fg(DIM)),
        Span::styled(" e ", Style::default().fg(Color::Black).bg(Color::Green)),
        Span::styled(" Export  ", Style::default().fg(DIM)),
        Span::styled(" u ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::styled(" Upload ", Style::default().fg(DIM)),
    ]));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), form_inner);

    // Right panel: preview
    let preview_config = creator.build_preview_config();
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(" Preview ", Style::default().fg(ACCENT)));
    let preview_inner = preview_block.inner(content[1]);
    f.render_widget(preview_block, content[1]);
    f.render_widget(
        crate::ui::preview::ThemePreview {
            theme: &preview_config,
        },
        preview_inner,
    );

    // Bottom bar
    let hints = if meta.editing && meta.field_index == 2 {
        vec![("j/k", "nav tags"), ("Space", "toggle"), ("Esc", "done")]
    } else if meta.editing {
        vec![("type", "edit"), ("Esc", "done")]
    } else {
        vec![
            ("j/k", "nav"),
            ("Enter", "edit"),
            ("a", "apply"),
            ("e", "export"),
            ("u", "upload"),
            ("Esc", "back"),
        ]
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(*key, Style::default().fg(ACCENT)));
        spans.push(Span::styled(
            format!(":{}", action),
            Style::default().fg(DIM),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), outer[2]);
}

fn field_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    }
}
