// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod dashboard;
mod forms;
mod help;
mod modals;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph};

use super::app::{App, AppScreen};
use super::theme;

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;

// ── Shared helpers ──────────────────────────────────────────────────

/// Render the standard outer wallet frame and return the inner area.
fn wallet_frame(frame: &mut Frame, right_title: Option<&str>) -> Rect {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled(" Rusk Wallet ", theme::title()),
            Span::styled(
                format!("v{} ", env!("CARGO_PKG_VERSION")),
                theme::dim(),
            ),
        ]))
        .border_style(theme::border())
        .padding(Padding::horizontal(1));

    if let Some(t) = right_title {
        block = block.title(
            Line::from(Span::styled(format!(" {t} "), theme::heading()))
                .right_aligned(),
        );
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

/// Center a fixed-height content area within `area`.
fn center_content(area: Rect, height: u16, h_pct: u16) -> Rect {
    let v = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(height),
        Constraint::Min(1),
    ])
    .split(area);

    let side = (100 - h_pct) / 2;
    Layout::horizontal([
        Constraint::Percentage(side),
        Constraint::Percentage(h_pct),
        Constraint::Percentage(side),
    ])
    .split(v[1])[1]
}

/// Render a bordered input field, returning its inner rect.
fn input_field(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
) -> Rect {
    let block = Block::default()
        .title(Span::styled(
            format!(" {title} "),
            if focused {
                theme::heading()
            } else {
                theme::label()
            },
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            theme::border_focused()
        } else {
            theme::border()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

/// Render masked password text + cursor in an area.
fn masked_input(
    frame: &mut Frame,
    area: Rect,
    len: usize,
    placeholder: &str,
    show_placeholder: bool,
) {
    let display = if len == 0 && show_placeholder {
        Span::styled(placeholder, theme::dim())
    } else {
        Span::styled("*".repeat(len), theme::value())
    };
    frame.render_widget(Paragraph::new(Line::from(display)), area);
    if area.width > 0 {
        let cx = area.x + (len as u16).min(area.width - 1);
        frame.set_cursor_position((cx, area.y));
    }
}

/// Render either an error message or a hint bar.
fn error_or_hints(
    frame: &mut Frame,
    area: Rect,
    error: Option<&str>,
    hints: &[(&str, &str)],
) {
    if let Some(err) = error {
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {err}"), theme::error())),
            area,
        );
    } else {
        let mut spans = Vec::new();
        for (i, (key, desc)) in hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
            }
            spans.push(Span::styled(*key, theme::dim()));
            spans.push(Span::styled(format!(" {desc}"), theme::dim()));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

// ── Top-level dispatch ──────────────────────────────────────────────

/// Top-level render dispatch.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Paragraph::new(format!(
                "Terminal too small ({}x{}). Need at least {}x{}.",
                area.width, area.height, MIN_WIDTH, MIN_HEIGHT
            ))
            .style(theme::warning()),
            area,
        );
        return;
    }

    // Dashboard always renders (History/StakeInfo swap the menu area)
    dashboard::render_dashboard(frame, app);

    // Overlay screens render on top of the dashboard
    match &app.screen {
        AppScreen::Dashboard
        | AppScreen::History { .. }
        | AppScreen::StakeInfo => {}
        AppScreen::Form { form } => forms::render_form_modal(frame, form),
        AppScreen::Confirmation => {
            modals::render_confirmation_modal(frame, app)
        }
        AppScreen::Executing { description } => modals::render_loading_modal(
            frame,
            description,
            &app.status_messages,
        ),
        AppScreen::Result { info } => modals::render_result_modal(frame, info),
        AppScreen::Help => help::render_help(frame),
    }
}

// ── Pre-wallet screens ──────────────────────────────────────────────

pub fn render_password_screen(
    frame: &mut Frame,
    pwd_len: usize,
    error: Option<&str>,
) {
    let inner = wallet_frame(frame, None);
    let content = center_content(inner, 8, 60);

    let field_inner = input_field(frame, content, "Password", true);
    masked_input(
        frame,
        field_inner,
        pwd_len,
        "Enter your wallet password",
        true,
    );

    let hint_area =
        Rect::new(content.x, content.y + content.height, content.width, 2);
    error_or_hints(
        frame,
        hint_area,
        error,
        &[("Enter", "Submit"), ("Esc", "Quit")],
    );
}

pub fn render_welcome_screen(frame: &mut Frame) {
    let inner = wallet_frame(frame, None);
    let content = center_content(inner, 10, 70);

    let lines = vec![
        Line::default(),
        Line::from(Span::styled("  No wallet found.", theme::heading())),
        Line::default(),
        Line::from(Span::styled(
            "  You can restore a wallet from an existing mnemonic",
            theme::value(),
        )),
        Line::from(Span::styled("  phrase (12 words).", theme::value())),
        Line::default(),
        Line::default(),
        Line::from(vec![
            Span::styled("  [r]", theme::hotkey()),
            Span::raw(" Restore from mnemonic  "),
            Span::styled("[q]", theme::hotkey()),
            Span::raw(" Quit"),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), content);
}

pub fn render_mnemonic_screen(
    frame: &mut Frame,
    input: &str,
    cursor: usize,
    error: Option<&str>,
) {
    let inner = wallet_frame(frame, Some("Restore"));
    let content = center_content(inner, 10, 80);

    let rows = Layout::vertical([
        Constraint::Length(1), // label
        Constraint::Length(1), // spacer
        Constraint::Length(3), // input
        Constraint::Length(1), // spacer
        Constraint::Length(2), // hint
    ])
    .split(content);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "Enter your 12-word mnemonic phrase:",
            theme::heading(),
        )),
        rows[0],
    );

    let field_inner = input_field(frame, rows[2], "", true);
    let display = if input.is_empty() {
        Span::styled("word1 word2 word3 ... word12", theme::dim())
    } else {
        Span::styled(input, theme::value())
    };
    frame.render_widget(Paragraph::new(Line::from(display)), field_inner);

    if field_inner.width > 0 {
        let cx = field_inner.x + (cursor as u16).min(field_inner.width - 1);
        frame.set_cursor_position((cx, field_inner.y));
    }

    error_or_hints(
        frame,
        rows[4],
        error,
        &[("Enter", "Submit"), ("Esc", "Back")],
    );
}

