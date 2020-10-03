// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use tracing::info;

pub(crate) mod circuit_helpers;
pub mod encoding;
pub mod services;

#[derive(Debug)]
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

pub mod proto_types {
    pub use super::services::rusk_proto::{
        BlsScalar, JubJubCompressed, JubJubScalar, Proof,
    };
}

use dusk_plonk::prelude::PublicParameters;
use lazy_static::lazy_static;
lazy_static! {
    static ref PUB_PARAMS: PublicParameters = {
        circuit_helpers::read_pub_params()
            .expect("Error reading Public Params.")
    };
}
