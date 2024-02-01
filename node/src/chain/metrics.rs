// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::Serializable;
use std::collections::VecDeque;
use std::io;
use std::io::{Read, Write};
use std::time::Duration;

const AVG_VALUES_NUM: usize = 5;

/// AverageElapsedTime calculates the average value of last N values added
#[derive(Debug)]
pub struct AverageElapsedTime(VecDeque<Duration>);
impl AverageElapsedTime {
    pub fn push_back(&mut self, value: Duration) {
        if self.0.len() == self.0.capacity() {
            self.0.pop_front();
        }
        self.0.push_back(value);
    }

    pub fn average(&self) -> Option<Duration> {
        if self.0.is_empty() {
            return None;
        }

        let sum: Duration = self.0.iter().sum();
        Some(sum / self.0.len() as u32)
    }
}

impl Default for AverageElapsedTime {
    fn default() -> Self {
        Self(VecDeque::with_capacity(AVG_VALUES_NUM))
    }
}

impl Serializable for AverageElapsedTime {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let mut bytes = Vec::new();
        self.0
            .iter()
            .for_each(|v| bytes.extend((v.as_millis() as u32).to_le_bytes()));
        w.write_all(&bytes)?;
        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut vec = VecDeque::with_capacity(AVG_VALUES_NUM);
        while let Ok(secs) = Self::read_u32_le(r) {
            vec.push_back(Duration::from_millis(secs as u64));
        }

        Ok(Self(vec))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average() {
        let expected = Duration::from_secs(108 as u64);
        let mut metric = AverageElapsedTime::default();
        for value in 100..111 {
            metric.push_back(Duration::from_secs(value as u64));
        }
        assert_eq!(metric.average().expect("positive number"), expected);

        // Marshal/Unmarshal
        let mut buf = Vec::new();
        metric.write(&mut buf).expect("all written");

        assert_eq!(
            AverageElapsedTime::read(&mut &buf[..])
                .expect("all read")
                .average()
                .unwrap(),
            expected
        );
    }
}
