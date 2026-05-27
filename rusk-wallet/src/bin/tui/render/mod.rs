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
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap,
};

use super::app::{App, AppScreen};
use super::{MnemonicEntryScreenConfig, theme};

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
    focused: bool,
) {
    let display = if len == 0 && show_placeholder {
        Span::styled(placeholder, theme::dim())
    } else {
        Span::styled("*".repeat(len), theme::value())
    };
    frame.render_widget(Paragraph::new(Line::from(display)), area);
    if focused && area.width > 0 {
        let cx = area.x + (len as u16).min(area.width - 1);
        frame.set_cursor_position((cx, area.y));
    }
}

fn mask_mnemonic_input(input: &str, cursor: usize) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }

        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let end = i;

        let reveal = cursor >= start && cursor <= end;
        if reveal {
            out.push_str(&input[start..end]);
        } else {
            out.push_str(&"*".repeat(end - start));
        }
    }

    out
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
        | AppScreen::StakeInfo { .. }
        | AppScreen::Addresses => {}
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

pub(super) struct MnemonicEntryView<'a> {
    pub screen: &'a MnemonicEntryScreenConfig,
    pub input: &'a str,
    pub cursor: usize,
    pub error: Option<&'a str>,
}

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

pub fn render_wallet_setup_screen(
    frame: &mut Frame,
    replacing_existing_wallet: bool,
) {
    let inner = wallet_frame(frame, None);
    let content = center_content(inner, 12, 76);

    let (heading, description, detail, restore_action, quit_action, spacing) =
        if replacing_existing_wallet {
            (
                "  Importing Different Wallet.",
                [
                    "  Choose whether to create a new wallet or replace",
                    "  the current one from an existing mnemonic phrase.",
                ],
                Some("  A backup is kept as wallet.dat.old."),
                " Import from mnemonic  ",
                " Cancel",
                1,
            )
        } else {
            (
                "  No wallet found.",
                [
                    "  You can restore a wallet from an existing mnemonic",
                    "  phrase (12 words).",
                ],
                None,
                " Restore from mnemonic  ",
                " Quit",
                2,
            )
        };

    let mut lines = vec![
        Line::default(),
        Line::from(Span::styled(heading, theme::heading())),
        Line::default(),
    ];
    lines.extend(
        description
            .into_iter()
            .map(|line| Line::from(Span::styled(line, theme::value()))),
    );
    if let Some(detail) = detail {
        lines.push(Line::from(Span::styled(detail, theme::dim())));
    }
    for _ in 0..spacing {
        lines.push(Line::default());
    }
    lines.push(Line::from(vec![
        Span::styled("  [c]", theme::hotkey()),
        Span::raw(" Create new wallet  "),
        Span::styled("  [r]", theme::hotkey()),
        Span::raw(restore_action),
        Span::styled("[q]", theme::hotkey()),
        Span::raw(quit_action),
    ]));

    frame.render_widget(Paragraph::new(lines), content);
}

