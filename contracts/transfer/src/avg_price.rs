// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const AVG_GAS_PRICE_K: u64 = 200; // moving average of K data points
const AVG_GAS_PRICE_FACTOR: u64 = 1_048_576; // factor for division precision

pub struct AvgGasPrice {
    avg_price: u64, // actual price times AVG_PRICE_FACTOR
    first: u64,     // moving first price times AVG_PRICE_FACTOR
    count: u64,     // counts to AVG_PRICE_K elements
}

impl AvgGasPrice {
    pub const fn new() -> Self {
        Self {
            avg_price: AVG_GAS_PRICE_FACTOR,
            first: AVG_GAS_PRICE_FACTOR,
            count: 0,
        }
    }

    pub fn update(&mut self, gas_price: u64) {
        let gas_price = gas_price * AVG_GAS_PRICE_FACTOR;
        if self.count > AVG_GAS_PRICE_K {
            self.avg_price += (gas_price - self.first) / AVG_GAS_PRICE_K;
        } else {
            self.avg_price =
                (self.avg_price * self.count + gas_price) / (self.count + 1)
        }
        self.first = gas_price;
        self.count += 1;
        if self.count > AVG_GAS_PRICE_K * 512 {
            // to avoid count overflow
            self.count = AVG_GAS_PRICE_K * 2;
        }
    }

    pub fn get(&self) -> u64 {
        self.avg_price / AVG_GAS_PRICE_FACTOR
    }
}
