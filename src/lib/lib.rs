// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use lazy_static::lazy_static;
use tracing::info;

/*
mod ops;
mod encoding;
mod transaction;

pub mod services;
pub use ops::{RuskExtenalError, RuskExternals};
*/

lazy_static! {
    pub static ref PUB_PARAMS: PublicParameters = {
        let buff =
            rusk_profile::get_common_reference_string().expect("CRS not found");

        unsafe {
            PublicParameters::from_slice_unchecked(&buff)
                .expect("CRS not decoded")
        }
    };
}

#[derive(Debug, Copy, Clone)]
pub struct Rusk {}

impl Default for Rusk {
    fn default() -> Rusk {
        // Initialize the PUB_PARAMS since they're lazily
        // evaluated. On that way we prevent the first usage
        // of the PUB_PARAMS to take a lot of time.
        info!("Loading CRS...");
        lazy_static::initialize(&PUB_PARAMS);
        info!("CRS was successfully loaded...");
        Rusk {}
    }
}
