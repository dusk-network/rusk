// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use wasmi::HostError;

#[derive(Debug)]
pub enum RuskExtenalError {
    WrongArgsNumber,
    InvokeIdxNotFound(usize),
    ResolverNameNotFound(String),
    InvalidFFIEncoding,
}

impl fmt::Display for RuskExtenalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl HostError for RuskExtenalError {}
