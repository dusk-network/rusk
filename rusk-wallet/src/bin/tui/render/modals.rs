// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Wrap};

use super::centered_rect;
use crate::tui::app::{App, ResultInfo};
use crate::tui::theme;

pub fn render_confirmation_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 40, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Confirm Transaction ")
        .title_style(theme::title())
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = app
        .pending_cmd_description
        .iter()
        .map(|d| {
            Line::from(vec![Span::raw("  "), Span::styled(d, theme::value())])
        })
        .collect();

    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("[y]", theme::hotkey()),
        Span::raw(" Confirm  "),
        Span::styled("[n]", theme::hotkey()),
        Span::raw(" Cancel"),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn render_loading_modal(
    frame: &mut Frame,
    description: &str,
    status_messages: &[String],
) {
    let area = centered_rect(50, 30, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Processing ")
        .title_style(theme::title())
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Braille spinner: smoother than ASCII
    let dots = [
        '\u{2834}', '\u{2826}', '\u{2816}', '\u{2812}', '\u{2832}', '\u{2821}',
        '\u{2818}', '\u{2828}',
    ];
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis()
        / 120) as usize;
    let spinner = dots[idx % dots.len()];

    let mut lines = vec![
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{spinner} {description}"), theme::warning()),
        ]),
        Line::default(),
    ];

    for msg in status_messages.iter().rev().take(3) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(msg, theme::dim()),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn render_result_modal(frame: &mut Frame, info: &ResultInfo) {
    let area = centered_rect(55, 35, frame.area());
    frame.render_widget(Clear, area);

    let (title, style) = match info {
        ResultInfo::Error { .. } => (" Error ", theme::error()),
        _ => (" Success ", theme::success()),
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(style)
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    match info {
        ResultInfo::TxSent {
            tx_hash,
            explorer_url,
        } => {
            lines.push(Line::from(Span::styled(
                "  Transaction sent!",
                theme::success(),
            )));
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("  TX: ", theme::label()),
                Span::styled(tx_hash, theme::value()),
            ]));
            if explorer_url.is_some() {
                lines.push(Line::default());
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("[o]", theme::hotkey()),
                    Span::raw(" Open in explorer  "),
                    Span::styled("[Enter]", theme::hotkey()),
                    Span::raw(" Close"),
                ]));
            }
        }
        ResultInfo::DeployTxSent {
            tx_hash,
            contract_id,
            explorer_url,
        } => {
            lines.push(Line::from(Span::styled(
                "  Contract deployed!",
                theme::success(),
            )));
            lines.push(Line::from(vec![
                Span::styled("  Contract: ", theme::label()),
                Span::styled(contract_id, theme::value()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  TX: ", theme::label()),
                Span::styled(tx_hash, theme::value()),
            ]));
            if explorer_url.is_some() {
                lines.push(Line::default());
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("[o]", theme::hotkey()),
                    Span::raw(" Open in explorer  "),
                    Span::styled("[Enter]", theme::hotkey()),
                    Span::raw(" Close"),
                ]));
            }
        }
        ResultInfo::ExportedKeys {
            pub_key_path,
            key_pair_path,
        } => {
            lines.push(Line::from(Span::styled(
                "  Keys exported!",
                theme::success(),
            )));
            lines.push(Line::from(vec![
                Span::styled("  Public key: ", theme::label()),
                Span::styled(pub_key_path, theme::value()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Key pair: ", theme::label()),
                Span::styled(key_pair_path, theme::value()),
            ]));
        }
        ResultInfo::Error { message } => {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(message, theme::error()),
            ]));
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("[r]", theme::hotkey()),
                Span::raw(" Retry  "),
                Span::styled("[Enter]", theme::hotkey()),
                Span::raw(" Close"),
            ]));
        }
    }

    // Close hint for non-error, non-explorer results
    if !matches!(
        info,
        ResultInfo::TxSent {
            explorer_url: Some(_),
            ..
        } | ResultInfo::DeployTxSent {
            explorer_url: Some(_),
            ..
        } | ResultInfo::Error { .. }
    ) {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("[Enter]", theme::hotkey()),
            Span::raw(" Close"),
        ]));
    }

    frame
        .render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
