use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode, AVAILABLE_TAGS};
use crate::ui::preview::ThemePreview;

const ACCENT: Color = Color::Rgb(187, 154, 247); // Purple accent
const DIM: Color = Color::Rgb(100, 100, 120);
const TAG_BG: Color = Color::Rgb(50, 50, 70);

pub fn render_browser(f: &mut Frame, app: &App) {
    let size = f.area();

    // Layout: [top bar] [main area] [bottom bar]
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // top bar
            Constraint::Min(5),   // main
            Constraint::Length(1), // status/bottom bar
        ])
        .split(size);

    render_top_bar(f, app, outer[0]);
    render_main(f, app, outer[1]);
    render_bottom_bar(f, app, outer[2]);

    // Tag selector overlay
    if app.input_mode == InputMode::TagSelect {
        render_tag_popup(f, app, size);
    }

    // Collection popup overlay
    if app.input_mode == InputMode::CollectionSelect || app.input_mode == InputMode::CollectionCreate {
        render_collection_popup(f, app, size);
    }
}

fn render_top_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // title
            Constraint::Min(20),   // search
            Constraint::Length(30), // filters info
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" ghostty", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(".styles", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Search bar
    let search_style = if app.input_mode == InputMode::Search {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(DIM)
    };
    let search_text = if app.input_mode == InputMode::Search {
        format!(" / {}_", app.search_input)
    } else if let Some(ref q) = app.active_query {
        format!(" / {} ", q)
    } else {
        " / search...".to_string()
    };
    let search = Paragraph::new(Span::styled(search_text, search_style))
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(search, chunks[1]);

    // Filter info
    let mut filter_spans = Vec::new();
    filter_spans.push(Span::styled(
        format!(" {} ", app.sort.label()),
        Style::default().fg(ACCENT),
    ));
    if let Some(ref tag) = app.active_tag {
        filter_spans.push(Span::styled(
            format!("[{}] ", tag),
            Style::default().fg(Color::Rgb(130, 200, 130)),
        ));
    }
    match app.dark_filter {
        Some(true) => filter_spans.push(Span::styled("dark ", Style::default().fg(DIM))),
        Some(false) => filter_spans.push(Span::styled("light ", Style::default().fg(DIM))),
        None => {}
    }
    filter_spans.push(Span::styled(
        format!("p{}/{} ", app.page, app.total_pages.max(1)),
        Style::default().fg(DIM),
    ));
    let filters = Paragraph::new(Line::from(filter_spans))
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(filters, chunks[2]);
}

fn render_main(f: &mut Frame, app: &App, area: Rect) {
    if app.loading {
        let loading = Paragraph::new(Span::styled(
            "  Loading themes...",
            Style::default().fg(ACCENT),
        ));
        f.render_widget(loading, area);
        return;
    }

    if let Some(ref err) = app.error {
        let error = Paragraph::new(vec![
            Line::from(Span::styled(
                "  Error loading themes",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}", err),
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press 'r' to retry",
                Style::default().fg(DIM),
            )),
        ]);
        f.render_widget(error, area);
        return;
    }

    if app.themes.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "  No themes found. Try a different search or filter.",
            Style::default().fg(DIM),
        ));
        f.render_widget(empty, area);
        return;
    }

    // Split: theme list | preview
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(area);

    render_theme_list(f, app, chunks[0]);
    render_preview_panel(f, app, chunks[1]);
}

