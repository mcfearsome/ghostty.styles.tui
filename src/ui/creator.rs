use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::creator::{ColorField, HslColor, PickerMode, SliderFocus};
use crate::ui::preview::ThemePreview;

const ACCENT: Color = Color::Rgb(187, 154, 247);
const DIM: Color = Color::Rgb(100, 100, 120);

/// Layout rectangles for mouse hit testing.
pub struct CreatorLayout {
    pub fields_inner: Rect,
    pub picker_inner: Rect,
}

/// Compute the layout rectangles for the creator screen, for mouse hit testing.
pub fn get_layout_rects(area: Rect) -> CreatorLayout {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(outer[1]);

    let fields_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)));
    let fields_inner = fields_block.inner(columns[0]);

    let picker_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)));
    let picker_inner = picker_block.inner(columns[1]);

    CreatorLayout {
        fields_inner,
        picker_inner,
    }
}

pub fn render_creator(f: &mut Frame, app: &App) {
    let state = match app.creator_state.as_ref() {
        Some(s) => s,
        None => return,
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

    render_top_bar(f, state, outer[0]);
    render_main_content(f, state, outer[1]);
    render_bottom_bar(f, state, outer[2]);
}

fn render_top_bar(f: &mut Frame, state: &crate::creator::CreatorState, area: Rect) {
    let mut title_spans = vec![
        Span::styled(" Create Theme: ", Style::default().fg(DIM)),
        Span::styled(
            &state.title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if state.osc_preview {
        title_spans.push(Span::styled(
            "  [OSC LIVE]",
            Style::default()
                .fg(Color::Rgb(255, 150, 50))
                .add_modifier(Modifier::BOLD),
        ));
    }

    let title =
        Paragraph::new(Line::from(title_spans)).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, area);
}

fn render_main_content(f: &mut Frame, state: &crate::creator::CreatorState, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ])
        .split(area);

    render_field_list(f, state, columns[0]);
    render_hsl_picker(f, state, columns[1]);
    render_preview_panel(f, state, columns[2]);
}

fn render_field_list(f: &mut Frame, state: &crate::creator::CreatorState, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
        .title(Span::styled(" Colors ", Style::default().fg(ACCENT)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let fields = ColorField::all();
    let visible_height = inner.height as usize;

    // Reserve one line at the bottom for the algorithm indicator.
    let list_height = visible_height.saturating_sub(1);

    let mut lines: Vec<Line> = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        if i < state.field_scroll {
            continue;
        }
        if lines.len() >= list_height {
            break;
        }

        let is_selected = i == state.field_index;
        let color = &state.colors[i];
        let indicator = if is_selected { ">" } else { " " };

        let swatch_color = color.to_ratatui_color();
        let hex = color.to_hex();
        let label = field.label();

        // Truncate label to fit: "> XX label  #aabbcc"
        // Available width = inner.width
        // Format: "{indicator} {swatch} {label}  {hex}"
        let max_label_len = (inner.width as usize).saturating_sub(2 + 2 + 2 + 8); // indicator + swatch + gap + hex

        let display_label = if label.len() > max_label_len {
            &label[..max_label_len]
        } else {
            &label
        };

        let label_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let indicator_style = if is_selected {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", indicator), indicator_style),
            Span::styled("  ", Style::default().bg(swatch_color)),
            Span::styled(format!(" {}", display_label), label_style),
            Span::styled(format!(" {}", hex), Style::default().fg(DIM)),
        ]));
    }

    // Algorithm indicator at the bottom.
    let algo_line = Line::from(vec![
        Span::styled(" gen: ", Style::default().fg(DIM)),
        Span::styled(state.gen_algorithm.label(), Style::default().fg(ACCENT)),
    ]);

    // Render field lines
    let field_area = Rect::new(inner.x, inner.y, inner.width, list_height as u16);
    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, field_area);

    // Render algorithm indicator
    if visible_height > 0 {
        let algo_area = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        );
        let algo_par = Paragraph::new(algo_line);
        f.render_widget(algo_par, algo_area);
    }
}

