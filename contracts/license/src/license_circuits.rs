// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_LICENSE_CIRCUIT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/b6ec670023d4be069ef4f63cf532cc8bb1dd02aa96e12566337a562a6a564e38.vd"));

/// Verifier data for the `License` circuit.
#[allow(dead_code)]
pub const fn verifier_data_license_circuit() -> &'static [u8] {
    VD_LICENSE_CIRCUIT
}
