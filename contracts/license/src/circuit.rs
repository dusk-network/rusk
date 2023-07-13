// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_LICENSE: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/f0aaeb94d6e80c1a02c9bb339241c730291a287eb6f73272c5474da71c2f6589.vd"
));

/// Verifier data for the `License` circuit.
pub const fn verifier_data_license() -> &'static [u8] {
    VD_LICENSE
}
