// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{Stdout, stdout};
use std::sync::Mutex;

use crossterm::cursor::{self, MoveUp};
use crossterm::execute;
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType};
use tracing::info;

const MAX_INTERACTIVE_STATUS_LINES: usize = 5;

/// Displays status for interactive mode in a windowed output
/// to avoid polluting stdout.
struct InteractiveStatus {
    status_lines_buffer: [Option<String>; MAX_INTERACTIVE_STATUS_LINES],
    next_line_in_buffer: usize,
    last_pos_in_terminal: Option<(u16, u16)>,
}

impl InteractiveStatus {
    const fn new() -> Self {
        Self {
            status_lines_buffer: [None, None, None, None, None],
            next_line_in_buffer: 0,
            last_pos_in_terminal: None,
        }
    }

    fn print(&mut self, status: &str) {
        let mut stdout = stdout();
        self.clear_lines_in_stdout(&mut stdout);
        if self.next_line_in_buffer == MAX_INTERACTIVE_STATUS_LINES {
            self.shift_buffer_lines_up();
        }
        self.status_lines_buffer[self.next_line_in_buffer] =
            Some(self.replace_non_ascii(status));
        self.flush_buffer_to_stdout(&mut stdout);
        self.next_line_in_buffer += 1;
    }

    fn clear_lines_in_stdout(&mut self, stdout: &mut Stdout) {
        if self.something_was_printed_elsewhere() {
            // Reset the window to avoid overwriting the output
            // that was printed elsewhere.
            self.clear_buffer();
            return;
        }
        let terminal_size = terminal::size();
        for i in (0..self.next_line_in_buffer).rev() {
            let num_lines_to_clear = if let Ok((width, _)) = terminal_size {
                let line = self.status_lines_buffer[i]
                    .as_ref()
                    .expect("a message should be present");
                let width = width as usize;
                // +2 accounts for the "> " prefix added in
                // flush_buffer_to_stdout
                let printed_len = line.len() + 2;
                let (mut num_lines, a_partially_filled_line) =
                    (printed_len / width, !printed_len.is_multiple_of(width));
                if a_partially_filled_line {
                    num_lines += 1;
                }
                num_lines
            } else {
                1
            };
            for _ in 0..num_lines_to_clear {
                let _ =
                    execute!(stdout, MoveUp(1), Clear(ClearType::CurrentLine));
            }
        }
    }

    fn shift_buffer_lines_up(&mut self) {
        for i in 0..self.status_lines_buffer.len() - 1 {
            self.status_lines_buffer[i] =
                self.status_lines_buffer[i + 1].take();
        }
        self.next_line_in_buffer -= 1;
    }

    fn flush_buffer_to_stdout(&mut self, stdout: &mut Stdout) {
        for line in self.status_lines_buffer.iter() {
            if let Some(status) = line.as_ref() {
                let _ = execute!(
                    stdout,
                    SetAttribute(Attribute::Dim),
                    Print(format!("> {}\n", status)),
                    SetAttribute(Attribute::Reset),
                );
            }
        }
        self.last_pos_in_terminal = cursor::position().ok();
    }

    fn clear_buffer(&mut self) {
        for line in self.status_lines_buffer.iter_mut() {
            *line = None;
        }
        self.next_line_in_buffer = 0;
    }

    fn replace_non_ascii(&self, status: &str) -> String {
        // Non-ascii characters are replaced here because their
        // column widths in the terminal can vary and that will affect
        // how multi-line status messages are removed.
        status
            .chars()
            .map(|c| if c.is_ascii() { c } else { ' ' })
            .collect()
    }

    /// Returns true if something has been printed to stdout in between status
    /// messages without using `InteractiveStatus`, like `println!` or tracing
    /// logs.
    fn something_was_printed_elsewhere(&mut self) -> bool {
        // This will return a false positive when the terminal is resized
        // but that shouldn't be a problem.
        match (self.last_pos_in_terminal, cursor::position()) {
            (Some((last_x, last_y)), Ok((current_x, current_y))) => {
                last_x != current_x || last_y != current_y
            }
            _ => false,
        }
    }
}

static INTERACTIVE_STATUS: Mutex<InteractiveStatus> =
    Mutex::new(InteractiveStatus::new());

/// Clears leftover lines in the interactive status buffer.
/// This function should be called after some operation that outputs
/// status messages is completed.
/// When `clear_stdout` is true, the status messages are also cleared from the
/// terminal.
pub(crate) fn clear_rem_interactive_status(clear_stdout: bool) {
    let mut stdout = stdout();
    if let Ok(mut status_handler) = INTERACTIVE_STATUS.lock() {
        if clear_stdout {
            status_handler.clear_lines_in_stdout(&mut stdout);
        }
        status_handler.clear_buffer();
    } else {
        eprintln!(
            "Failed to acquire interactive status lock to clear the status"
        );
    }
}

/// Prints an interactive status message
pub(crate) fn interactive(status: &str) {
    if let Ok(mut status_handler) = INTERACTIVE_STATUS.lock() {
        status_handler.print(status);
    } else {
        eprintln!(
            "Failed to acquire interactive status lock to print the status"
        );
    }
}

/// Logs the status message at info level
pub(crate) fn headless(status: &str) {
    info!(status);
}
