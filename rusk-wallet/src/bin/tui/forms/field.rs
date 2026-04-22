// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_wallet::currency::Dusk;

/// A single input field in a form.
#[derive(Debug, Clone)]
pub struct FormField {
    pub name: &'static str,
    pub label: String,
    pub value: String,
    pub cursor: usize,
    pub kind: FieldKind,
    pub placeholder: String,
    pub selected_option: Option<usize>,
}

/// The type of input a field expects.
#[derive(Debug, Clone)]
pub enum FieldKind {
    Text,
    Amount { max: Option<Dusk> },
    Number,
    Select { options: Vec<String> },
}

impl FormField {
    pub fn text(name: &'static str, label: &str) -> Self {
        Self {
            name,
            label: label.to_string(),
            value: String::new(),
            cursor: 0,
            kind: FieldKind::Text,
            placeholder: String::new(),
            selected_option: None,
        }
    }

    pub fn text_with_default(
        name: &'static str,
        label: &str,
        default: &str,
    ) -> Self {
        Self {
            name,
            label: label.to_string(),
            value: default.to_string(),
            cursor: default.len(),
            kind: FieldKind::Text,
            placeholder: String::new(),
            selected_option: None,
        }
    }

    pub fn amount(name: &'static str, label: &str, max: Dusk) -> Self {
        Self::amount_with_max(name, label, Some(max))
    }

    pub fn amount_unknown(name: &'static str, label: &str) -> Self {
        Self::amount_with_max(name, label, None)
    }

    fn amount_with_max(
        name: &'static str,
        label: &str,
        max: Option<Dusk>,
    ) -> Self {
        Self {
            name,
            label: label.to_string(),
            value: String::new(),
            cursor: 0,
            kind: FieldKind::Amount { max },
            placeholder: "0.000000000".to_string(),
            selected_option: None,
        }
    }

    pub fn number(name: &'static str, label: &str, default: u64) -> Self {
        Self {
            name,
            label: label.to_string(),
            value: default.to_string(),
            cursor: default.to_string().len(),
            kind: FieldKind::Number,
            placeholder: String::new(),
            selected_option: None,
        }
    }

    pub fn select_with_default(
        name: &'static str,
        label: &str,
        options: Vec<String>,
        default_idx: usize,
    ) -> Self {
        let idx = default_idx.min(options.len().saturating_sub(1));
        let value = options.get(idx).cloned().unwrap_or_default();
        Self {
            name,
            label: label.to_string(),
            value,
            cursor: 0,
            kind: FieldKind::Select { options },
            placeholder: String::new(),
            selected_option: Some(idx),
        }
    }

    /// Returns true if this is a select field.
    pub fn is_select(&self) -> bool {
        matches!(self.kind, FieldKind::Select { .. })
    }

    /// Cycle to the next option in a select field.
    pub fn cycle_next(&mut self) {
        if let FieldKind::Select { options } = &self.kind {
            let current = self.selected_option.unwrap_or(0);
            let next = (current + 1) % options.len();
            self.selected_option = Some(next);
            self.value = options[next].clone();
        }
    }

    /// Cycle to the previous option in a select field.
    pub fn cycle_prev(&mut self) {
        if let FieldKind::Select { options } = &self.kind {
            let current = self.selected_option.unwrap_or(0);
            let prev = if current == 0 {
                options.len() - 1
            } else {
                current - 1
            };
            self.selected_option = Some(prev);
            self.value = options[prev].clone();
        }
    }

    /// Set value to the max amount (for Amount fields).
    pub fn set_max(&mut self) {
        if let FieldKind::Amount { max: Some(max) } = &self.kind {
            self.value = format!("{max}");
            self.cursor = self.value.len();
        }
    }

    /// Move cursor to start of value.
    pub fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end of value.
    pub fn move_cursor_end(&mut self) {
        self.cursor = self.value.len();
    }

    pub fn input_char(&mut self, c: char) {
        match &self.kind {
            FieldKind::Select { .. } => {
                // Select fields use Left/Right arrows, not char input
            }
            FieldKind::Number => {
                if c.is_ascii_digit() {
                    self.value.insert(self.cursor, c);
                    self.cursor += 1;
                }
            }
            FieldKind::Amount { .. } => {
                if c.is_ascii_digit() || c == '.' {
                    // Only allow one decimal point
                    if c == '.' && self.value.contains('.') {
                        return;
                    }
                    self.value.insert(self.cursor, c);
                    self.cursor += 1;
                }
            }
            _ => {
                self.value.insert(self.cursor, c);
                self.cursor += 1;
            }
        }
    }

    pub fn delete_char(&mut self) {
        if matches!(self.kind, FieldKind::Select { .. }) {
            return;
        }
        if self.cursor > 0 {
            self.value.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    /// Returns a display value for rendering. Masks password fields.
    pub fn display_value(&self) -> String {
        if self.value.is_empty() && !self.placeholder.is_empty() {
            return String::new();
        }
        self.value.clone()
    }
}
