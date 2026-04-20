// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Padding, Paragraph,
};
use rusk_wallet::currency::Dusk;
use wallet_core::BalanceInfo;

use crate::command::TransactionHistory;
use crate::tui::app::{App, AppScreen, MENU_ITEMS, StakeState, SyncStatus};
use crate::tui::theme;

const MAX_BALANCE_TEXT_LEN: usize = 24;

pub fn render_dashboard(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let num_profiles = app.wallet.profiles().len();
    let profile_label =
        format!(" Profile {} of {} ", app.profile_idx + 1, num_profiles);

    let outer = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled(" Rusk Wallet ", theme::title()),
            Span::styled(
                format!("v{} ", env!("CARGO_PKG_VERSION")),
                theme::dim(),
            ),
        ]))
        .title(
            Line::from(Span::styled(profile_label, theme::heading()))
                .right_aligned(),
        )
        .border_style(theme::border())
        .padding(Padding::horizontal(1));

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let profile = app.current_profile();

    let w = inner.width.max(1);
    let shielded_full = profile.shielded_account_string();
    let public_full = profile.public_account_string();
    let sh_rows = (12 + shielded_full.len() as u16).div_ceil(w);
    let pu_rows = (12 + public_full.len() as u16).div_ceil(w);
    // Both show full (≤2 rows each) or both fall back to the preview (1 row).
    let use_full = sh_rows <= 2 && pu_rows <= 2;
    let (shielded_text, public_text) = if use_full {
        (shielded_full, public_full)
    } else {
        (
            profile.shielded_account_preview(),
            profile.public_account_preview(),
        )
    };
    let info_height = if use_full { sh_rows + pu_rows } else { 2 } + 4;

    // Layout: info section, separator + menu, hint bar
    let layout = Layout::vertical([
        Constraint::Length(info_height),
        Constraint::Min(4), // menu (scrollable, fills remaining)
        Constraint::Length(1), // hint bar
    ])
    .split(inner);

    let bal = app.current_balance();
    let stake = app.current_stake_state();

    render_info_block(
        frame,
        layout[0],
        app,
        &bal,
        stake,
        shielded_text,
        public_text,
    );

    match &app.screen {
        AppScreen::History { entries } => {
            render_history_panel(
                frame,
                layout[1],
                entries,
                app.history_selected,
            );
            render_panel_hint_bar(
                frame,
                layout[2],
                &[("\u{2191}\u{2193}", "Scroll"), ("Esc", "Back")],
            );
        }
        AppScreen::StakeInfo => {
            render_stake_panel(frame, layout[1], stake);
            render_panel_hint_bar(frame, layout[2], &[("Esc", "Back")]);
        }
        AppScreen::Addresses => {
            render_addresses_panel(frame, layout[1], profile);
            render_panel_hint_bar(
                frame,
                layout[2],
                &[
                    ("s", "Copy Shielded"),
                    ("p", "Copy Public"),
                    ("Esc", "Back"),
                ],
            );
            if let Some(msg) = app.clipboard_message() {
                // Overlay a temporary success toast on the right side of the
                // hint bar so the shortcuts remain visible.
                let area = layout[2];
                let text = format!(" {msg} ");
                let toast_w = (text.len() as u16).min(area.width);
                let toast = Rect {
                    x: area.x + area.width.saturating_sub(toast_w),
                    y: area.y,
                    width: toast_w,
                    height: area.height,
                };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        text,
                        theme::success(),
                    ))),
                    toast,
                );
            }
        }
        _ => {
            render_menu(
                frame,
                layout[1],
                app.menu_selected,
                app.num_profiles(),
            );
            render_hint_bar(frame, layout[2]);
        }
    }
}

// ── Info block ───────────────────────────────────────────────────────

fn render_info_block(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    app: &App,
    bal: &super::super::app::ProfileBalance,
    stake: Option<&StakeState>,
    shielded_text: String,
    public_text: String,
) {
    let lines = vec![
        // ── Shielded ───────────────────────────────────────────────
        Line::from(vec![
            Span::styled("  Shielded  ", theme::heading()),
            Span::styled(shielded_text, theme::dim()),
        ]),
        shielded_balance_line(bal.phoenix.as_ref()),
        // ── Public ─────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("  Public    ", theme::heading()),
            Span::styled(public_text, theme::dim()),
        ]),
        public_balance_line(bal.moonlight),
        // ── Staking ────────────────────────────────────────────────
        staking_line(stake),
        // ── Status ─────────────────────────────────────────────────
        status_line(
            &app.sync_status,
            app.sync_block_height,
            &app.network_label,
            &app.connection,
        ),
    ];

    frame.render_widget(
        Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false }),
        area,
    );
}

fn shielded_balance_line(balance: Option<&BalanceInfo>) -> Line<'static> {
    match balance {
        Some(bal) => {
            let spendable: Dusk = bal.spendable.into();
            let total: Dusk = bal.value.into();
            Line::from(vec![
                Span::raw("              "),
                Span::styled("Spendable ", theme::label()),
                Span::styled(truncate_balance(spendable), theme::value()),
                Span::styled("  \u{00b7}  ", theme::dim()),
                Span::styled("Total ", theme::label()),
                Span::styled(truncate_balance(total), theme::value()),
            ])
        }
        None => Line::from(vec![
            Span::raw("              "),
            Span::styled("Syncing\u{2026}", theme::dim()),
        ]),
    }
}

