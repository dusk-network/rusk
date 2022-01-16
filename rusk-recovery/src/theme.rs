// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use console::Style;

pub struct Theme {
    success: Style,
    error: Style,
    warn: Style,
    info: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

impl Theme {
    pub fn new() -> Self {
        let general = Style::new().bright().bold();

        Self {
            success: general.clone().green(),
            error: general.clone().red(),
            warn: general.clone().yellow(),
            info: general.cyan(),
        }
    }
    fn fmt(&self, style: &Style, s: impl AsRef<str>) -> String {
        format!("{:>12}", style.apply_to(s.as_ref()))
    }

    pub fn success(&self, s: impl AsRef<str>) -> String {
        self.fmt(&self.success, s)
    }

    pub fn action(&self, s: impl AsRef<str>) -> String {
        self.fmt(&self.success, s)
    }

    pub fn error(&self, s: impl AsRef<str>) -> String {
        self.fmt(&self.error, s)
    }

    pub fn warn(&self, s: impl AsRef<str>) -> String {
        self.fmt(&self.warn, s)
    }

    pub fn info(&self, s: impl AsRef<str>) -> String {
        self.fmt(&self.info, s)
    }
}