pub fn render_new_password_screen(
    frame: &mut Frame,
    pwd_len: usize,
    confirm_len: usize,
    on_confirm: bool,
    error: Option<&str>,
) {
    let inner = wallet_frame(frame, Some("Set Password"));
    let content = center_content(inner, 12, 60);

    let rows = Layout::vertical([
        Constraint::Length(1), // label
        Constraint::Length(1), // spacer
        Constraint::Length(3), // password
        Constraint::Length(3), // confirm
        Constraint::Length(1), // spacer
        Constraint::Length(2), // hint
    ])
    .split(content);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "Create a password to secure your wallet:",
            theme::heading(),
        )),
        rows[0],
    );

    let pwd_inner = input_field(frame, rows[2], "Password", !on_confirm);
    masked_input(frame, pwd_inner, pwd_len, "Enter password", !on_confirm);

    let cfm_inner = input_field(frame, rows[3], "Confirm Password", on_confirm);
    masked_input(
        frame,
        cfm_inner,
        confirm_len,
        "Confirm password",
        on_confirm,
    );

    error_or_hints(
        frame,
        rows[5],
        error,
        &[
            ("Tab/Enter", "Next"),
            ("Shift+Tab", "Prev"),
            ("Esc", "Back"),
        ],
    );
}

pub fn render_startup_sync_screen(
    frame: &mut Frame,
    latest_status: &str,
    block_height: Option<u64>,
    recent_messages: &[String],
    stage_label: &str,
    stage_progress: (usize, usize),
    spinner_frame: usize,
) {
    let inner = wallet_frame(frame, Some("Syncing"));
    let content = center_content(inner, 16, 80);
    let (stage_index, stage_total) = stage_progress;

    let rows = Layout::vertical([
        Constraint::Length(1), // title
        Constraint::Length(1), // spacer
        Constraint::Length(1), // stage
        Constraint::Length(1), // progress bar
        Constraint::Length(1), // spacer
        Constraint::Length(1), // height
        Constraint::Length(1), // spacer
        Constraint::Length(1), // latest status
        Constraint::Length(1), // spacer
        Constraint::Min(4),    // recent messages
    ])
    .split(content);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "Synchronizing chain state before opening the wallet UI...",
            theme::heading(),
        )),
        rows[0],
    );

    let stage_line = Line::from(vec![
        Span::styled("Stage: ", theme::label()),
        Span::styled(stage_label, theme::value()),
        Span::styled("  ", theme::dim()),
        Span::styled(format!("{stage_index}/{stage_total}"), theme::heading()),
    ]);
    frame.render_widget(Paragraph::new(stage_line), rows[2]);

    let bar_width = 28usize;
    let filled = if stage_total == 0 {
        0
    } else {
        (stage_index.min(stage_total) * bar_width) / stage_total
    };
    let spinner = ['|', '/', '-', '\\'][spinner_frame % 4];
    let progress_line = Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("=".repeat(filled), theme::success()),
        Span::styled(
            "-".repeat(bar_width.saturating_sub(filled)),
            theme::dim(),
        ),
        Span::styled("]", theme::dim()),
        Span::styled(format!("  {spinner}"), theme::hotkey()),
    ]);
    frame.render_widget(Paragraph::new(progress_line), rows[3]);

    let height_line = match block_height {
        Some(height) => Line::from(vec![
            Span::styled("Current block height: ", theme::label()),
            Span::styled(height.to_string(), theme::value()),
        ]),
        None => Line::from(vec![
            Span::styled("Current block height: ", theme::label()),
            Span::styled("pending...", theme::dim()),
        ]),
    };
    frame.render_widget(Paragraph::new(height_line), rows[5]);

    let latest_line = if latest_status.is_empty() {
        Line::from(vec![
            Span::styled("Status: ", theme::label()),
            Span::styled("initializing...", theme::dim()),
        ])
    } else {
        Line::from(vec![
            Span::styled("Status: ", theme::label()),
            Span::styled(latest_status, theme::value()),
        ])
    };
    frame.render_widget(Paragraph::new(latest_line), rows[7]);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "Recent sync events:",
        theme::label(),
    )));

    for msg in recent_messages.iter().rev().take(4).rev() {
        lines.push(Line::from(vec![
            Span::styled("  - ", theme::dim()),
            Span::styled(msg.as_str(), theme::dim()),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), rows[9]);
}

// ── Utilities ───────────────────────────────────────────────────────

/// Create a centered rectangle within the given area.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(v[1])[1]
}
