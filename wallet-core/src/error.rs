// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Error-type for wallet-core.

use core::fmt;

/// The execution-core error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Recovery phrase is not valid
    InvalidMnemonicPhrase,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Wallet-Core Error: {:?}", &self)
    }
}
