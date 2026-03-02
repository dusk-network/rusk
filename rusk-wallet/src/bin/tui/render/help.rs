// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use super::centered_rect;
use crate::tui::theme;

pub fn render_help(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Help ")
        .title_style(theme::title())
        .border_style(theme::border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_items = vec![
        (
            "Dashboard",
            vec![
                ("\u{2191}\u{2193}/jk", "Navigate menu"),
                ("Enter", "Select action"),
                ("t s u c ...", "Direct hotkeys"),
                ("p", "Switch profile"),
                ("a", "Add profile"),
            ],
        ),
        (
            "Forms",
            vec![
                ("Tab/Down", "Next field"),
                ("Shift+Tab/Up", "Previous field"),
                ("Left/Right", "Cycle select options"),
                ("m", "Fill max amount"),
                ("Home/End", "Move cursor to start/end"),
                ("Enter", "Submit (on last field)"),
                ("Esc", "Cancel / close"),
            ],
        ),
        ("General", vec![("Ctrl+C", "Force quit")]),
    ];

    let mut lines = Vec::new();
    for (section, bindings) in help_items {
        lines.push(Line::from(Span::styled(
            format!("  {section}"),
            theme::heading(),
        )));
        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("    {key:<16}"), theme::hotkey()),
                Span::styled(desc, theme::value()),
            ]));
        }
        lines.push(Line::default());
    }

    lines.push(Line::from(Span::styled(
        "  Press any key to close",
        theme::dim(),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}
