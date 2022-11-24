// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use lazy_static::lazy_static;

const TRANSFER: &[u8] = include_bytes!(
    "../../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);
const STAKE: &[u8] = include_bytes!(
    "../../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
);
const ALICE: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/alice.wasm");
const BOB: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/bob.wasm");

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

mod wrapper;