fn render_theme_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .themes
        .iter()
        .enumerate()
        .map(|(i, theme)| {
            let is_selected = i == app.selected;
            let indicator = if is_selected { ">" } else { " " };

            let mut spans = vec![
                Span::styled(
                    format!("{} ", indicator),
                    Style::default().fg(if is_selected { ACCENT } else { DIM }),
                ),
                Span::styled(
                    truncate(&theme.title, 28),
                    Style::default()
                        .fg(if is_selected { Color::White } else { Color::Gray })
                        .add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ];

            // Vote count
            spans.push(Span::styled(
                format!(" {} ", vote_icon(theme.vote_count)),
                Style::default().fg(DIM),
            ));

            // Tags (first 2)
            for tag in theme.tags.iter().take(2) {
                spans.push(Span::styled(
                    format!(" {} ", tag),
                    Style::default().fg(Color::Rgb(140, 140, 160)).bg(TAG_BG),
                ));
                spans.push(Span::raw(" "));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default())
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
                .title(Span::styled(
                    format!(" Themes ({}) ", app.total_results),
                    Style::default().fg(ACCENT),
                )),
        );

    let mut state = ListState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_preview_panel(f: &mut Frame, app: &App, area: Rect) {
    if let Some(theme) = app.selected_theme() {
        let block = Block::default()
            .title(Span::styled(" Preview ", Style::default().fg(ACCENT)))
            .borders(Borders::NONE);
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(ThemePreview { theme }, inner);
    } else {
        let placeholder = Paragraph::new(Span::styled(
            "Select a theme to preview",
            Style::default().fg(DIM),
        ));
        f.render_widget(placeholder, area);
    }
}

fn render_bottom_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![];

    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" {} ", msg),
            Style::default().fg(Color::Rgb(130, 200, 130)),
        ));
    } else {
        let osc_indicator = if app.osc_preview_active {
            Span::styled(" [LIVE] ", Style::default().fg(Color::Rgb(255, 150, 50)).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("")
        };
        spans.push(osc_indicator);

        let hints = vec![
            ("j/k", "nav"),
            ("Enter", "detail"),
            ("/", "search"),
            ("t", "tags"),
            ("s", "sort"),
            ("d", "dark/light"),
            ("p", "preview"),
            ("a", "apply"),
            ("c", "collect"),
            ("C", "collections"),
            ("]/[", "page"),
            ("n", "new"),
            ("q", "quit"),
        ];
        for (key, desc) in hints {
            spans.push(Span::styled(
                format!(" {} ", key),
                Style::default().fg(ACCENT),
            ));
            spans.push(Span::styled(
                format!("{} ", desc),
                Style::default().fg(DIM),
            ));
        }
    }

    let bar = Paragraph::new(Line::from(spans));
    f.render_widget(bar, area);
}

fn render_tag_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 30u16;
    let popup_height = (AVAILABLE_TAGS.len() as u16 + 2).min(area.height);
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = AVAILABLE_TAGS
        .iter()
        .enumerate()
        .map(|(i, tag)| {
            let is_cursor = i == app.tag_cursor;
            let is_active = app.active_tag.as_deref() == Some(tag);
            let marker = if is_active { "[x]" } else { "[ ]" };
            let style = if is_cursor {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(Color::Rgb(130, 200, 130))
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Span::styled(format!(" {} {} ", marker, tag), style))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(" Filter by Tag ", Style::default().fg(ACCENT)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(list, popup_area);
}

fn render_collection_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 40u16;

    if app.input_mode == InputMode::CollectionCreate {
        let popup_height = 5u16;
        let x = area.width.saturating_sub(popup_width) / 2;
        let y = area.height.saturating_sub(popup_height) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        f.render_widget(Clear, popup_area);

        let lines = vec![
            Line::from(Span::styled(
                format!(" > {}_ ", app.collection_name_input),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Enter confirm  Esc cancel",
                Style::default().fg(DIM),
            )),
        ];
        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(Span::styled(" New Collection ", Style::default().fg(ACCENT)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT)),
        );
        f.render_widget(paragraph, popup_area);
    } else {
        // CollectionSelect
        let popup_height = (app.collection_names.len() as u16 + 4).min(area.height);
        let x = area.width.saturating_sub(popup_width) / 2;
        let y = area.height.saturating_sub(popup_height) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        f.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = app
            .collection_names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let is_cursor = i == app.collection_popup_cursor;
                let indicator = if is_cursor { ">" } else { " " };
                let style = if is_cursor {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                ListItem::new(Span::styled(format!(" {} {} ", indicator, name), style))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(" Add to Collection ", Style::default().fg(ACCENT)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT)),
        );
        f.render_widget(list, popup_area);

        // Render hint at bottom of popup
        let hint_y = popup_area.y + popup_area.height.saturating_sub(1);
        if hint_y < area.height {
            let hint_area = Rect::new(popup_area.x + 1, hint_y, popup_area.width.saturating_sub(2), 1);
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("n", Style::default().fg(ACCENT)),
                Span::styled(" new  ", Style::default().fg(DIM)),
                Span::styled("Enter", Style::default().fg(ACCENT)),
                Span::styled(" select  ", Style::default().fg(DIM)),
                Span::styled("Esc", Style::default().fg(ACCENT)),
                Span::styled(" cancel", Style::default().fg(DIM)),
            ]));
            f.render_widget(hint, hint_area);
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

fn vote_icon(count: i32) -> String {
    if count > 0 {
        format!("{}{}", '\u{2665}', count) // heart + count
    } else {
        String::new()
    }
}
