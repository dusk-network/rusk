// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::Serializable;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::io;
use std::io::{Read, Write};

const AVG_VALUES_NUM: usize = 5;

/// Implements logic of calculating the average of last N stored values
#[derive(Debug)]
pub struct AvgValidationTime(VecDeque<u16>, u64);
impl AvgValidationTime {
    pub fn update(&mut self, round: u64, value: u16) -> Option<u16> {
        match round.cmp(&self.1) {
            Ordering::Equal => {
                if let Some(v) = self.0.back_mut() {
                    *v = value;
                    return Some(*v);
                }
            }
            Ordering::Greater => {
                if self.0.len() == self.0.capacity() {
                    self.0.pop_front();
                }
                self.0.push_back(value);
                self.1 = round;

                return Some(value);
            }
            Ordering::Less => {}
        }

        None
    }

    pub fn average(&self) -> Option<u16> {
        let sum: u16 = self.0.iter().sum();
        if sum == 0 {
            return None;
        }
        Some(sum / self.0.len() as u16)
    }
}

impl Default for AvgValidationTime {
    fn default() -> Self {
        Self(VecDeque::with_capacity(AVG_VALUES_NUM), 0)
    }
}

impl Serializable for AvgValidationTime {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.1.to_le_bytes())?;

        let mut bytes = Vec::new();
        self.0.iter().for_each(|v| bytes.extend(v.to_le_bytes()));
        w.write_all(&bytes)?;
        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let round = Self::read_u64_le(r)?;

        let mut buf = Vec::new();
        _ = r.read_to_end(&mut buf)?;
        let mut vec = VecDeque::with_capacity(AVG_VALUES_NUM);
        vec.extend(
            (0..buf.len())
                .step_by(2)
                .map(|i| u16::from_le_bytes([buf[i], buf[i + 1]])),
        );

        Ok(Self(vec, round))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average() {
        let test_cases = [
            (1, 101, true),
            (2, 102, true),
            (3, 103, true),
            (4, 104, true),
            (5, 100, true),
            (5, 105, true),
            (4, 105, false),
            (6, 106, true),
            (7, 107, true),
        ];
        let avg = 525 / AVG_VALUES_NUM as u16;

        let mut metric = AvgValidationTime::default();
        for (round, value, exp) in test_cases.iter() {
            assert_eq!(metric.update(*round, *value).is_some(), *exp);
        }
        assert_eq!(metric.average().expect("positive number"), avg);

        // Marshal/Unmarshal
        let mut buf = Vec::new();
        metric.write(&mut buf).expect("all written");
        assert_eq!(
            AvgValidationTime::read(&mut &buf[..])
                .expect("all read")
                .average()
                .unwrap(),
            avg
        );
    }
}