fn render_hsl_picker(f: &mut Frame, state: &crate::creator::CreatorState, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
        .title(Span::styled(" HSL Picker ", Style::default().fg(ACCENT)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if !state.editing {
        // Show "Press Enter to edit" centered.
        let msg = Paragraph::new(Line::from(Span::styled(
            "Press Enter to edit",
            Style::default().fg(DIM),
        )));
        let y_offset = inner.height / 2;
        let msg_area = Rect::new(inner.x, inner.y + y_offset, inner.width, 1);
        f.render_widget(msg, msg_area);
        return;
    }

    let color = state.current_color();

    match state.picker_mode {
        PickerMode::Slider => {
            render_slider_mode(f, state, color, inner);
        }
        PickerMode::HexInput => {
            render_hex_input_mode(f, state, color, inner);
        }
    }
}

fn render_slider_mode(
    f: &mut Frame,
    state: &crate::creator::CreatorState,
    color: &HslColor,
    area: Rect,
) {
    let mut y = area.y;

    // Hue slider
    if y < area.y + area.height {
        let is_focused = state.slider_focus == SliderFocus::Hue;
        render_slider_row(
            f,
            area.x,
            y,
            area.width,
            "H",
            color.h,
            0.0,
            360.0,
            is_focused,
            |pos| {
                // Color at this position: vary hue, keep s/l fixed
                let h = HslColor::new(pos, color.s, color.l.max(20.0).min(80.0));
                h.to_ratatui_color()
            },
        );
        y += 1;
    }

    // Spacer
    if y < area.y + area.height {
        y += 1;
    }

    // Saturation slider
    if y < area.y + area.height {
        let is_focused = state.slider_focus == SliderFocus::Saturation;
        render_slider_row(
            f,
            area.x,
            y,
            area.width,
            "S",
            color.s,
            0.0,
            100.0,
            is_focused,
            |pos| {
                let h = HslColor::new(color.h, pos, color.l.max(20.0).min(80.0));
                h.to_ratatui_color()
            },
        );
        y += 1;
    }

    // Spacer
    if y < area.y + area.height {
        y += 1;
    }

    // Lightness slider
    if y < area.y + area.height {
        let is_focused = state.slider_focus == SliderFocus::Lightness;
        render_slider_row(
            f,
            area.x,
            y,
            area.width,
            "L",
            color.l,
            0.0,
            100.0,
            is_focused,
            |pos| {
                let h = HslColor::new(color.h, color.s, pos);
                h.to_ratatui_color()
            },
        );
        y += 1;
    }

    // Spacer
    if y < area.y + area.height {
        y += 1;
    }

    // Color swatch (2 rows)
    let swatch_color = color.to_ratatui_color();
    for _ in 0..2 {
        if y >= area.y + area.height {
            break;
        }
        let swatch_width = area.width.min(20);
        let swatch_line = Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " ".repeat(swatch_width as usize),
                Style::default().bg(swatch_color),
            ),
        ]);
        let swatch_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(swatch_line), swatch_area);
        y += 1;
    }

    // Spacer
    if y < area.y + area.height {
        y += 1;
    }

    // Hex + HSL value display
    if y < area.y + area.height {
        let hex = color.to_hex();
        let info_line = Line::from(vec![
            Span::styled("  Hex: ", Style::default().fg(DIM)),
            Span::styled(&hex, Style::default().fg(Color::White)),
        ]);
        let info_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(info_line), info_area);
        y += 1;
    }
    if y < area.y + area.height {
        let hsl_line = Line::from(vec![
            Span::styled("  HSL: ", Style::default().fg(DIM)),
            Span::styled(
                format!("{:.0}\u{00b0} {:.0}% {:.0}%", color.h, color.s, color.l),
                Style::default().fg(Color::White),
            ),
        ]);
        let hsl_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(hsl_line), hsl_area);
    }
}

fn render_hex_input_mode(
    f: &mut Frame,
    state: &crate::creator::CreatorState,
    color: &HslColor,
    area: Rect,
) {
    let mut y = area.y;

    // Label
    if y < area.y + area.height {
        let label = Line::from(Span::styled(
            "  Hex Input:",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
        let label_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(label), label_area);
        y += 1;
    }

    // Spacer
    if y < area.y + area.height {
        y += 1;
    }

    // Input field with cursor
    if y < area.y + area.height {
        let input_line = Line::from(vec![
            Span::styled("  #", Style::default().fg(Color::White)),
            Span::styled(
                &state.hex_input,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("_", Style::default().fg(ACCENT)),
        ]);
        let input_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(input_line), input_area);
        y += 1;
    }

    // Spacer
    if y < area.y + area.height {
        y += 1;
    }

    // Color swatch (2 rows)
    let swatch_color = color.to_ratatui_color();
    for _ in 0..2 {
        if y >= area.y + area.height {
            break;
        }
        let swatch_width = area.width.min(20);
        let swatch_line = Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " ".repeat(swatch_width as usize),
                Style::default().bg(swatch_color),
            ),
        ]);
        let swatch_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(swatch_line), swatch_area);
        y += 1;
    }

    // Spacer
    if y < area.y + area.height {
        y += 1;
    }

    // HSL value display
    if y < area.y + area.height {
        let hex = color.to_hex();
        let info_line = Line::from(vec![
            Span::styled("  Hex: ", Style::default().fg(DIM)),
            Span::styled(&hex, Style::default().fg(Color::White)),
        ]);
        let info_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(info_line), info_area);
        y += 1;
    }
    if y < area.y + area.height {
        let hsl_line = Line::from(vec![
            Span::styled("  HSL: ", Style::default().fg(DIM)),
            Span::styled(
                format!("{:.0}\u{00b0} {:.0}% {:.0}%", color.h, color.s, color.l),
                Style::default().fg(Color::White),
            ),
        ]);
        let hsl_area = Rect::new(area.x, y, area.width, 1);
        f.render_widget(Paragraph::new(hsl_line), hsl_area);
    }
}

