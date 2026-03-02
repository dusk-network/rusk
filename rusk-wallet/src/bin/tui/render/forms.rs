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
    let area = centered_rect(60, 70, frame.area());
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

    let label = &field.label;

    // Build the title with optional hints
    let title = match &field.kind {
        FieldKind::Amount { max } => {
            format!(" {label} (max: {max}) [m] ")
        }
        _ => format!(" {label} "),
    };

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
