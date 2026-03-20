// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};

/// Poll for a crossterm event with a timeout.
/// Returns `None` if no event is available within the timeout.
/// Filters to only KeyPress events (crossterm 0.28 emits Press/Release/Repeat).
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<KeyEvent>> {
    if event::poll(timeout)?
        && let CrosstermEvent::Key(key) = event::read()?
    {
        // Only handle key press events, not release/repeat
        if key.kind == KeyEventKind::Press {
            return Ok(Some(key));
        }
    }
    Ok(None)
}
