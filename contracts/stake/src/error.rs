// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    DuskBytes,
    PlonkKeys,
    PlonkProver,
}

impl From<dusk_bytes::Error> for Error {
    fn from(_: dusk_bytes::Error) -> Self {
        Self::DuskBytes
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
