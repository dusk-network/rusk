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
use crate::tui::forms::field::{FieldKind, FormField};
use crate::tui::forms::{FormId, FormState, TransferModel};
use crate::tui::theme;

const FORM_MIN_WIDTH: u16 = 60;
const FIELD_HEIGHT: u16 = 3;
const RESIZE_WARNING_MIN_WIDTH: u16 = 56;
const RESIZE_WARNING_MIN_HEIGHT: u16 = 9;

pub fn render_form_modal(frame: &mut Frame, form: &FormState) {
    let screen = frame.area();
    let min_height = required_form_height(form);
    let area = form_modal_area(screen, min_height);

    if !form_fits(area, min_height) {
        render_resize_warning(frame, screen, form, min_height);
        return;
    }

    let footer_rows = footer_row_count(form);
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", form.title))
        .title_style(theme::title())
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let constraints: Vec<Constraint> = form
        .fields
        .iter()
        .map(|_| Constraint::Length(FIELD_HEIGHT))
        .chain((0..footer_rows).map(|_| Constraint::Length(1)))
        .collect();

    let field_areas = Layout::vertical(constraints).split(inner);

    for (i, field) in form.fields.iter().enumerate() {
        let is_focused = i == form.focused;
        render_form_field(frame, field_areas[i], field, is_focused);
    }

    let footer_index = form.fields.len();
    if let Some(error) = &form.error {
        let error_area = footer_area(&field_areas, footer_index);
        let error_widget = Paragraph::new(Line::from(vec![
            Span::styled(" Error: ", theme::error()),
            Span::styled(error.clone(), theme::error()),
        ]));
        frame.render_widget(error_widget, error_area);
    } else if form.id == FormId::Transfer {
        render_transfer_model_footer(frame, field_areas[footer_index], form);
        render_form_hints(frame, field_areas[footer_index + 1]);
    } else {
        render_form_hints(frame, field_areas[footer_index]);
    }
}

fn required_form_height(form: &FormState) -> u16 {
    form.fields.len() as u16 * FIELD_HEIGHT + footer_row_count(form) + 2
}

fn form_modal_area(screen: Rect, min_height: u16) -> Rect {
    let full = centered_rect(60, 70, screen);
    let width = full.width.max(FORM_MIN_WIDTH).min(screen.width);
    let height = full.height.max(min_height).min(screen.height);
    let x = screen.x + screen.width.saturating_sub(width) / 2;
    let y = screen.y + screen.height.saturating_sub(height) / 2;

    Rect::new(x, y, width, height)
}

fn form_fits(area: Rect, min_height: u16) -> bool {
    area.width >= FORM_MIN_WIDTH && area.height >= min_height
}

fn footer_row_count(form: &FormState) -> u16 {
    if form.id == FormId::Transfer { 2 } else { 1 }
}

fn footer_area(areas: &[Rect], footer_index: usize) -> Rect {
    let first = areas[footer_index];
    let last = *areas.last().expect("form footer should exist");
    Rect::new(first.x, first.y, first.width, last.bottom() - first.y)
}

fn render_form_hints(frame: &mut Frame, area: Rect) {
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
    frame.render_widget(hint, area);
}

