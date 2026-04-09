// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use super::centered_rect;
use crate::tui::forms::FormState;
use crate::tui::forms::field::{FieldKind, FormField};
use crate::tui::theme;

pub fn render_form_modal(frame: &mut Frame, form: &FormState) {
    let screen = frame.area();
    // Each field is 3 rows tall; add 1 for the hint/error row and 2 for the
    // modal border.
    let needed_h = (form.fields.len() as u16 * 3 + 3).min(screen.height);
    let area = {
        let full = centered_rect(60, 70, screen);
        // If the percentage-based height is too small, centre a fixed-height
        // rect instead so all fields remain visible.
        if full.height < needed_h {
            let top = screen.y + screen.height.saturating_sub(needed_h) / 2;
            Rect::new(full.x, top, full.width, needed_h)
        } else {
            full
        }
    };
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", form.title))
        .title_style(theme::title())
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let field_height = 3u16;
    let constraints: Vec<Constraint> = form
        .fields
        .iter()
        .map(|_| Constraint::Length(field_height))
        .chain(std::iter::once(Constraint::Min(1))) // error/hint area
        .collect();

    let field_areas = Layout::vertical(constraints).split(inner);

    for (i, field) in form.fields.iter().enumerate() {
        let is_focused = i == form.focused;
        render_form_field(frame, field_areas[i], field, is_focused);
    }

    // Error display at bottom
    if let Some(error) = &form.error {
        let error_area = *field_areas.last().unwrap();
        let error_widget = Paragraph::new(Line::from(vec![
            Span::styled(" Error: ", theme::error()),
            Span::styled(error.clone(), theme::error()),
        ]));
        frame.render_widget(error_widget, error_area);
    } else {
        let hint_area = *field_areas.last().unwrap();
        let hint = Paragraph::new(Line::from(vec![
            Span::styled(" Tab", theme::dim()),
            Span::styled(" next", theme::dim()),
            Span::styled("  \u{00b7}  ", theme::dim()),
            Span::styled("Shift+Tab", theme::dim()),
            Span::styled(" prev", theme::dim()),
            Span::styled("  \u{00b7}  ", theme::dim()),
            Span::styled("Enter", theme::dim()),
            Span::styled(" submit", theme::dim()),
            Span::styled("  \u{00b7}  ", theme::dim()),
            Span::styled("Esc", theme::dim()),
            Span::styled(" cancel", theme::dim()),
        ]));
        frame.render_widget(hint, hint_area);
    }
}

fn render_form_field(
    frame: &mut Frame,
    area: Rect,
    field: &FormField,
    focused: bool,
) {
    let border_style = if focused {
        theme::border_focused()
    } else {
        theme::border()
    };

    // Build the title with optional hints
    let title = field_title(field);

    let block = Block::default()
        .title(title)
        .title_style(if focused {
            theme::heading()
        } else {
            theme::label()
        })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match &field.kind {
        FieldKind::Select { options } => {
            let selected = field.selected_option.unwrap_or(0);
            let display: Vec<Span> = options
                .iter()
                .enumerate()
                .flat_map(|(i, opt)| {
                    if i == selected {
                        vec![
                            Span::styled(
                                if focused { " \u{25C0} " } else { " " },
                                theme::dim(),
                            ),
                            Span::styled(format!(" {opt} "), theme::selected()),
                            Span::styled(
                                if focused { " \u{25B6} " } else { " " },
                                theme::dim(),
                            ),
                        ]
                    } else {
                        vec![Span::styled(format!("  {opt}  "), theme::dim())]
                    }
                })
                .collect();
            frame.render_widget(Paragraph::new(Line::from(display)), inner);
        }
        _ => {
            let display_val = field.display_value();
            let display = if display_val.is_empty() {
                Span::styled(&field.placeholder, theme::dim())
            } else {
                Span::styled(display_val, theme::value())
            };

            frame.render_widget(Paragraph::new(Line::from(display)), inner);

            // Show cursor position for focused text fields
            if focused && inner.width > 0 {
                let cursor_x =
                    inner.x + (field.cursor as u16).min(inner.width - 1);
                frame.set_cursor_position((cursor_x, inner.y));
            }
        }
    }
}

fn field_title(field: &FormField) -> String {
    let label = &field.label;

    match &field.kind {
        FieldKind::Amount { max: Some(max) } => {
            format!(" {label} (max: {max}) [m] ")
        }
        FieldKind::Amount { max: None } => {
            format!(" {label} (max: Unknown) ")
        }
        _ => format!(" {label} "),
    }
}
