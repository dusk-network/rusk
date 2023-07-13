// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_LICENSE_CIRCUIT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/f0aaeb94d6e80c1a02c9bb339241c730291a287eb6f73272c5474da71c2f6589.vd"));

/// Verifier data for the `License` circuit.
#[allow(dead_code)]
pub const fn verifier_data_license_circuit() -> &'static [u8] {
    VD_LICENSE_CIRCUIT
}
