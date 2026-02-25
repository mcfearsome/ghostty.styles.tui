use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, CollectionsMode};
use crate::collection;

const ACCENT: Color = Color::Rgb(187, 154, 247);
const DIM: Color = Color::Rgb(100, 100, 120);

pub fn render_collections(f: &mut Frame, app: &App) {
    let area = f.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // top bar
            Constraint::Min(5),   // main
            Constraint::Length(1), // bottom bar
        ])
        .split(area);

    render_top_bar(f, outer[0]);
    render_main(f, app, outer[1]);
    render_bottom_bar(f, app, outer[2]);

    // Overlay popups for modal modes
    match app.collections_mode {
        CollectionsMode::NewCollection => render_new_collection_popup(f, app, area),
        CollectionsMode::SetInterval => render_set_interval_popup(f, app, area),
        CollectionsMode::ConfirmDelete => render_confirm_delete_popup(f, app, area),
        CollectionsMode::Normal => {}
    }
}

fn render_top_bar(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " ghostty",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ".styles",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " / Collections",
            Style::default().fg(DIM),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, area);
}

fn render_main(f: &mut Frame, app: &App, area: Rect) {
    if app.collections_list.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No collections yet.",
                Style::default().fg(DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press 'n' to create one.",
                Style::default().fg(DIM),
            )),
        ]);
        f.render_widget(empty, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(area);

    render_collection_list(f, app, chunks[0]);
    render_theme_panel(f, app, chunks[1]);
}

fn render_collection_list(f: &mut Frame, app: &App, area: Rect) {
    let config = collection::load_config();
    let active_name = config.active_collection.as_deref();

    let items: Vec<ListItem> = app
        .collections_list
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == app.collections_cursor;
            let is_active = active_name == Some(name.as_str());
            let indicator = if is_selected { ">" } else { " " };
            let active_marker = if is_active { " *" } else { "" };

            let theme_count = collection::load_collection(name)
                .map(|c| c.themes.len())
                .unwrap_or(0);

            let spans = vec![
                Span::styled(
                    format!("{} ", indicator),
                    Style::default().fg(if is_selected { ACCENT } else { DIM }),
                ),
                Span::styled(
                    name.clone(),
                    Style::default()
                        .fg(if is_selected {
                            Color::White
                        } else {
                            Color::Gray
                        })
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(
                    active_marker.to_string(),
                    Style::default()
                        .fg(Color::Rgb(130, 200, 130))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ({} themes)", theme_count),
                    Style::default().fg(DIM),
                ),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
            .title(Span::styled(
                format!(" Collections ({}) ", app.collections_list.len()),
                Style::default().fg(ACCENT),
            )),
    );
    f.render_widget(list, area);
}

fn render_theme_panel(f: &mut Frame, app: &App, area: Rect) {
    if !app.collections_viewing_themes {
        let hint = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Select a collection and press Enter",
                Style::default().fg(DIM),
            )),
            Line::from(Span::styled(
                "  to view its themes.",
                Style::default().fg(DIM),
            )),
        ])
        .block(
            Block::default()
                .title(Span::styled(" Themes ", Style::default().fg(ACCENT)))
                .borders(Borders::NONE),
        );
        f.render_widget(hint, area);
        return;
    }

    let coll = match &app.collections_detail {
        Some(c) => c,
        None => return,
    };

    if coll.themes.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No themes in this collection.",
                Style::default().fg(DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Add themes from the Browse screen with 'c'.",
                Style::default().fg(DIM),
            )),
        ])
        .block(
            Block::default()
                .title(Span::styled(
                    format!(" {} ", coll.name),
                    Style::default().fg(ACCENT),
                ))
                .borders(Borders::NONE),
        );
        f.render_widget(empty, area);
        return;
    }

    // Collection info line + theme list
    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // info
            Constraint::Min(3),   // theme list
        ])
        .split(area);

    // Info line: order, interval
    let order_str = match coll.order {
        collection::CycleOrder::Sequential => "sequential",
        collection::CycleOrder::Shuffle => "shuffle",
    };
    let interval_str = coll
        .interval
        .as_deref()
        .unwrap_or("not set");
    let info = Paragraph::new(Line::from(vec![
        Span::styled("  Order: ", Style::default().fg(DIM)),
        Span::styled(order_str, Style::default().fg(Color::White)),
        Span::styled("  Interval: ", Style::default().fg(DIM)),
        Span::styled(interval_str, Style::default().fg(Color::White)),
    ]));
    f.render_widget(info, inner_layout[0]);

    // Theme list
    let items: Vec<ListItem> = coll
        .themes
        .iter()
        .enumerate()
        .map(|(i, theme)| {
            let is_selected = i == app.collections_theme_cursor;
            let is_current = i == coll.current_index;
            let indicator = if is_selected { ">" } else { " " };
            let current_marker = if is_current { " <-" } else { "" };
            let mode_indicator = if theme.is_dark { " [dark]" } else { " [light]" };

            let spans = vec![
                Span::styled(
                    format!("  {} ", indicator),
                    Style::default().fg(if is_selected { ACCENT } else { DIM }),
                ),
                Span::styled(
                    theme.title.clone(),
                    Style::default()
                        .fg(if is_selected {
                            Color::White
                        } else {
                            Color::Gray
                        })
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(mode_indicator, Style::default().fg(DIM)),
                Span::styled(
                    current_marker.to_string(),
                    Style::default().fg(Color::Rgb(130, 200, 130)),
                ),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                format!(" {} ({} themes) ", coll.name, coll.themes.len()),
                Style::default().fg(ACCENT),
            ))
            .borders(Borders::NONE),
    );
    f.render_widget(list, inner_layout[1]);
}