/// Render a single HSL slider row.
///
/// Format: `  H: ████████░░ 240°`
/// Each block character is colored at that position's value via `color_fn`.
/// The `░` marks the current value position.
fn render_slider_row<F>(
    f: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    value: f64,
    min: f64,
    max: f64,
    focused: bool,
    color_fn: F,
) where
    F: Fn(f64) -> Color,
{
    // Layout: "  H: " (6 chars) + bar + " 240°" (~6 chars)
    let prefix_len: u16 = 5;
    let suffix_len: u16 = 6;
    let bar_width = width.saturating_sub(prefix_len + suffix_len) as usize;

    if bar_width == 0 {
        return;
    }

    let label_style = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    };

    // Compute the marker position
    let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let marker_pos =
        ((normalized * (bar_width as f64 - 1.0)).round() as usize).min(bar_width.saturating_sub(1));

    let mut spans = vec![Span::styled(format!("  {}: ", label), label_style)];

    // Build the bar character by character
    for i in 0..bar_width {
        let pos_value = min + (i as f64 / (bar_width as f64 - 1.0).max(1.0)) * (max - min);
        let fg_color = color_fn(pos_value);

        if i == marker_pos {
            // Marker position: use a different character
            spans.push(Span::styled(
                "\u{2591}", // ░
                Style::default().fg(Color::White).bg(fg_color),
            ));
        } else {
            spans.push(Span::styled(
                "\u{2588}", // █
                Style::default().fg(fg_color),
            ));
        }
    }

    // Suffix with value
    let suffix = if max > 200.0 {
        format!(" {:.0}\u{00b0}", value) // degree symbol for hue
    } else {
        format!(" {:.0}%", value)
    };
    spans.push(Span::styled(suffix, Style::default().fg(Color::White)));

    let line = Line::from(spans);
    let row_area = Rect::new(x, y, width, 1);
    f.render_widget(Paragraph::new(line), row_area);
}

fn render_preview_panel(f: &mut Frame, state: &crate::creator::CreatorState, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Preview ", Style::default().fg(ACCENT)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let config = state.build_preview_config();
    f.render_widget(ThemePreview { theme: &config }, inner);
}

fn render_bottom_bar(f: &mut Frame, state: &crate::creator::CreatorState, area: Rect) {
    let spans = if state.editing {
        vec![
            Span::styled(" \u{2190}/\u{2192}", Style::default().fg(ACCENT)),
            Span::styled(":adjust ", Style::default().fg(DIM)),
            Span::styled("Shift+\u{2190}/\u{2192}", Style::default().fg(ACCENT)),
            Span::styled(":\u{00d7}10 ", Style::default().fg(DIM)),
            Span::styled("\u{2191}/\u{2193}", Style::default().fg(ACCENT)),
            Span::styled(":slider ", Style::default().fg(DIM)),
            Span::styled("Tab", Style::default().fg(ACCENT)),
            Span::styled(":hex/slider ", Style::default().fg(DIM)),
            Span::styled("Esc", Style::default().fg(ACCENT)),
            Span::styled(":done", Style::default().fg(DIM)),
        ]
    } else {
        vec![
            Span::styled(" j/k", Style::default().fg(ACCENT)),
            Span::styled(":nav ", Style::default().fg(DIM)),
            Span::styled("Enter", Style::default().fg(ACCENT)),
            Span::styled(":edit ", Style::default().fg(DIM)),
            Span::styled("g", Style::default().fg(ACCENT)),
            Span::styled(":generate ", Style::default().fg(DIM)),
            Span::styled("p", Style::default().fg(ACCENT)),
            Span::styled(":osc preview ", Style::default().fg(DIM)),
            Span::styled("s", Style::default().fg(ACCENT)),
            Span::styled(":save ", Style::default().fg(DIM)),
            Span::styled("Esc", Style::default().fg(ACCENT)),
            Span::styled(":quit", Style::default().fg(DIM)),
        ]
    };

    let bar = Paragraph::new(Line::from(spans));
    f.render_widget(bar, area);
}
