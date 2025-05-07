// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::fmt;
use dusk_core::Error as ExecutionError;

#[derive(Debug, Clone)]
pub enum Error {
    #[allow(dead_code)]
    /// Wrapper of dusk-core error type.
    Execution(ExecutionError),
    /// A contract balance is not sufficient for the requested withdrawal
    NotEnoughBalance,
}

impl From<ExecutionError> for Error {
    fn from(e: ExecutionError) -> Self {
        Self::Execution(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