fn public_balance_line(balance: Option<Dusk>) -> Line<'static> {
    match balance {
        Some(bal) => Line::from(vec![
            Span::raw("              "),
            Span::styled("Balance ", theme::label()),
            Span::styled(truncate_balance(bal), theme::value()),
        ]),
        None => Line::from(vec![
            Span::raw("              "),
            Span::styled("Loading\u{2026}", theme::dim()),
        ]),
    }
}

fn staking_line(stake: Option<&StakeState>) -> Line<'static> {
    let mut spans = Vec::new();

    match stake {
        Some(StakeState::Loaded(data)) => {
            if let Some(amt) = &data.amount {
                let eligible: Dusk = amt.value.into();
                let rewards: Dusk = data.reward.into();
                spans.push(Span::styled("  Staking   ", theme::heading()));
                spans.push(Span::styled(
                    truncate_balance(eligible),
                    theme::value(),
                ));
                spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
                spans.push(Span::styled("Rewards ", theme::label()));
                spans.push(Span::styled(
                    truncate_balance(rewards),
                    theme::success(),
                ));
            } else {
                let rewards: Dusk = data.reward.into();
                if rewards > Dusk::from(0) {
                    spans.push(Span::styled("  Rewards   ", theme::heading()));
                    spans.push(Span::styled(
                        truncate_balance(rewards),
                        theme::success(),
                    ));
                } else {
                    spans.push(Span::styled("  No stake", theme::dim()));
                }
            }
        }
        Some(StakeState::NoStake) => {
            spans.push(Span::styled("  No stake", theme::dim()));
        }
        None => {
            spans.push(Span::styled("  \u{2500}\u{2500}", theme::dim()));
        }
    }

    Line::from(spans)
}

fn truncate_balance(value: Dusk) -> String {
    let text = value.to_string();
    if text.chars().count() <= MAX_BALANCE_TEXT_LEN {
        return text;
    }

    let mut shortened = String::new();
    for ch in text.chars().take(MAX_BALANCE_TEXT_LEN - 3) {
        shortened.push(ch);
    }
    shortened.push_str("...");
    shortened
}

