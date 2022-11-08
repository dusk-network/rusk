// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::fmt;
use phoenix_core::Error as PhoenixError;

#[derive(Debug, Clone)]
pub enum Error {
    Phoenix(PhoenixError),
    NoteNotFound,
    MessageNotFound,
    CrossoverNotFound,
    ExecuteRecursion,
    NotEnoughBalance,
    ProofVerificationError,
    PaymentTypeNotAccepted,
    ContractNotFound,
}

impl From<PhoenixError> for Error {
    fn from(e: PhoenixError) -> Self {
        Self::Phoenix(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