fn render_transfer_model_footer(
    frame: &mut Frame,
    area: Rect,
    form: &FormState,
) {
    let spans = match form.transfer_model() {
        TransferModel::Public => vec![
            Span::styled(" Transaction model: ", theme::label()),
            Span::styled("Public", theme::value()),
        ],
        TransferModel::Shielded => vec![
            Span::styled(" Transaction model: ", theme::label()),
            Span::styled("Shielded", theme::value()),
        ],
        TransferModel::Unknown => vec![
            Span::styled(" Transaction model: ", theme::label()),
            Span::styled("Enter a valid recipient address", theme::dim()),
        ],
    };
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_resize_warning(
    frame: &mut Frame,
    screen: Rect,
    form: &FormState,
    min_height: u16,
) {
    let area = resize_warning_area(screen);
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Terminal Too Small ")
        .title_style(theme::title())
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Form: ", theme::label()),
            Span::styled(&form.title, theme::value()),
        ]),
        Line::from(vec![
            Span::styled("  Current: ", theme::label()),
            Span::styled(
                format!("{}x{}", screen.width, screen.height),
                theme::value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Required: ", theme::label()),
            Span::styled(
                format!("{FORM_MIN_WIDTH}x{min_height}"),
                theme::value(),
            ),
        ]),
        Line::default(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Resize the terminal to view the full form.",
                theme::warning(),
            ),
        ]),
        Line::default(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Esc", theme::dim()),
            Span::styled(" cancel", theme::dim()),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn resize_warning_area(screen: Rect) -> Rect {
    let full = centered_rect(60, 40, screen);
    let width = full.width.max(RESIZE_WARNING_MIN_WIDTH).min(screen.width);
    let height = full
        .height
        .max(RESIZE_WARNING_MIN_HEIGHT)
        .min(screen.height);
    let x = screen.x + screen.width.saturating_sub(width) / 2;
    let y = screen.y + screen.height.saturating_sub(height) / 2;

    Rect::new(x, y, width, height)
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
            let current = options.get(selected).map_or("", String::as_str);
            let display = if focused {
                vec![
                    Span::styled(" \u{25C0} ", theme::dim()),
                    Span::styled(format!(" {current} "), theme::selected()),
                    Span::styled(" \u{25B6} ", theme::dim()),
                ]
            } else {
                vec![Span::styled(format!(" {current} "), theme::value())]
            };
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

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use rusk_wallet::Address;
    use rusk_wallet::Profile;
    use rusk_wallet::currency::Dusk;
    use wallet_core::Seed;
    use wallet_core::keys::{derive_bls_pk, derive_phoenix_pk};

    use super::{
        FIELD_HEIGHT, FORM_MIN_WIDTH, RESIZE_WARNING_MIN_HEIGHT,
        RESIZE_WARNING_MIN_WIDTH, form_fits, form_modal_area,
        render_form_modal, required_form_height, resize_warning_area,
    };
    use crate::tui::forms::{FormId, build_form};

    fn test_profile_at(index: u8) -> Profile {
        let seed: Seed = [7u8; 64];
        Profile {
            shielded_addr: derive_phoenix_pk(&seed, index),
            public_addr: derive_bls_pk(&seed, index),
        }
    }

    fn test_profile() -> Profile {
        test_profile_at(0)
    }

    fn form(id: FormId) -> crate::tui::forms::FormState {
        let temp_dir = std::env::temp_dir();

        build_form(
            id,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &[test_profile()],
            temp_dir.as_path(),
        )
    }

    fn rendered_form(form: &crate::tui::forms::FormState) -> String {
        let backend = TestBackend::new(FORM_MIN_WIDTH, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_form_modal(frame, form))
            .unwrap();

        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn transfer_required_height_is_nineteen() {
        assert_eq!(
            required_form_height(&form(FormId::Transfer)),
            5 * FIELD_HEIGHT + 2 + 2
        );
    }

    #[test]
    fn contract_deploy_required_height_is_twenty_one() {
        assert_eq!(
            required_form_height(&form(FormId::ContractDeploy)),
            6 * FIELD_HEIGHT + 1 + 2
        );
    }

    #[test]
    fn contract_call_required_height_is_twenty_four() {
        assert_eq!(
            required_form_height(&form(FormId::ContractCall)),
            7 * FIELD_HEIGHT + 1 + 2
        );
    }

    #[test]
    fn form_modal_area_honors_minimum_width() {
        let area = form_modal_area(Rect::new(0, 0, FORM_MIN_WIDTH, 24), 24);

        assert_eq!(area.width, FORM_MIN_WIDTH);
    }

    #[test]
    fn fit_check_succeeds_at_required_size() {
        let area = form_modal_area(Rect::new(0, 0, FORM_MIN_WIDTH, 24), 24);

        assert!(form_fits(area, 24));
    }

    #[test]
    fn fit_check_fails_below_required_height() {
        let area = form_modal_area(Rect::new(0, 0, FORM_MIN_WIDTH, 23), 24);

        assert!(!form_fits(area, 24));
    }

    #[test]
    fn resize_warning_area_stays_readable_at_min_supported_screen_size() {
        let area = resize_warning_area(Rect::new(0, 0, FORM_MIN_WIDTH, 20));

        assert_eq!(area.width, RESIZE_WARNING_MIN_WIDTH);
        assert_eq!(area.height, RESIZE_WARNING_MIN_HEIGHT);
    }

    #[test]
    fn selected_stake_owner_is_visible_at_min_width() {
        let temp_dir = std::env::temp_dir();
        let profiles = vec![test_profile_at(0), test_profile_at(1)];
        let mut form = build_form(
            FormId::Stake,
            0,
            Dusk::from(11),
            Dusk::from(22),
            None,
            &profiles,
            temp_dir.as_path(),
        );
        let owner_idx = form
            .fields
            .iter()
            .position(|field| field.name == "owner")
            .expect("stake form should have owner field");
        form.focused = owner_idx;
        form.cycle_next();

        let rendered = rendered_form(&form);

        assert!(rendered.contains("Profile 2"));
        assert!(rendered.contains(
            Address::Public(profiles[1].public_addr).preview().as_str()
        ));
    }
}