pub fn render_generated_mnemonic_screen(frame: &mut Frame, phrase: &str) {
    let inner = wallet_frame(frame, Some("Create Wallet"));
    let content = center_content(inner, 13, 84);

    let lines = vec![
        Line::from(Span::styled(
            "Write down this 12-word mnemonic phrase and store it safely:",
            theme::heading(),
        )),
        Line::default(),
        Line::from(Span::styled(phrase, theme::value())),
        Line::default(),
        Line::from(Span::styled(
            "This phrase is required to restore access to your wallet.",
            theme::value(),
        )),
        Line::from(Span::styled(
            "Back it up before continuing to verification.",
            theme::dim(),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        content,
    );

    let hint_area =
        Rect::new(content.x, content.y + content.height, content.width, 2);
    error_or_hints(
        frame,
        hint_area,
        None,
        &[("Enter", "Continue"), ("Esc", "Cancel")],
    );
}

pub(super) fn render_mnemonic_entry_screen(
    frame: &mut Frame,
    view: &MnemonicEntryView<'_>,
) {
    let inner = wallet_frame(frame, Some(view.screen.right_title));
    let content = center_content(
        inner,
        view.screen.content_height,
        view.screen.width_pct,
    );
    let heading_height = if view.screen.helper.is_some() { 2 } else { 1 };
    let rows = Layout::vertical([
        Constraint::Length(heading_height), // label
        Constraint::Length(1),              // spacer
        Constraint::Length(3),              // input
        Constraint::Length(1),              // spacer
        Constraint::Length(2),              // hint
    ])
    .split(content);

    let mut label_lines = vec![Line::from(Span::styled(
        view.screen.heading,
        theme::heading(),
    ))];
    if let Some(helper) = view.screen.helper {
        label_lines.push(Line::from(Span::styled(helper, theme::dim())));
    }
    frame.render_widget(
        Paragraph::new(label_lines).wrap(Wrap { trim: false }),
        rows[0],
    );

    let field_inner = input_field(frame, rows[2], "", true);
    let display = if view.input.is_empty() {
        Span::styled("word1 word2 word3 ... word12", theme::dim())
    } else if view.screen.mask_input {
        Span::styled(
            mask_mnemonic_input(view.input, view.cursor),
            theme::value(),
        )
    } else {
        Span::styled(view.input, theme::value())
    };
    frame.render_widget(Paragraph::new(Line::from(display)), field_inner);

    if field_inner.width > 0 {
        let cx =
            field_inner.x + (view.cursor as u16).min(field_inner.width - 1);
        frame.set_cursor_position((cx, field_inner.y));
    }

    error_or_hints(
        frame,
        rows[4],
        view.error,
        &[
            ("Enter", view.screen.enter_hint),
            ("Esc", view.screen.escape_hint),
        ],
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
    masked_input(
        frame,
        pwd_inner,
        pwd_len,
        "Enter password",
        !on_confirm,
        !on_confirm,
    );

    let cfm_inner = input_field(frame, rows[3], "Confirm Password", on_confirm);
    masked_input(
        frame,
        cfm_inner,
        confirm_len,
        "Confirm password",
        on_confirm,
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

pub struct StartupSyncView<'a> {
    pub latest_status: &'a str,
    pub block_height: Option<u64>,
    pub streamed_blocks: Option<u64>,
    pub recent_messages: &'a [String],
    pub stage_label: &'a str,
    pub stage_progress: (usize, usize),
    pub progress_percent: usize,
    pub spinner_frame: usize,
}

pub fn render_startup_sync_screen(
    frame: &mut Frame,
    view: &StartupSyncView<'_>,
) {
    let latest_status = view.latest_status;
    let block_height = view.block_height;
    let streamed_blocks = view.streamed_blocks;
    let recent_messages = view.recent_messages;
    let stage_label = view.stage_label;
    let (stage_index, stage_total) = view.stage_progress;
    let progress_percent = view.progress_percent;
    let spinner_frame = view.spinner_frame;

    let inner = wallet_frame(frame, Some("Syncing"));
    let content = center_content(inner, 17, 84);

    let rows = Layout::vertical([
        Constraint::Length(1), // title
        Constraint::Length(1), // spacer
        Constraint::Length(1), // cycle tracker
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

    let cycle_steps = ["Init", "Cache", "Fetch", "Stream", "Ready"];
    let active_step = stage_index.clamp(1, cycle_steps.len());
    let mut cycle_spans = Vec::new();
    for (idx, step) in cycle_steps.iter().enumerate() {
        if idx > 0 {
            cycle_spans.push(Span::styled(" -> ", theme::dim()));
        }

        let step_number = idx + 1;
        if step_number < active_step {
            cycle_spans
                .push(Span::styled(format!("[x] {step}"), theme::success()));
        } else if step_number == active_step {
            cycle_spans
                .push(Span::styled(format!("[>] {step}"), theme::heading()));
        } else {
            cycle_spans.push(Span::styled(format!("[ ] {step}"), theme::dim()));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(cycle_spans)), rows[2]);

    let stage_line = Line::from(vec![
        Span::styled("Stage: ", theme::label()),
        Span::styled(stage_label, theme::value()),
        Span::styled("  ", theme::dim()),
        Span::styled(format!("{stage_index}/{stage_total}"), theme::heading()),
        Span::styled("  ", theme::dim()),
        Span::styled(format!("{progress_percent}%"), theme::hotkey()),
    ]);
    frame.render_widget(Paragraph::new(stage_line), rows[3]);

    let bar_width = 28usize;
    let filled = if progress_percent == 0 {
        0
    } else {
        (progress_percent.min(100) * bar_width) / 100
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
    frame.render_widget(Paragraph::new(progress_line), rows[4]);

    let height_line = match (block_height, streamed_blocks) {
        (Some(height), Some(delta)) if delta > 0 => Line::from(vec![
            Span::styled("Current block height: ", theme::label()),
            Span::styled(height.to_string(), theme::value()),
            Span::styled("  ", theme::dim()),
            Span::styled(format!("(+{delta} in this sync)"), theme::success()),
        ]),
        (Some(height), _) => Line::from(vec![
            Span::styled("Current block height: ", theme::label()),
            Span::styled(height.to_string(), theme::value()),
        ]),
        (None, _) => Line::from(vec![
            Span::styled("Current block height: ", theme::label()),
            Span::styled("pending...", theme::dim()),
        ]),
    };
    frame.render_widget(Paragraph::new(height_line), rows[6]);

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
    frame.render_widget(Paragraph::new(latest_line), rows[8]);

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

    frame.render_widget(Paragraph::new(lines), rows[10]);
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