fn render_bottom_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![];

    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(
            format!(" {} ", msg),
            Style::default().fg(Color::Rgb(130, 200, 130)),
        ));
    } else {
        let hints: Vec<(&str, &str)> = match app.collections_mode {
            CollectionsMode::Normal if app.collections_viewing_themes => {
                vec![
                    ("j/k", "nav"),
                    ("x", "remove"),
                    ("Esc", "back"),
                ]
            }
            CollectionsMode::Normal => {
                vec![
                    ("j/k", "nav"),
                    ("Enter", "view"),
                    ("n", "new"),
                    ("d", "delete"),
                    ("u", "activate"),
                    ("s", "order"),
                    ("i", "interval"),
                    ("Esc", "back"),
                ]
            }
            CollectionsMode::NewCollection => {
                vec![
                    ("type", "name"),
                    ("Enter", "confirm"),
                    ("Esc", "cancel"),
                ]
            }
            CollectionsMode::SetInterval => {
                vec![
                    ("type", "interval"),
                    ("Enter", "confirm"),
                    ("Esc", "cancel"),
                ]
            }
            CollectionsMode::ConfirmDelete => {
                vec![
                    ("y", "confirm"),
                    ("n/Esc", "cancel"),
                ]
            }
        };

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

fn render_new_collection_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 40u16;
    let popup_height = 5u16;
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            format!(" > {}_ ", app.collections_input),
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
            .title(Span::styled(
                " New Collection ",
                Style::default().fg(ACCENT),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(paragraph, popup_area);
}

fn render_set_interval_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 40u16;
    let popup_height = 6u16;
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            " e.g. 30m, 1h, 2h30m",
            Style::default().fg(DIM),
        )),
        Line::from(Span::styled(
            format!(" > {}_ ", app.collections_input),
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
            .title(Span::styled(
                " Set Interval ",
                Style::default().fg(ACCENT),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT)),
    );
    f.render_widget(paragraph, popup_area);
}

fn render_confirm_delete_popup(f: &mut Frame, app: &App, area: Rect) {
    let name = app
        .collections_list
        .get(app.collections_cursor)
        .cloned()
        .unwrap_or_default();

    let popup_width = 40u16;
    let popup_height = 5u16;
    let x = area.width.saturating_sub(popup_width) / 2;
    let y = area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            format!(" Delete '{}'?", name),
            Style::default()
                .fg(Color::Rgb(255, 200, 50))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" y", Style::default().fg(ACCENT)),
            Span::styled(" confirm  ", Style::default().fg(DIM)),
            Span::styled("n/Esc", Style::default().fg(ACCENT)),
            Span::styled(" cancel", Style::default().fg(DIM)),
        ]),
    ];
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(
                " Confirm Delete ",
                Style::default().fg(Color::Rgb(255, 100, 100)),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(255, 100, 100))),
    );
    f.render_widget(paragraph, popup_area);
}
