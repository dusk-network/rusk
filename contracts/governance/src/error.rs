// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::CanonError;
use core::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    AddressIsNotAllowed,
    BalanceOverflow,
    InvalidPublicKey,
    Canon(CanonError),
    ContractIsPaused,
    InsufficientBalance,
    InvalidSignature,
    SeedAlreadyUsed,
}

impl From<CanonError> for Error {
    fn from(e: CanonError) -> Self {
        Self::Canon(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
