// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ratatui::style::{Color, Modifier, Style};

// ── Palette ──────────────────────────────────────────────────────────

pub const PURPLE: Color = Color::Rgb(138, 79, 255);
pub const PURPLE_DIM: Color = Color::Rgb(90, 50, 170);
pub const CYAN: Color = Color::Rgb(0, 212, 255);
pub const GREEN: Color = Color::Rgb(80, 220, 100);
pub const RED: Color = Color::Rgb(255, 82, 82);
pub const YELLOW: Color = Color::Rgb(255, 213, 79);
pub const DIM: Color = Color::DarkGray;
pub const TEXT: Color = Color::White;
pub const TEXT_SECONDARY: Color = Color::Gray;
pub const HIGHLIGHT_BG: Color = Color::Rgb(45, 20, 80);

// ── Semantic styles ──────────────────────────────────────────────────

pub fn title() -> Style {
    Style::default().fg(PURPLE).add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

pub fn label() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

pub fn value() -> Style {
    Style::default().fg(TEXT)
}

pub fn success() -> Style {
    Style::default().fg(GREEN)
}

pub fn error() -> Style {
    Style::default().fg(RED)
}

pub fn warning() -> Style {
    Style::default().fg(YELLOW)
}

pub fn dim() -> Style {
    Style::default().fg(DIM)
}

pub fn hotkey() -> Style {
    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
}

// ── Selection / menu ─────────────────────────────────────────────────

pub fn selected() -> Style {
    Style::default()
        .fg(TEXT)
        .bg(PURPLE)
        .add_modifier(Modifier::BOLD)
}

pub fn menu_selected() -> Style {
    Style::default()
        .fg(TEXT)
        .bg(HIGHLIGHT_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn menu_item() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

pub fn menu_hotkey() -> Style {
    Style::default().fg(DIM)
}

// ── Borders ──────────────────────────────────────────────────────────

pub fn border() -> Style {
    Style::default().fg(PURPLE_DIM)
}

pub fn border_focused() -> Style {
    Style::default().fg(PURPLE)
}

// ── Status indicators ────────────────────────────────────────────────

pub fn status_ok() -> Style {
    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
}

pub fn status_err() -> Style {
    Style::default().fg(RED).add_modifier(Modifier::BOLD)
}
