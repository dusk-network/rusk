// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp::Ordering;
use std::collections::VecDeque;

/// Implements logic of calculating the average of last N stored values
pub struct AvgValidationTime(VecDeque<u16>, u64);
impl AvgValidationTime {
    pub fn update(&mut self, round: u64, value: u16) {
        match round.cmp(&self.1) {
            Ordering::Equal => {
                if let Some(v) = self.0.back_mut() {
                    *v = value
                }
            }
            Ordering::Greater => {
                if self.0.len() == self.0.capacity() {
                    self.0.pop_front();
                }
                self.0.push_back(value);
                self.1 = round;
            }
            Ordering::Less => {}
        }
    }

    pub fn average(&self) -> Option<u16> {
        let sum: u16 = self.0.iter().sum();
        if sum == 0 {
            return None;
        }
        Some(sum / self.0.len() as u16)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.0.iter().for_each(|v| bytes.extend(v.to_le_bytes()));
        bytes
    }

    pub fn from_bytes(buf: &Vec<u8>, cap: usize) -> Self {
        let mut res = Self(VecDeque::with_capacity(cap), 0);
        let value_size = std::mem::size_of::<u16>();

        if buf.len() != cap * value_size {
            return res;
        }

        res.0.extend(
            (0..buf.len())
                .step_by(value_size)
                .map(|i| u16::from_le_bytes([buf[i], buf[i + 1]])),
        );

        res
    }
}