fn status_line(
    sync: &SyncStatus,
    block_height: Option<u64>,
    network_label: &str,
    conn: &super::super::app::ConnectionStatus,
) -> Line<'static> {
    let mut spans = Vec::new();
    spans.push(Span::raw("  "));

    if !conn.state {
        spans.push(Span::styled(
            "\u{25cf} OFFLINE",
            ratatui::style::Style::default()
                .fg(theme::RED)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
        spans.push(Span::styled(network_label.to_string(), theme::heading()));
        return Line::from(spans);
    }

    match sync {
        SyncStatus::Synced => {
            spans.push(Span::styled("\u{25cf} ", theme::status_ok()));
            spans.push(Span::styled("Synced", theme::dim()));
            spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
            spans.push(Span::styled(
                network_label.to_string(),
                theme::heading(),
            ));
            spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
            spans.push(Span::styled("Block ", theme::label()));
            match block_height {
                Some(height) => {
                    spans
                        .push(Span::styled(height.to_string(), theme::value()));
                }
                None => {
                    spans.push(Span::styled("--", theme::dim()));
                }
            }
        }
        SyncStatus::Syncing => {
            spans.push(Span::styled("\u{25cf} ", theme::warning()));
            spans.push(Span::styled("Syncing\u{2026}", theme::dim()));
            spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
            spans.push(Span::styled(
                network_label.to_string(),
                theme::heading(),
            ));
            spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
            spans.push(Span::styled("Block ", theme::label()));
            match block_height {
                Some(height) => {
                    spans
                        .push(Span::styled(height.to_string(), theme::value()));
                }
                None => {
                    spans.push(Span::styled("--", theme::dim()));
                }
            }
        }
        SyncStatus::Error(e) => {
            spans.push(Span::styled("\u{25cf} ", theme::status_err()));
            spans.push(Span::styled(
                network_label.to_string(),
                theme::heading(),
            ));
            spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
            let msg = if e.len() > 20 {
                format!("Err: {e:.20}\u{2026}")
            } else {
                format!("Err: {e}")
            };
            spans.push(Span::styled(msg, theme::dim()));
        }
        SyncStatus::Unknown => {
            spans.push(Span::styled("\u{25cb} ", theme::dim()));
            spans.push(Span::styled(
                network_label.to_string(),
                theme::heading(),
            ));
            spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
            spans.push(Span::styled("--", theme::dim()));
        }
    }

    Line::from(spans)
}

// ── Menu list ────────────────────────────────────────────────────────

fn render_menu(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    selected: usize,
    _num_profiles: usize,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme::border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;
    let total = MENU_ITEMS.len();
    let offset = scroll_offset(selected, total, visible);
    let width = inner.width as usize;

    let mut lines = Vec::new();
    let end = (offset + visible).min(total);

    for (i, (key, static_label)) in
        MENU_ITEMS.iter().enumerate().take(end).skip(offset)
    {
        if i == selected {
            // Full-width highlight bar with arrow indicator
            let pad = width.saturating_sub(7); // "  ▸ " (4) + " X " (3)
            let text = format!("  \u{25b8} {static_label:<pad$} {key} ");
            lines.push(Line::from(Span::styled(text, theme::menu_selected())));
        } else {
            let pad = width.saturating_sub(7);
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{static_label:<pad$}"),
                    theme::menu_item(),
                ),
                Span::styled(format!(" {key} "), theme::menu_hotkey()),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn scroll_offset(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let half = visible / 2;
    if selected <= half {
        0
    } else if selected >= total - half {
        total - visible
    } else {
        selected - half
    }
}

// ── Hint bar ─────────────────────────────────────────────────────────

fn render_hint_bar(frame: &mut Frame, area: ratatui::layout::Rect) {
    render_panel_hint_bar(
        frame,
        area,
        &[
            ("\u{2191}\u{2193}", "Navigate"),
            ("Enter", "Select"),
            ("v", "Addresses"),
            ("?", "Help"),
            ("q", "Quit"),
        ],
    );
}

fn render_panel_hint_bar(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    hints: &[(&str, &str)],
) {
    let mut spans = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  \u{00b7}  ", theme::dim()));
        }
        spans.push(Span::styled(format!(" {key}"), theme::dim()));
        spans.push(Span::styled(format!(" {desc}"), theme::dim()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_addresses_panel(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    profile: &rusk_wallet::Profile,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme::border())
        .title(Span::styled(" Full Addresses ", theme::heading()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let shielded =
        rusk_wallet::Address::Shielded(profile.shielded_addr).to_string();
    let public = rusk_wallet::Address::Public(profile.public_addr).to_string();

    let lines = vec![
        Line::from(Span::styled("  Shielded:", theme::label())),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(shielded, theme::value()),
        ]),
        Line::default(),
        Line::from(Span::styled("  Public:", theme::label())),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(public, theme::value()),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false }),
        inner,
    );
}

// ── History panel (replaces menu) ───────────────────────────────────

fn render_history_panel(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    entries: &[TransactionHistory],
    selected: usize,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme::border())
        .title(Span::styled(" Transaction History ", theme::heading()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if entries.is_empty() {
        frame.render_widget(
            Paragraph::new("  No transactions found").style(theme::dim()),
            inner,
        );
        return;
    }

    let visible = inner.height as usize;
    let total = entries.len();
    let offset = scroll_offset(selected, total, visible);
    let end = (offset + visible).min(total);

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .skip(offset)
        .take(end - offset)
        .map(|(i, tx)| {
            let style = if i == selected {
                theme::selected()
            } else {
                theme::value()
            };
            ListItem::new(format!("  {}", tx.tui_line())).style(style)
        })
        .collect();

    frame.render_widget(List::new(items), inner);
}

// ── Stake info panel (replaces menu) ────────────────────────────────

fn render_stake_panel(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    stake: Option<&StakeState>,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme::border())
        .title(Span::styled(" Stake Info ", theme::heading()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    match stake {
        Some(StakeState::Loaded(data)) => {
            if let Some(amt) = &data.amount {
                let eligible: Dusk = amt.value.into();
                let locked: Dusk = amt.locked.into();
                lines.push(Line::from(vec![
                    Span::styled("  Eligible     ", theme::label()),
                    Span::styled(format!("{eligible}"), theme::value()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Locked       ", theme::label()),
                    Span::styled(format!("{locked}"), theme::value()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Eligibility  ", theme::label()),
                    Span::styled(
                        format!("from epoch {}", amt.eligibility),
                        theme::dim(),
                    ),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  No active stake",
                    theme::dim(),
                )));
            }

            lines.push(Line::default());
            let rewards: Dusk = data.reward.into();
            lines.push(Line::from(vec![
                Span::styled("  Rewards      ", theme::label()),
                Span::styled(format!("{rewards}"), theme::success()),
            ]));

            let faults = data.faults;
            let hard_faults = data.hard_faults;
            if faults > 0 || hard_faults > 0 {
                lines.push(Line::default());
                lines.push(Line::from(vec![
                    Span::styled("  Faults       ", theme::label()),
                    Span::styled(format!("{faults}"), theme::warning()),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("  Hard faults  ", theme::label()),
                    Span::styled(
                        format!("{hard_faults}"),
                        if hard_faults > 0 {
                            theme::error()
                        } else {
                            theme::dim()
                        },
                    ),
                ]));
            }
        }
        Some(StakeState::NoStake) => {
            lines.push(Line::from(Span::styled(
                "  No stake found for this profile",
                theme::dim(),
            )));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "  Loading stake info\u{2026}",
                theme::dim(),
            )));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
