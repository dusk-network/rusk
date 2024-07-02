// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

const AVG_GAS_PRICE_K: usize = 200; // moving average of K data points
const RING_SIZE: usize = AVG_GAS_PRICE_K + 1;
const AVG_GAS_PRICE_FACTOR: i64 = 1_048_576; // factor for division precision

pub struct AvgGasPrice {
    avg_price: i64, // actual price times AVG_PRICE_FACTOR
    window: ConstGenericRingBuffer<i64, RING_SIZE>,
    count: usize,
}

impl AvgGasPrice {
    pub const fn new(default: u64) -> Self {
        Self {
            avg_price: default as i64 * AVG_GAS_PRICE_FACTOR,
            window: ConstGenericRingBuffer::new(),
            count: AVG_GAS_PRICE_K,
        }
    }

    pub fn update(&mut self, gas_price: u64) {
        let gas_price = gas_price as i64 * AVG_GAS_PRICE_FACTOR;
        self.window.push(gas_price);
        if self.count == 0 {
            let first = *self.window.get(0).unwrap();
            self.avg_price += (gas_price - first) / AVG_GAS_PRICE_K as i64;
        } else {
            self.avg_price = (self.avg_price
                * (AVG_GAS_PRICE_K - self.count) as i64
                + gas_price)
                / ((AVG_GAS_PRICE_K - self.count) + 1) as i64;
            self.count -= 1;
        }
    }

    pub fn get(&self) -> u64 {
        (self.avg_price / AVG_GAS_PRICE_FACTOR) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avg_no_items() {
        let avg = AvgGasPrice::new(1);
        assert_eq!(avg.get(), 1);
    }

    #[test]
    fn avg_window_not_full() {
        let mut avg = AvgGasPrice::new(1);
        avg.update(2000000);
        avg.update(3000000);
        avg.update(4000000);
        avg.update(5000000);
        assert_eq!(avg.get(), 3500000);
    }

    #[test]
    fn avg_window_full() {
        let mut avg = AvgGasPrice::new(1);
        for _ in 0..AVG_GAS_PRICE_K {
            avg.update(2000000);
        }
        for _ in 0..(AVG_GAS_PRICE_K / 2) {
            avg.update(4000000);
        }
        assert_eq!(avg.get(), 3000000);
    }
}
