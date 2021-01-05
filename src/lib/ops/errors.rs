// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use wasmi::HostError;

#[derive(Debug)]
pub enum RuskExternalError {
    WrongArgsNumber,
    InvokeIdxNotFound(usize),
    ResolverNameNotFound(String),
}

impl fmt::Display for RuskExternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl HostError for RuskExternalError {}
