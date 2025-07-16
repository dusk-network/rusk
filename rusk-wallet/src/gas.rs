// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This module contains the primitive related to the gas used for transaction
//! in the Dusk Network.

use serde::Deserialize;

use crate::currency::Lux;

/// The minimum gas limit
pub const MIN_LIMIT: u64 = 100_000;

/// The default gas limit for transfer transactions
pub const DEFAULT_LIMIT_TRANSFER: u64 = 2_500_000;

/// The default gas limit for a contract deployment
pub const DEFAULT_LIMIT_DEPLOYMENT: u64 =
    1024 * 1024 * 100 + DEFAULT_LIMIT_TRANSFER; //1MB at 100gas per byte + transfer limit

/// The default gas limit for contract calls
pub const DEFAULT_LIMIT_CALL: u64 = 2_000_000_000;

/// The default gas limit for stake/unstake/claim-rewards actions
pub const DEFAULT_LIMIT_STAKE: u64 = 50_000_000;

/// The gas cost per deployed byte
pub const GAS_PER_DEPLOY_BYTE: u64 = 100;

/// The default gas price
pub const DEFAULT_PRICE: Lux = 1;

/// The minimum gas price for a contract deployment
pub const MIN_PRICE_DEPLOYMENT: Lux = 2_000;

#[derive(Debug)]
/// Gas price and limit for any transaction
pub struct Gas {
    /// The gas price in [Lux]
    pub price: Lux,
    /// The gas limit
    pub limit: u64,
}

impl Gas {
    /// Default gas price and limit
    #[must_use]
    pub fn new(limit: u64) -> Self {
        Gas {
            price: DEFAULT_PRICE,
            limit,
        }
    }

    /// Returns `true` if the gas is equal or greater than the minimum limit
    #[must_use]
    pub fn is_enough(&self) -> bool {
        self.limit >= MIN_LIMIT
    }

    /// Set the price
    pub fn set_price<T>(&mut self, price: T)
    where
        T: Into<Option<Lux>>,
    {
        self.price = price.into().unwrap_or(DEFAULT_PRICE);
    }

    /// Set the price and return the Gas
    #[must_use]
    pub fn with_price<T>(mut self, price: T) -> Self
    where
        T: Into<Lux>,
    {
        self.price = price.into();
        self
    }

    /// Set the limit
    pub fn set_limit<T>(&mut self, limit: T)
    where
        T: Into<Option<u64>>,
    {
        if let Some(limit) = limit.into() {
            self.limit = limit;
        }
    }
}

impl Default for Gas {
    fn default() -> Self {
        Self::new(DEFAULT_LIMIT_TRANSFER)
    }
}

/// Dynamic gas prices information from the mempool
#[derive(Debug, Deserialize)]
pub struct MempoolGasPrices {
    /// Average gas price in the mempool in [Lux]
    pub average: Lux,
    /// Maximum gas price in the mempool in [Lux]
    pub max: Lux,
    /// Median gas price in the mempool in [Lux]
    pub median: Lux,
    /// Minimum gas price in the mempool in [Lux]
    pub min: Lux,
}
